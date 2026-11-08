use core::{
    alloc::{AllocError, Allocator, Layout},
    cell::Cell,
    marker::PhantomData,
    num::NonZeroUsize,
    ops::Range,
    ptr::{self, NonNull},
};

use crate::{
    Bump, BumpScope, Checkpoint, Unmanaged, allocator_impl,
    bump_scope::BumpScopeWithoutLifetime,
    chunk_size::{ChunkSizeCapacityFirst, ChunkSizeDefault},
    chunk_types::{AllocatedChunk, Chunk, ChunkKind},
    config::{self, BumpConfig, DEFAULT_BUMP_CONFIG, config},
    error_mode::ErrorMode,
    layout::{ArrayLayout, CustomLayout, LayoutProps, SizedLayout},
    polyfill::hint::unlikely,
    stats::Stats,
    util::{SizedTypeProperties, align_pos, bump_down, up_align_usize_unchecked},
};

/// Provides bump allocation functionality for both `Bump` and `BumpScope`.
pub(crate) struct BumpImp<'a, const C: BumpConfig, A, const IS_SCOPE: bool> {
    /// Chunk is always dereferenceable.
    pub(crate) chunk: NonNull<Cell<Chunk<C>>>,
    pub(crate) marker: PhantomData<&'a A>,
}

/// Must match the repr of [`Bump`]!
#[repr(C)]
struct BumpData<const C: BumpConfig, A> {
    chunk: Cell<Chunk<C>>,
    allocator_ptr: Cell<*const A>,
    allocator: A,
}

/// Must match the repr of [`BumpScope`]!
#[repr(C)]
struct BumpScopeData<const C: BumpConfig, A> {
    chunk: Cell<Chunk<C>>,
    allocator_ptr: Cell<*const A>,
}

impl<const C: BumpConfig, A, const IS_SCOPE: bool> Clone for BumpImp<'_, C, A, IS_SCOPE> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const C: BumpConfig, A, const IS_SCOPE: bool> Copy for BumpImp<'_, C, A, IS_SCOPE> {}

impl<'a, const C: BumpConfig, A: Allocator, const IS_SCOPE: bool> BumpImp<'a, C, A, IS_SCOPE> {
    #[inline(always)]
    pub(crate) fn chunk(self) -> Chunk<C> {
        unsafe { self.chunk.as_ref() }.get()
    }

    #[inline(always)]
    pub(crate) fn set_chunk(self, chunk: Chunk<C>) {
        unsafe { self.chunk.as_ref() }.set(chunk);
    }

    #[inline(always)]
    pub(crate) fn allocator(self) -> &'a A {
        if IS_SCOPE {
            unsafe { &*(*self.chunk.as_ptr().cast::<BumpScopeData<C, A>>()).allocator_ptr.get() }
        } else {
            unsafe { &(*self.chunk.as_ptr().cast::<BumpData<C, A>>()).allocator }
        }
    }

    /// Reset's the bump pointer to the very start.
    #[inline]
    pub(crate) fn reset_to_start(self) {
        let Some(mut chunk) = self.chunk().as_allocated() else {
            return;
        };

        while let Some(prev) = chunk.prev() {
            chunk = prev;
        }

        chunk.reset();

        self.set_chunk(Chunk::from_allocated(chunk));
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(self) -> Stats<'a, C, A> {
        Stats::from_raw_chunk(self.chunk())
    }

    #[inline(always)]
    pub(crate) fn align_to(self, align: usize) {
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

    pub(crate) const fn const_assert_upwards_matches<const NEW_C: BumpConfig>() {
        const {
            let old = config!(upwards);
            let new = config!(NEW_C, upwards);
            assert!(old == new, "can't change bump direction");
        }
    }

    fn can_convert_claimable<const NEW_C: BumpConfig>(self) -> bool {
        // either nothing changed, the new config is claimable, the old one isn't, or the bump is currently not claimed
        (const {
            let old = config!(claimable);
            let new = config!(NEW_C, claimable);
            old == new || new || !old
        }) || !self.is_claimed()
    }

    pub(crate) fn assert_convertible_claimable<const NEW_C: BumpConfig>(self) {
        if !self.can_convert_claimable::<NEW_C>() {
            config::cant_change_due_to_claimable();
        }
    }

    fn can_convert_const_constructible<const NEW_C: BumpConfig>(self) -> bool {
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

    pub(crate) fn assert_convertible_const_constructible<const NEW_C: BumpConfig>(self) {
        if !self.can_convert_const_constructible::<NEW_C>() {
            config::cant_change_due_to_const_constructible();
        }
    }

    pub(crate) fn ensure_satisfies_config_for_borrow<const NEW_C: BumpConfig>(self) {
        Self::const_assert_upwards_matches::<NEW_C>();

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

        // Can't make this bump allocator's chunk be the const constructor chunk
        // from a shared reference so this is enough.
        self.assert_convertible_const_constructible::<NEW_C>();
    }

    pub(crate) fn ensure_satisfies_config_for_borrow_mut<const NEW_C: BumpConfig>(self) {
        Self::const_assert_upwards_matches::<NEW_C>();

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

    /// Returns a pointer range.
    /// The start and end pointers are aligned.
    /// But `end - start` is *not* a multiple of `size_of::<T>()`.
    /// So `end.offset_from_unsigned(start)` may not be used!
    #[inline(always)]
    pub(crate) fn prepare_allocation_range<B: ErrorMode, T>(self, cap: usize) -> Result<Range<NonNull<T>>, B> {
        let Ok(layout) = ArrayLayout::array::<T>(cap) else {
            return Err(B::capacity_overflow());
        };

        let range = match self.chunk().prepare_allocation_range(layout) {
            Some(ptr) => ptr,
            None => self.prepare_allocation_range_in_another_chunk(layout)?,
        };

        Ok(range.start.cast::<T>()..range.end.cast::<T>())
    }

    /// Allocation slow path.
    /// The active chunk must *not* have space for `layout`.
    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_in_another_chunk<E: ErrorMode>(self, layout: Layout) -> Result<NonNull<u8>, E> {
        unsafe { self.in_another_chunk(CustomLayout(layout), Chunk::alloc) }
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_sized_in_another_chunk<E: ErrorMode, T>(self) -> Result<NonNull<u8>, E> {
        self.alloc_in_another_chunk(Layout::new::<T>())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_slice_in_another_chunk<E: ErrorMode, T>(self, len: usize) -> Result<NonNull<u8>, E> {
        let Ok(layout) = Layout::array::<T>(len) else {
            return Err(E::capacity_overflow());
        };

        self.alloc_in_another_chunk(layout)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn prepare_allocation_in_another_chunk<E: ErrorMode, T>(self) -> Result<NonNull<u8>, E> {
        let layout = CustomLayout(Layout::new::<T>());
        unsafe { self.in_another_chunk(layout, Chunk::prepare_allocation) }
    }

    #[cold]
    #[inline(never)]
    fn prepare_allocation_range_in_another_chunk<E: ErrorMode>(self, layout: ArrayLayout) -> Result<Range<NonNull<u8>>, E> {
        unsafe { self.in_another_chunk(layout, Chunk::prepare_allocation_range) }
    }

    /// # Safety
    ///
    /// `f` on the new chunk created by `RawChunk::append_for` with the layout `layout` must return `Some`.
    #[inline(always)]
    pub(crate) unsafe fn in_another_chunk<E: ErrorMode, R, L: LayoutProps>(
        self,
        layout: L,
        mut f: impl FnMut(Chunk<C>, L) -> Option<R>,
    ) -> Result<R, E>
    where
        A: Allocator,
    {
        let new_chunk: AllocatedChunk<C> = match self.chunk().kind() {
            ChunkKind::Claimed => Err(E::claimed()),
            ChunkKind::ConstConstructor => {
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

unsafe impl<const C: BumpConfig, A: Allocator, const IS_SCOPE: bool> Allocator for BumpImp<'_, C, A, IS_SCOPE> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        allocator_impl::allocate(*self, layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { allocator_impl::deallocate(*self, ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::grow(*self, ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::grow_zeroed(*self, ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::shrink(*self, ptr, old_layout, new_layout) }
    }
}

pub(crate) trait BumpImpTrait<'a> {
    const CONFIG: BumpConfig;

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
    fn alloc<E: ErrorMode>(&self, layout: Layout) -> Result<NonNull<u8>, E>;

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
    fn alloc_sized<E: ErrorMode, T>(&self) -> Result<NonNull<T>, E>;

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
    fn alloc_slice<E: ErrorMode, T>(&self, len: usize) -> Result<NonNull<T>, E>;

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
    fn alloc_slice_for<E: ErrorMode, T>(&self, slice: &[T]) -> Result<NonNull<T>, E>;

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///
    /// The allocation takes up as much of the free space of the chunk as possible.
    fn prepare_slice<E: ErrorMode, T>(&self, len: usize) -> Result<NonNull<[T]>, E>;

    /// Allocate part of the slice returned from a [`spec_prepare_slice`] call.
    ///
    /// # Safety
    /// - `ptr` and `cap` must form the slice that has been returned from [`spec_prepare_slice`]
    /// - no allocation, grow, shrink or deallocate must have taken place since then
    /// - no resets must have taken place since then
    /// - `len` must be less than or equal to `cap`
    /// - the bump allocator must be [unclaimed] at the time [`spec_prepare_slice`] was called and when calling this function
    ///
    /// [`spec_prepare_slice`]: BumpAllocatorPrivate::spec_prepare_slice
    /// [unclaimed]: crate::BumpScope::claim
    unsafe fn commit_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>;

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
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>>;

    /// Prepares allocation for an instance of `T` memory.
    ///
    /// This is like [`spec_alloc_sized`], except that it doesn't change the bump pointer.
    ///
    /// [`spec_alloc_sized`]: BumpAllocatorPrivate::spec_alloc_sized
    fn prepare_sized_allocation<B: ErrorMode, T>(&self) -> Result<NonNull<T>, B>;

    /// See [`BumpAllocator::checkpoint`].
    #[must_use]
    fn checkpoint(&self) -> Checkpoint;

    /// See [`BumpAllocator::reset_to`].
    unsafe fn reset_to(&self, checkpoint: Checkpoint);

    /// Returns the bump pointer.
    fn pos(&self) -> *mut u8;

    /// Sets the bump pointer.
    ///
    /// # Safety
    /// Chunk must be allocated.
    unsafe fn set_pos(&self, addr: usize);

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    fn reserve<E: ErrorMode>(self, additional: usize) -> Result<(), E>;

    fn make_allocated<E: ErrorMode>(self) -> Result<(), E>;

    #[expect(clippy::wrong_self_convention)]
    fn is_claimed(self) -> bool;

    fn allocated(&self) -> usize;
}

impl<'a, const C: BumpConfig, A: Allocator, const IS_SCOPE: bool> BumpImpTrait<'a> for BumpImp<'a, C, A, IS_SCOPE> {
    const CONFIG: BumpConfig = C;

    #[inline(always)]
    fn alloc<E: ErrorMode>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        match self.chunk().alloc(CustomLayout(layout)) {
            Some(ptr) => Ok(ptr),
            None => self.alloc_in_another_chunk(layout),
        }
    }

    #[inline(always)]
    fn alloc_sized<E: ErrorMode, T>(&self) -> Result<NonNull<T>, E> {
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

    #[inline(always)]
    fn alloc_slice<E: ErrorMode, T>(&self, len: usize) -> Result<NonNull<T>, E> {
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

    #[inline(always)]
    fn alloc_slice_for<E: ErrorMode, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        let layout = ArrayLayout::for_value(slice);

        match self.chunk().alloc(layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.alloc_slice_in_another_chunk::<E, T>(slice.len()) {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline(always)]
    fn prepare_slice<E: ErrorMode, T>(&self, len: usize) -> Result<NonNull<[T]>, E> {
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

    #[inline(always)]
    unsafe fn commit_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
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

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
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

    #[inline(always)]
    fn prepare_sized_allocation<B: ErrorMode, T>(&self) -> Result<NonNull<T>, B> {
        match self.chunk().prepare_allocation(SizedLayout::new::<T>()) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.prepare_allocation_in_another_chunk::<B, T>() {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline]
    fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk())
    }

    #[cfg_attr(not(debug_assertions), inline(always))]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        if unlikely(config!(claimable) && checkpoint.chunk.is_claimed()) {
            // We do nothing if the checkpoint is claimed.
            // This state is reachable in safe code when a `BumpClaimGuard` is forgotten.
            return;
        }

        // If the checkpoint was created when the bump allocator had no allocated chunk
        // then the chunk pointer will point to the const constructor chunk.
        //
        // In such cases we reset the bump pointer to the very start of the very first chunk.
        //
        // We don't check if the chunk pointer points to the const constructor chunk if the bump allocator is **not** `const_constructible`.
        // We are allowed to not do this check because of this safety condition of `reset_to`:
        // > the checkpoint must not have been created by a `const_constructible` when self is **not** `const_constructible`
        if unlikely(config!(const_constructible) && checkpoint.chunk.is_const_constructor()) {
            self.reset_to_start();
            return;
        }

        #[cfg(debug_assertions)]
        {
            assert!(
                !checkpoint.chunk.is_const_constructor(),
                "the checkpoint must not have been created by a `const_constructible` when self is not `const_constructible`"
            );

            if !self.is_claimed() {
                let chunk = self
                    .stats()
                    .small_to_big()
                    .find(|chunk| chunk.header().addr() == checkpoint.chunk.as_ptr().addr())
                    .expect("this checkpoint does not refer to any chunk in this bump allocator");

                let start = chunk.content_start().addr().get();
                let end = chunk.content_end().addr().get();
                let addr = checkpoint.address.get();

                assert!(
                    addr >= start && addr <= end,
                    "checkpoint address is not in the chunk's content range"
                );
            }
        }

        // We handled the const constructor case above.
        // It can't be a claimed chunk because that would not be allowed due to the safety invariants.
        unsafe {
            // Reset the chunk position of the checkpoint chunk.
            let ptr = checkpoint.chunk.as_ptr().with_addr(checkpoint.address).cast();
            AllocatedChunk::<C>::from_ptr(checkpoint.chunk.as_ptr()).set_pos(ptr);

            self.set_chunk(Chunk::from_ptr(checkpoint.chunk.as_ptr()));
        }
    }

    #[inline]
    fn pos(&self) -> *mut u8 {
        self.chunk().pos().as_ptr()
    }

    #[inline]
    unsafe fn set_pos(&self, addr: usize) {
        // SAFETY: callee must guarantee that the chunk is allocated
        unsafe { self.chunk().as_allocated_unchecked().set_pos_addr(addr) };
    }

    #[inline]
    fn reserve<E: ErrorMode>(self, additional: usize) -> Result<(), E> {
        let chunk = self.chunk();

        match chunk.kind() {
            ChunkKind::Claimed => Err(E::claimed()),
            ChunkKind::ConstConstructor => {
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

    #[inline]
    fn make_allocated<E: ErrorMode>(self) -> Result<(), E> {
        match self.chunk().kind() {
            ChunkKind::Claimed => Err(E::claimed()),
            ChunkKind::ConstConstructor => {
                let allocator = self.allocator();
                let chunk = Chunk::from_allocated(AllocatedChunk::new_first::<E, A>(ChunkSizeDefault, allocator)?);
                self.set_chunk(chunk);
                Ok(())
            }
            ChunkKind::Allocated(_) => Ok(()),
        }
    }

    #[inline(always)]
    fn is_claimed(self) -> bool {
        self.chunk().is_claimed()
    }

    #[inline(always)]
    fn allocated(&self) -> usize {
        self.stats().allocated()
    }
}

impl<'a> BumpImpTrait<'a> for Unmanaged<'a> {
    // FIXME: make this an invalid bump config?
    const CONFIG: BumpConfig = DEFAULT_BUMP_CONFIG;

    fn alloc<E: ErrorMode>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        _ = layout;
        Err(E::unmanaged())
    }

    fn alloc_sized<E: ErrorMode, T>(&self) -> Result<NonNull<T>, E> {
        Err(E::unmanaged())
    }

    fn alloc_slice<E: ErrorMode, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        _ = len;
        Err(E::unmanaged())
    }

    fn alloc_slice_for<E: ErrorMode, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        _ = slice;
        Err(E::unmanaged())
    }

    fn prepare_slice<E: ErrorMode, T>(&self, len: usize) -> Result<NonNull<[T]>, E> {
        _ = len;
        Err(E::unmanaged())
    }

    unsafe fn commit_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        _ = (ptr, len, cap);
        NonNull::slice_from_raw_parts(NonNull::dangling(), 0)
    }

    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        _ = (ptr, old_len, new_len);
        None
    }

    fn prepare_sized_allocation<B: ErrorMode, T>(&self) -> Result<NonNull<T>, B> {
        Err(B::unmanaged())
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint::INVALID
    }

    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        _ = checkpoint;
    }

    fn pos(&self) -> *mut u8 {
        ptr::dangling_mut()
    }

    unsafe fn set_pos(&self, addr: usize) {
        _ = addr;
    }

    fn reserve<E: ErrorMode>(self, additional: usize) -> Result<(), E> {
        _ = additional;
        Err(E::unmanaged())
    }

    fn make_allocated<E: ErrorMode>(self) -> Result<(), E> {
        Err(E::unmanaged())
    }

    fn is_claimed(self) -> bool {
        false
    }

    fn allocated(&self) -> usize {
        0
    }
}

pub(crate) trait AsBumpImp {
    type Type<'a>: BumpImpTrait<'a>
    where
        Self: 'a;

    fn imp(&self) -> Self::Type<'_>;
}

impl<const C: BumpConfig, A: Allocator, const IS_SCOPE: bool> AsBumpImp for BumpImp<'_, C, A, IS_SCOPE> {
    type Type<'a>
        = BumpImp<'a, C, A, IS_SCOPE>
    where
        Self: 'a;

    fn imp(&self) -> Self::Type<'_> {
        *self
    }
}

impl<T: AsBumpImp> AsBumpImp for &T {
    type Type<'a>
        = T::Type<'a>
    where
        Self: 'a;

    fn imp(&self) -> Self::Type<'_> {
        T::imp(self)
    }
}

impl<T: AsBumpImp> AsBumpImp for &mut T {
    type Type<'a>
        = T::Type<'a>
    where
        Self: 'a;

    fn imp(&self) -> Self::Type<'_> {
        T::imp(self)
    }
}

impl<const C: BumpConfig, A: Allocator> AsBumpImp for Bump<C, A> {
    type Type<'a>
        = BumpImp<'a, C, A, false>
    where
        Self: 'a;

    fn imp(&self) -> Self::Type<'_> {
        BumpImp {
            chunk: NonNull::from(self).cast(),
            marker: PhantomData,
        }
    }
}

impl<const C: BumpConfig, A: Allocator> AsBumpImp for BumpScope<'_, C, A> {
    type Type<'a>
        = BumpImp<'a, C, A, true>
    where
        Self: 'a;

    fn imp(&self) -> Self::Type<'_> {
        BumpImp {
            chunk: NonNull::from(self).cast(),
            marker: PhantomData,
        }
    }
}

impl<const C: BumpConfig, A: Allocator> AsBumpImp for BumpScopeWithoutLifetime<C, A> {
    type Type<'a>
        = BumpImp<'a, C, A, true>
    where
        Self: 'a;

    fn imp(&self) -> Self::Type<'_> {
        BumpImp {
            chunk: NonNull::from(self).cast(),
            marker: PhantomData,
        }
    }
}

impl AsBumpImp for Unmanaged<'_> {
    type Type<'a>
        = Unmanaged<'a>
    where
        Self: 'a;

    fn imp(&self) -> Self::Type<'_> {
        Unmanaged::VALUE
    }
}
