use core::{
    alloc::{AllocError, Allocator, Layout},
    cell::Cell,
    fmt,
    num::NonZeroUsize,
    ops::Range,
    ptr::NonNull,
};

use crate::{
    BumpClaimGuard, BumpScopeGuard, ErrorMode, allocator_impl,
    base_allocator_storage::BaseAllocatorStorage,
    bump_align_guard::BumpAlignGuard,
    checkpoint::Checkpoint,
    chunk_size::{ChunkSizeCapacityFirst, ChunkSizeDefault},
    chunk_types::{AllocatedChunk, Chunk, ChunkKind},
    config::{self, BumpConfig, config},
    layout::{ArrayLayout, CustomLayout, LayoutProps, SizedLayout},
    polyfill::hint::unlikely,
    stats::Stats,
    util::{
        SizedTypeProperties, align_pos, bump_down, const_assert_upwards_matches, transmute_mut, transmute_ref,
        transmute_value, up_align_usize_unchecked,
    },
};

/// A bump allocation scope.
///
/// [`Bump`] and [`BumpScopeGuard`] dereference to this type.
///
/// [`Bump`]: crate::Bump
pub struct BumpScope<const C: BumpConfig, A> {
    /// Either a chunk allocated from `A`, or a [`CLAIMED`] or [`CONST_CONSTRUCTED`] dummy chunk.
    ///
    /// [`CLAIMED`]: crate::chunk_types::DummyChunk::CLAIMED
    /// [`CONST_CONSTRUCTED`]: crate::chunk_types::DummyChunk::CONST_CONSTRUCTED
    pub(crate) chunk: Cell<Chunk<C>>,
    pub(crate) allocator: BaseAllocatorStorage<A>,
}

impl<const C: BumpConfig, A: Allocator> fmt::Debug for BumpScope<C, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.stats().debug_format("BumpScope", f)
    }
}

/// # Private convenience methods.
impl<const C: BumpConfig, A> BumpScope<C, A> {
    #[inline(always)]
    pub(crate) fn chunk(&self) -> Chunk<C> {
        self.chunk.get()
    }

    #[inline(always)]
    pub(crate) fn set_chunk(&self, chunk: Chunk<C>) {
        self.chunk.set(chunk);
    }
}

/// # Creating child scopes.
impl<const C: BumpConfig, A: Allocator> BumpScope<C, A> {
    /// Creates a new [`BumpScopeGuard`].
    ///
    /// This allows for creation of child scopes.
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// {
    ///     let mut guard = bump.begin_scope();
    ///     let bump = guard.scope();
    ///     bump.alloc_str("Hello, world!").into_ref();
    ///     assert_eq!(bump.allocated(), 13);
    /// }
    ///
    /// assert_eq!(bump.allocated(), 0);
    /// ```
    ///
    /// [claimed]: BumpScope::claim
    #[inline(always)]
    pub fn begin_scope(&mut self) -> BumpScopeGuard<'_, C, A> {
        BumpScopeGuard::new(self)
    }

    /// Creates a checkpoint of the current bump position.
    ///
    /// The bump position can be reset to this checkpoint with [`reset_to`].
    ///
    /// [`reset_to`]: BumpScope::reset_to
    #[must_use]
    #[inline(always)]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk())
    }

    /// Resets the bump position to a previously created checkpoint.
    ///
    /// The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    ///
    /// - This method invalidates all memory blocks allocated since creation of this checkpoint.
    /// - The provided checkpoint must have been created by this bump allocator.
    /// - The bump allocator must not have been [`reset`] since creation of this checkpoint.
    /// - The checkpoint must not have been created by a bump allocator with no chunks when self is **not** [`const_constructible`].
    /// - The checkpoint must not have been created by a [claimed] bump allocator when self is **not** [`claimable`].
    ///
    /// [`reset`]: crate::Bump::reset
    /// [claimed]: crate::BumpScope::claim
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    /// # Panics
    /// With `debug_assertions` on, this panics when the checkpoint does not belong to this bump allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.allocated(), 5);
    ///     # _ = hello;
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.allocated(), 0);
    /// ```
    #[cfg_attr(not(debug_assertions), inline(always))]
    pub unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        if (config!(claimable) || config!(const_constructible)) && checkpoint.chunk.is_dummy() {
            return unsafe { self.reset_to_with_dummy_checkpoint(checkpoint) };
        }

        // Check if the checkpoint belongs to this bump allocator.
        #[cfg(debug_assertions)]
        if !self.is_claimed() {
            let chunk = self
                .stats()
                .small_to_big()
                .find(|chunk| chunk.as_ptr().addr() == checkpoint.chunk.as_ptr().addr())
                .expect("this checkpoint does not refer to any chunk in this bump allocator");

            let start = chunk.content_start().addr().get();
            let end = chunk.content_end().addr().get();
            let addr = checkpoint.address.get();

            assert!(
                addr >= start && addr <= end,
                "checkpoint address is not in the chunk's content range"
            );
        }

        unsafe {
            // Reset the chunk position of the checkpoint chunk.
            let ptr = checkpoint.chunk.as_ptr().with_addr(checkpoint.address).cast();
            AllocatedChunk::<C>::from_ptr(checkpoint.chunk.as_ptr()).set_pos(ptr);
            self.set_chunk(Chunk::from_ptr(checkpoint.chunk.as_ptr()));
        }
    }

    /// # Safety
    /// The caller must satisfy the the invariants of `reset_to` and
    /// `checkpoint.chunk.is_dummy()` must be `true`.
    #[cold]
    #[inline(never)]
    unsafe fn reset_to_with_dummy_checkpoint(&self, checkpoint: Checkpoint) {
        debug_assert!(checkpoint.chunk.is_dummy());

        if unlikely(config!(claimable) && checkpoint.chunk.is_claimed()) {
            // We do nothing if the checkpoint is claimed.
            // This state is reachable in safe code when a `BumpClaimGuard` is forgotten.
            return;
        }

        // Now the chunk must be const constructed, because it is the only dummy chunk
        // variant other than "claimed".
        debug_assert!(checkpoint.chunk.is_const_constructed());

        // If the checkpoint was created when the bump allocator had no allocated chunk
        // then the chunk pointer will point to the const constructed chunk.
        //
        // In such cases we reset the bump pointer to the very start of the very first chunk.
        //
        // SAFETY: nothing is allocated
        unsafe { self.reset_to_start() };
    }

    /// # Safety
    /// Nothing must be allocated.
    #[inline(always)]
    pub(crate) unsafe fn reset_to_start(&self) {
        let Some(mut chunk) = self.chunk().as_allocated() else {
            return;
        };

        while let Some(prev) = chunk.prev() {
            chunk = prev;
        }

        chunk.reset();

        self.set_chunk(Chunk::from_allocated(chunk));
    }

    /// Converts this `&mut BumpScope` into an owned `BumpScope`.
    ///
    /// Allocates a chunk if none has been allocated yet.
    ///
    /// This can be used to remove an indirection and allow
    /// for more config conversions via [`change_config`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// Errors if the bump allocator is currently [claimed].
    ///
    /// [claimed]: crate::BumpScope::claim
    /// [`change_config`]: BumpScope::change_config
    /// [`mut_change_config`]: BumpScope::mut_change_config
    #[inline(always)]
    pub fn by_value<E: ErrorMode>(&mut self) -> Result<BumpScope<C, A>, E> {
        self.make_allocated::<E>()?;

        Ok(Self {
            chunk: self.chunk.clone(),
            allocator: self.allocator.create_ref(),
        })
    }

    #[inline]
    fn make_allocated<E: ErrorMode>(&self) -> Result<(), E> {
        match self.chunk().kind() {
            ChunkKind::Claimed => Err(E::claimed()),
            ChunkKind::ConstConstructed => {
                let allocator = self.allocator();
                let chunk = Chunk::from_allocated(AllocatedChunk::new_first::<E, A>(ChunkSizeDefault, allocator)?);
                self.set_chunk(chunk);
                Ok(())
            }
            ChunkKind::Allocated(_) => Ok(()),
        }
    }
}

/// # Claim.
impl<const C: BumpConfig, A: Allocator> BumpScope<C, A> {
    /// Claims exclusive access to the bump allocator from a shared reference.
    ///
    /// This makes it possible to enter scopes while a there are still outstanding
    /// references to that bump allocator.
    ///
    /// The `claim` call replaces this allocator with a dummy allocator while the returned `BumpClaimGuard`
    /// is live. This dummy allocator errors on `allocate` / `grow`, does nothing on `deallocate` / `shrink` and
    /// reports an empty bump allocator from the `stats` api.
    ///
    /// # Panics
    /// Panics if the bump allocator is already claimed.
    ///
    /// # Compile-Time Assertions
    /// Fails to compile when not [`claimable`].
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, extra};
    ///
    /// let bump: Bump = Bump::new();
    /// let vec1: Vec<u8, _> = extra::vec![in &bump; 1, 2, 3];
    /// let vec2: Vec<u8, _> = extra::vec![in &bump; 4, 5, 6];
    ///
    /// bump.claim().scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    ///
    /// assert_eq!(vec1, [1, 2, 3]);
    /// assert_eq!(vec2, [4, 5, 6]);
    /// ```
    ///
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    pub fn claim(&self) -> BumpClaimGuard<'_, C, A> {
        BumpClaimGuard::new(self)
    }

    /// Returns true if the bump allocator is currently [claimed].
    ///
    /// [claimed]: crate::BumpScope::claim
    #[must_use]
    #[inline(always)]
    pub fn is_claimed(&self) -> bool {
        self.chunk().is_claimed()
    }
}

/// # Miscellaneous.
impl<const C: BumpConfig, A: Allocator> BumpScope<C, A> {
    /// Returns the amount of allocated bytes.
    /// This includes padding and wasted space due to reallocations.
    ///
    /// Equivalent to `.[stats]\().[allocated]\()`.
    ///
    /// [stats]: BumpScope::stats
    /// [allocated]: crate::stats::Stats::allocated
    #[inline]
    pub fn allocated(&self) -> usize {
        self.stats().allocated()
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'_, C> {
        Stats::from_raw_chunk(self.chunk())
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        &self.allocator
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve`, <code>self.[stats][]().[remaining][]()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that these bytes are not necessarily in one contiguous region but
    /// might be spread across multiple chunks.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// Errors if the bump allocator is currently [claimed].
    ///
    /// # Examples
    /// ```
    /// use bump_scope_core::Bump;
    ///
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    ///
    /// [stats]: crate::BumpScope::stats
    /// [remaining]: crate::stats::Stats::remaining
    /// [claimed]: crate::BumpScope::claim
    pub fn reserve<E: ErrorMode>(&self, additional: usize) -> Result<(), E> {
        let chunk = self.chunk();

        match chunk.kind() {
            ChunkKind::Claimed => Err(E::claimed()),
            ChunkKind::ConstConstructed => {
                let Ok(layout) = Layout::from_size_align(additional, 1) else {
                    return Err(E::capacity_overflow());
                };

                let allocator = self.allocator();
                let size = ChunkSizeCapacityFirst(layout);
                let chunk = AllocatedChunk::new_first::<E, A>(size, allocator)?;
                self.set_chunk(Chunk::from_allocated(chunk));
                Ok(())
            }
            ChunkKind::Allocated(mut chunk) => {
                let mut additional = additional;

                if let Some(rest) = additional.checked_sub(chunk.remaining()) {
                    additional = rest;
                } else {
                    return Ok(());
                }

                while let Some(next) = chunk.next() {
                    chunk = next;

                    if let Some(rest) = additional.checked_sub(chunk.capacity()) {
                        additional = rest;
                    } else {
                        return Ok(());
                    }
                }

                if additional == 0 {
                    return Ok(());
                }

                let Ok(layout) = Layout::from_size_align(additional, 1) else {
                    return Err(E::capacity_overflow());
                };

                AllocatedChunk::new_non_first(layout, chunk, self.allocator()).map(drop)
            }
        }
    }
}

/// # Specialized allocation methods.
impl<const C: BumpConfig, A: Allocator> BumpScope<C, A> {
    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///
    /// Behaves like:
    /// ```
    /// # use core::{alloc::Layout, alloc::{Allocator, AllocError}, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
    /// Ok(self.allocate(layout)?.cast::<u8>())
    /// #     }
    /// # }
    /// ```
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn alloc<E: ErrorMode>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        match self.chunk().alloc(CustomLayout(layout)) {
            Some(ptr) => Ok(ptr),
            None => self.alloc_in_another_chunk(layout),
        }
    }

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///
    /// Behaves like:
    /// ```
    /// # use core::{alloc::Layout, alloc::{Allocator, AllocError}, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn<T>(&self) -> Result<NonNull<T>, AllocError> {
    /// Ok(self.allocate(Layout::new::<T>())?.cast::<T>())
    /// #     }
    /// # }
    /// ```
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn alloc_sized<E: ErrorMode, T>(&self) -> Result<NonNull<T>, E> {
        let layout = const {
            match SizedLayout::new::<T>().increase_align(config!(min_align)) {
                Ok(ok) => ok,
                Err(_) => panic!("failed to construct sized layout with min_align"),
            }
        };

        match self.chunk().alloc(layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.alloc_sized_in_another_chunk::<E, T>() {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///    
    /// Behaves like:
    /// ```
    /// # extern crate alloc;
    /// # use core::{alloc::Layout, alloc::Allocator, ptr::NonNull, error::Error};
    /// # use alloc::boxed::Box;
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn<T>(&self, len: usize) -> Result<NonNull<T>, Box<dyn Error>> {
    /// Ok(self.allocate(Layout::array::<T>(len)?)?.cast::<T>())
    /// #     }
    /// # }
    /// ```
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn alloc_slice<E: ErrorMode, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        let Ok(layout) = ArrayLayout::array::<T>(len) else {
            return Err(E::capacity_overflow());
        };

        match self.chunk().alloc(layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.alloc_slice_in_another_chunk::<E, T>(len) {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///
    /// Behaves like:
    /// ```
    /// # use core::{alloc::Layout, alloc::{Allocator, AllocError}, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
    /// Ok(self.allocate(Layout::for_value(slice))?.cast::<T>())
    /// #     }
    /// # }
    /// ```
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn alloc_slice_for<E: ErrorMode, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        let layout = ArrayLayout::for_value(slice);

        match self.chunk().alloc(layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.alloc_slice_in_another_chunk::<E, T>(slice.len()) {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    /// FIXME: proper documentation
    ///
    /// The allocation takes up as much of the free space of the chunk as possible.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn prepare_slice<E: ErrorMode, T>(&self, len: usize) -> Result<NonNull<[T]>, E> {
        let range = self.prepare_allocation_range::<E, T>(len)?;

        // NB: We can't use `offset_from_unsigned`, because the size is not a multiple of `T`'s.
        let cap = unsafe { range.end.byte_offset_from_unsigned(range.start) } / T::SIZE;

        let ptr = if config!(upwards) {
            range.start
        } else {
            unsafe { range.end.sub(cap) }
        };

        Ok(NonNull::slice_from_raw_parts(ptr, cap))
    }

    /// Returns a pointer range.
    /// The start and end pointers are aligned.
    /// But `end - start` is *not* a multiple of `size_of::<T>()`.
    /// So `end.offset_from_unsigned(start)` may not be used!
    #[inline(always)]
    fn prepare_allocation_range<B: ErrorMode, T>(&self, cap: usize) -> Result<Range<NonNull<T>>, B> {
        let Ok(layout) = ArrayLayout::array::<T>(cap) else {
            return Err(B::capacity_overflow());
        };

        let range = match self.chunk().prepare_allocation_range(layout) {
            Some(ptr) => ptr,
            None => self.prepare_allocation_range_in_another_chunk(layout)?,
        };

        Ok(range.start.cast::<T>()..range.end.cast::<T>())
    }

    /// Allocate part of the slice returned from a [`prepare_slice`] call.
    ///
    /// # Safety
    /// - `ptr` and `cap` must form the slice that has been returned from [`prepare_slice`]
    /// - no allocation, grow, shrink or deallocate must have taken place since then
    /// - no resets must have taken place since then
    /// - `len` must be less than or equal to `cap`
    /// - the bump allocator must be [unclaimed] at the time [`prepare_slice`] was called and when calling this function
    ///
    /// [`prepare_slice`]: BumpScope::prepare_slice
    /// [unclaimed]: BumpScope::claim
    #[inline(always)]
    pub unsafe fn commit_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe {
            // SAFETY: successful `spec_prepare_slice` guarantees an allocated chunk
            let chunk = self.chunk().as_allocated_unchecked();
            let end = ptr.add(len);

            if config!(upwards) {
                chunk.set_pos_addr_and_align_from(end.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(ptr, len)
            } else {
                let dst_end = ptr.add(cap);
                let dst = dst_end.sub(len);
                ptr.copy_to(dst, len);
                chunk.set_pos_addr_and_align_from(dst.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            }
        }
    }

    /// A specialized version of [`shrink`](core::alloc::Allocator::shrink).
    ///
    /// Behaves like the following code except that it returns `None`
    /// when the allocation remains unchanged and the pointer stays valid.
    /// ```
    /// # use core::{alloc::Layout, alloc::Allocator, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> NonNull<T> {
    /// self.shrink(ptr.cast(),
    ///     Layout::array::<T>(old_len).unwrap_unchecked(),
    ///     Layout::array::<T>(new_len).unwrap_unchecked(),
    /// ).unwrap_unchecked().cast()
    /// #     }
    /// # }
    /// ```
    ///
    /// # Safety
    /// Same safety conditions as for the code above apply.
    ///
    /// [shrink]: core::alloc::Allocator::shrink
    /// [array]: Layout::array
    #[inline(always)]
    pub unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        if !config!(shrinks) {
            return None;
        }

        let old_ptr = ptr.cast::<u8>();
        let old_size = old_len * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_len * T::SIZE; // its less than the capacity so this can't overflow

        // Adapted from `Allocator::shrink`.
        unsafe {
            let chunk = self.chunk();

            let is_last = if config!(upwards) {
                old_ptr.as_ptr().add(old_size) == chunk.pos().as_ptr()
            } else {
                old_ptr == chunk.pos()
            };

            // if that's not the last allocation, there is nothing we can do
            if !is_last {
                return None;
            }

            // `is_last` is true, which guarantees a non-dummy, see `allocator_impl::is_last`
            let chunk = chunk.as_allocated_unchecked();

            if config!(upwards) {
                let end = old_ptr.addr().get() + new_size;

                // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(end, config!(min_align));

                chunk.set_pos_addr(new_pos);
                Some(old_ptr.cast())
            } else {
                let old_addr = old_ptr.addr();
                let old_addr_old_end = NonZeroUsize::new_unchecked(old_addr.get() + old_size);

                let new_addr = bump_down(old_addr_old_end, new_size, T::ALIGN.max(config!(min_align)));
                let new_addr = NonZeroUsize::new_unchecked(new_addr);
                let new_ptr = old_ptr.with_addr(new_addr);

                old_ptr.copy_to(new_ptr, new_size);

                chunk.set_pos(new_ptr);
                Some(new_ptr.cast())
            }
        }
    }

    /// Prepares allocation for an instance of `T` memory.
    ///
    /// This is like [`alloc_sized`], except that it doesn't change the bump pointer.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// [`alloc_sized`]: BumpScope::alloc_sized
    #[inline(always)]
    pub fn prepare_sized_allocation<E: ErrorMode, T>(&self) -> Result<NonNull<T>, E> {
        match self.chunk().prepare_allocation(SizedLayout::new::<T>()) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.prepare_allocation_in_another_chunk::<E, T>() {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }
}

/// # Change config.
impl<const C: BumpConfig, A: Allocator> BumpScope<C, A> {
    /// Converts this `BumpScope` into a `BumpScope` with a new config.
    ///
    /// # Panics
    /// - Panics if [`claimable`] becomes false and [`self.is_claimed()`]
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`self.is_claimed()`]: crate::BumpScope::is_claimed
    #[inline]
    pub fn change_config<const NEW_C: BumpConfig>(self) -> BumpScope<NEW_C, A> {
        const_assert_upwards_matches::<C, NEW_C>();

        // Since `self` is by value, this can only happen if a claim guard was forgotten.
        self.assert_convertible_claimable::<NEW_C>();

        // No need to check const_constructible, a scope by value must have been created
        // by `(try_)by_value` which always makes it allocated.

        self.align_to(config!(NEW_C, min_align));

        unsafe { transmute_value(self) }
    }

    /// Borrows this `BumpScope` with a new config.
    ///
    /// # Panics
    /// - Panics if [`const_constructible`] becomes false and `self` has no chunks
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    /// - Fails to compile if [`claimable`] changes
    /// - Fails to compile if [`min_align`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    /// [`min_align`]: crate::config::BumpConfigBuilder::min_align
    #[inline]
    pub fn ref_change_config<const NEW_C: BumpConfig>(&self) -> &BumpScope<NEW_C, A> {
        self.ensure_satisfies_config_for_borrow::<NEW_C>();
        unsafe { transmute_ref(self) }
    }

    /// Borrows this `BumpScope` mutably with a new config.
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    /// - Fails to compile if [`claimable`] changes
    /// - Fails to compile if [`const_constructible`] changes
    /// - Fails to compile if [`min_align`] decreases
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    /// [`min_align`]: crate::config::BumpConfigBuilder::min_align
    #[inline]
    pub fn mut_change_config<const NEW_C: BumpConfig>(&mut self) -> &mut BumpScope<NEW_C, A> {
        self.ensure_satisfies_config_for_borrow_mut::<NEW_C>();
        unsafe { transmute_mut(self) }
    }

    /// Calls `f` with this scope but with a new config.
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    /// - Fails to compile if [`claimable`] changes
    /// - Fails to compile if [`const_constructible`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    /// # Examples
    ///
    /// Increase the minimum alignment:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, AsMutBumpScope, config::{BumpConfig, BumpConfigBuilder}};
    ///
    /// const MIN_ALIGN_8: BumpConfig = BumpConfigBuilder::new().min_align(8).build();
    ///
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_bump_scope();
    ///
    /// // here we're allocating with a `MIN_ALIGN` of `1`
    /// let foo = bump.alloc_str("foo").into_ref();
    /// assert_eq!(bump.allocated(), 3);
    ///
    /// let bar = bump.with_config::<MIN_ALIGN_8, _>(|bump| {
    ///     // in here the bump position has been aligned to `8`
    ///     assert_eq!(bump.allocated(), 8);
    ///     assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///
    ///     // make some allocations that benefit from the higher `MIN_ALIGN` of `8`
    ///     let bar = bump.alloc(0u64).into_ref();
    ///     assert_eq!(bump.allocated(), 16);
    ///  
    ///     // the bump position will stay aligned to `8`
    ///     bump.alloc(0u8).into_ref();
    ///     assert_eq!(bump.allocated(), 24);
    ///
    ///     bar
    /// });
    ///
    /// assert_eq!(bump.allocated(), 24);
    ///
    /// // continue making allocations with a `MIN_ALIGN` of `1`
    /// let baz = bump.alloc_str("baz").into_ref();
    /// assert_eq!(bump.allocated(), 24 + 3);
    ///
    /// dbg!(foo, bar, baz);
    /// ```
    ///
    /// Decrease the minimum alignment:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, AsMutBumpScope, config::{BumpConfig, BumpConfigBuilder}};
    ///
    /// const MIN_ALIGN_8: BumpConfig = BumpConfigBuilder::new().min_align(8).build();
    /// const MIN_ALIGN_1: BumpConfig = BumpConfigBuilder::new().build();
    ///
    /// let mut bump: Bump<MIN_ALIGN_8> = Bump::new();
    /// let bump = bump.as_mut_bump_scope();
    ///
    /// // make some allocations that benefit from the `MIN_ALIGN` of `8`
    /// let foo = bump.alloc(0u64).into_ref();
    ///
    /// let bar = bump.with_config::<MIN_ALIGN_1, _>(|bump| {
    ///     // make some allocations that benefit from the lower `MIN_ALIGN` of `1`
    ///     let bar = bump.alloc(0u8).into_ref();
    ///
    ///     // the bump position will not get aligned to `8` in here
    ///     assert_eq!(bump.allocated(), 8 + 1);
    ///
    ///     bar
    /// });
    ///
    /// // after `with_config()`, the bump position will be aligned to `8` again
    /// // to satisfy our `MIN_ALIGN`
    /// assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    /// assert_eq!(bump.allocated(), 16);
    ///
    /// // continue making allocations that benefit from the `MIN_ALIGN` of `8`
    /// let baz = bump.alloc(0u64);
    ///
    /// dbg!(foo, bar, baz);
    /// ```
    #[inline(always)]
    pub fn with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<NEW_C, A>) -> R) -> R {
        const_assert_upwards_matches::<C, NEW_C>();

        const {
            assert!(
                config!(NEW_C, claimable) == config!(claimable),
                "can't change `claimable` using `Bump(Scope)::with_config`"
            );

            assert!(
                config!(NEW_C, const_constructible) == config!(const_constructible),
                "can't change `const_constructible` using `Bump(Scope)::with_config`"
            );
        }

        if config!(NEW_C, min_align) < config!(min_align) {
            let guard = BumpAlignGuard::new(self);

            // SAFETY: bump is already aligned to the new alignment and the guard will ensure
            // that the bump pointer will again be aligned to old alignment once it drops
            let bump = unsafe { transmute_mut(guard.scope) };

            f(bump)
        } else {
            self.align_to(config!(NEW_C, min_align));

            // SAFETY: we aligned the bump pointer
            let bump = unsafe { transmute_mut(self) };

            f(bump)
        }
    }

    /// Calls `f` on a child scope with a new config.
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    ///
    /// # Panics
    ///
    /// - Panics if [`claimable`] becomes false and [`self.is_claimed()`]
    /// - Panics if [`const_constructible`] becomes false and `self` has no chunks
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`self.is_claimed()`]: crate::BumpScope::is_claimed
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, config::{BumpConfig, BumpConfigBuilder}};
    ///
    /// const CONFIG_MIN_ALIGN_8: BumpConfig = BumpConfigBuilder::new()
    ///     .min_align(8)
    ///     .build();
    ///
    /// let mut bump: Bump = Bump::new();
    ///
    /// // bump starts off by being aligned to 16
    /// assert!(bump.stats().bump_position().unwrap().addr().get() % 16 == 0);
    ///
    /// // allocate one byte
    /// bump.alloc(1u8).into_ref();
    ///
    /// // now the bump is only aligned to 1
    /// // (if our `MIN_ALIGN` was higher, it would be that)
    /// assert!(bump.stats().bump_position().unwrap().addr().get() % 2 == 1);
    /// assert_eq!(bump.allocated(), 1);
    ///
    /// bump.scoped_with_config::<CONFIG_MIN_ALIGN_8, ()>(|bump| {
    ///    // in here, the bump will have the specified minimum alignment of 8
    ///    assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///    assert_eq!(bump.allocated(), 8);
    ///
    ///    // allocating a value with its size being a multiple of 8 will no longer have
    ///    // to align the bump pointer before allocation
    ///    bump.alloc(1u64).into_ref();
    ///    assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///    assert_eq!(bump.allocated(), 16);
    ///    
    ///    // allocating a value smaller than the minimum alignment must align the bump pointer
    ///    // after the allocation, resulting in some wasted space
    ///    bump.alloc(1u8).into_ref();
    ///    assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///    assert_eq!(bump.allocated(), 24);
    /// });
    ///
    /// assert_eq!(bump.allocated(), 1);
    /// ```
    ///
    /// [claimed]: crate::BumpScope::claim
    #[inline(always)]
    pub fn scoped_with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<NEW_C, A>) -> R) -> R {
        const_assert_upwards_matches::<C, NEW_C>();

        self.assert_convertible_claimable::<NEW_C>();
        self.assert_convertible_const_constructible::<NEW_C>();

        // The scope guard will reset the bump pointer to the current position,
        // which is aligned to the current `min_align`.
        let mut scope = self.begin_scope();
        scope.align_to(config!(NEW_C, min_align));
        let scope: &mut BumpScope<C, A> = &mut scope;
        f(unsafe { transmute_mut(scope) })
    }

    fn can_convert_claimable<const NEW_C: BumpConfig>(&self) -> bool {
        // either nothing changed, the new config is claimable, the old one isn't, or the bump is currently not claimed
        (const {
            let old = config!(claimable);
            let new = config!(NEW_C, claimable);
            old == new || new || !old
        }) || !self.is_claimed()
    }

    pub(crate) fn assert_convertible_claimable<const NEW_C: BumpConfig>(&self) {
        if !self.can_convert_claimable::<NEW_C>() {
            config::cant_change_due_to_claimable();
        }
    }

    fn can_convert_const_constructible<const NEW_C: BumpConfig>(&self) -> bool {
        // To be able to convert const_constructible, either:
        // - nothing changed
        // - the new config is const_constructible
        // - the old config isn't const_constructible
        // - the bump currently has an allocated chunk
        (const {
            let old = config!(const_constructible);
            let new = config!(NEW_C, const_constructible);
            old == new || new || !old
        }) || self.stats().current_chunk().is_some()
    }

    pub(crate) fn assert_convertible_const_constructible<const NEW_C: BumpConfig>(&self) {
        if !self.can_convert_const_constructible::<NEW_C>() {
            config::cant_change_due_to_const_constructible();
        }
    }

    pub(crate) fn ensure_satisfies_config_for_borrow<const NEW_C: BumpConfig>(&self) {
        const_assert_upwards_matches::<C, NEW_C>();

        const {
            assert!(
                config!(NEW_C, claimable) == config!(claimable),
                "can't change `claimable` using `Bump(Scope)::ref_change_config`"
            );

            assert!(
                config!(NEW_C, min_align) == config!(min_align),
                "can't change `min_align` using `Bump(Scope)::ref_change_config`"
            );
        }

        // Can't make this bump allocator's chunk be the const constructed chunk
        // from a shared reference so this is enough.
        self.assert_convertible_const_constructible::<NEW_C>();
    }

    pub(crate) fn ensure_satisfies_config_for_borrow_mut<const NEW_C: BumpConfig>(&self) {
        const_assert_upwards_matches::<C, NEW_C>();

        const {
            assert!(
                config!(NEW_C, min_align) >= config!(min_align),
                "can't decrease `min_align` using `Bump(Scope)::mut_change_config`"
            );

            assert!(
                config!(NEW_C, claimable) == config!(claimable),
                "can't change `claimable` using `Bump(Scope)::mut_change_config`"
            );

            assert!(
                config!(NEW_C, const_constructible) == config!(const_constructible),
                "can't change `const_constructible` using `Bump(Scope)::mut_change_config`"
            );
        }

        self.align_to(config!(NEW_C, min_align));
    }

    #[inline(always)]
    pub(crate) fn align_to(&self, align: usize) {
        assert!(align.is_power_of_two() && align <= 16);

        if align > config!(min_align) {
            // a dummy chunk is always aligned
            if let Some(chunk) = self.chunk().as_allocated() {
                let pos = chunk.pos().addr().get();
                let addr = align_pos(config!(upwards), align, pos);
                unsafe { chunk.set_pos_addr(addr) };
            }
        }
    }
}

/// # Allocation slow paths.
impl<const C: BumpConfig, A: Allocator> BumpScope<C, A> {
    /// Allocation slow path.
    /// The active chunk must *not* have space for `layout`.
    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_in_another_chunk<E: ErrorMode>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        unsafe { self.in_another_chunk(CustomLayout(layout), Chunk::alloc) }
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_sized_in_another_chunk<E: ErrorMode, T>(&self) -> Result<NonNull<u8>, E> {
        self.alloc_in_another_chunk(Layout::new::<T>())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_slice_in_another_chunk<E: ErrorMode, T>(&self, len: usize) -> Result<NonNull<u8>, E> {
        let Ok(layout) = Layout::array::<T>(len) else {
            return Err(E::capacity_overflow());
        };

        self.alloc_in_another_chunk(layout)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn prepare_allocation_in_another_chunk<E: ErrorMode, T>(&self) -> Result<NonNull<u8>, E> {
        let layout = CustomLayout(Layout::new::<T>());
        unsafe { self.in_another_chunk(layout, Chunk::prepare_allocation) }
    }

    #[cold]
    #[inline(never)]
    fn prepare_allocation_range_in_another_chunk<E: ErrorMode>(&self, layout: ArrayLayout) -> Result<Range<NonNull<u8>>, E> {
        unsafe { self.in_another_chunk(layout, Chunk::prepare_allocation_range) }
    }

    /// # Safety
    ///
    /// `f` on the new chunk created by `RawChunk::append_for` with the layout `layout` must return `Some`.
    #[inline(always)]
    pub(crate) unsafe fn in_another_chunk<E: ErrorMode, R, L: LayoutProps>(
        &self,
        layout: L,
        mut f: impl FnMut(Chunk<C>, L) -> Option<R>,
    ) -> Result<R, E>
    where
        A: Allocator,
    {
        let new_chunk: AllocatedChunk<C> = match self.chunk().kind() {
            ChunkKind::Claimed => Err(E::claimed()),
            ChunkKind::ConstConstructed => {
                let allocator = self.allocator();
                let size = ChunkSizeCapacityFirst(*layout);
                AllocatedChunk::new_first::<E, A>(size, allocator)
            }
            ChunkKind::Allocated(mut chunk) => {
                while let Some(next_chunk) = chunk.next() {
                    chunk = next_chunk;

                    // We don't reset the chunk position when we leave a scope, so we need to do it here.
                    chunk.reset();

                    self.set_chunk(Chunk::from_allocated(chunk));

                    if let Some(ptr) = f(Chunk::from_allocated(chunk), layout) {
                        return Ok(ptr);
                    }
                }

                // there is no chunk that fits, we need a new chunk
                AllocatedChunk::new_non_first(*layout, chunk, self.allocator())
            }
        }?;

        self.set_chunk(Chunk::from_allocated(new_chunk));

        match f(Chunk::from_allocated(new_chunk), layout) {
            Some(ptr) => Ok(ptr),
            _ => {
                // SAFETY: We just appended a chunk for that specific layout, it must have enough space.
                // We don't panic here so we don't produce any panic code when using `try_` apis.
                // We check for that in `test-no-panic`.
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
    }
}

/// # Very low level apis.
impl<const C: BumpConfig, A: Allocator> BumpScope<C, A> {
    /// Returns the bump pointer.
    #[doc(hidden)]
    #[inline(always)]
    pub fn pos(&self) -> NonNull<u8> {
        self.chunk().pos()
    }

    /// Sets the bump pointer.
    #[doc(hidden)]
    #[inline(always)]
    pub unsafe fn set_pos(&self, addr: usize) {
        // SAFETY: callee must guarantee that the chunk is allocated
        unsafe { self.chunk().as_allocated_unchecked().set_pos_addr(addr) };
    }
}

unsafe impl<const C: BumpConfig, A: Allocator> Allocator for BumpScope<C, A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        allocator_impl::allocate(self, layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { allocator_impl::deallocate(self, ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::grow(self, ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::grow_zeroed(self, ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::shrink(self, ptr, old_layout, new_layout) }
    }
}
