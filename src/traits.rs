use core::{
    alloc::{Allocator, Layout},
    num::NonZeroUsize,
    ptr::NonNull,
};

use crate::{
    Bump, BumpScope, Checkpoint, SizedTypeProperties, Unmanaged, bump_down,
    config::{BumpConfig, config},
    error_behavior::ErrorBehavior,
    polyfill::{transmute_mut, transmute_ref},
    up_align_usize_unchecked,
};

pub(crate) unsafe trait Sealed {
    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///
    /// Behaves like:
    /// ```
    /// # use core::{alloc::Layout, alloc::{Allocator, AllocError}, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
    /// self.allocate(layout)?.cast::<u8>()
    /// #     }
    /// # }
    /// ```
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E>;

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E>;

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E>;

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E>;

    /// A specialized version of [`shrink`](core::alloc::Allocator::shrink).
    ///
    /// Behaves like the following code except that it returns `None`
    /// when the allocation remains unchanged and the pointer stays valid.
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # type T = i32;
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> NonNull<T> {
    /// #         unsafe {
    /// self.shrink(ptr.cast(),
    ///     Layout::array::<T>(old_len).unwrap_unchecked(),
    ///     Layout::array::<T>(new_len).unwrap_unchecked(),
    /// ).unwrap_unchecked().cast()
    /// #         }
    /// #     }
    /// # }
    /// ```
    ///
    /// # Safety
    ///
    /// Same safety conditions as for the code above apply.
    ///
    /// [shrink]: core::alloc::Allocator::shrink
    /// [array]: Layout::array
    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>>;

    /// A specialized version of [`prepare_allocation`].
    ///
    /// Returns a `[T]` of free space in the bump allocator.
    ///
    /// [`prepare_allocation`]: crate::traits::BumpAllocatorCore::prepare_allocation
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E>;

    /// A specialized version of [`allocate_prepared`].
    ///
    /// Allocates part of the free space returned from a
    /// <code>([try_](BumpAllocator::try_prepare_slice_allocation))[prepare_slice_allocation](BumpAllocator::prepare_slice_allocation)</code>
    /// call.
    ///
    /// # Safety
    /// - `ptr..ptr + cap` must be the pointer range returned from
    ///   <code>([try_](BumpAllocator::try_prepare_slice_allocation))[prepare_slice_allocation](BumpAllocator::prepare_slice_allocation)</code>.
    /// - no allocation, grow, shrink or deallocate must have taken place since then
    /// - no resets must have taken place since then
    /// - `len` must be less than or equal to `cap`
    ///
    /// [`allocate_prepared`]: BumpAllocatorCore::allocate_prepared
    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>;

    /// A specialized version of [`prepare_allocation_rev`].
    ///
    /// Returns an end pointer and length of free space in the bump allocator.
    ///
    /// [`prepare_allocation_rev`]: crate::traits::BumpAllocatorCore::prepare_allocation_rev
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn spec_prepare_slice_allocation_rev<E: ErrorBehavior, T>(&self, cap: usize) -> Result<(NonNull<T>, usize), E>;

    /// A specialized version of [`allocate_prepared_rev`].
    ///
    /// Allocates part of the free space returned from a
    /// <code>([try_](BumpAllocator::try_prepare_slice_allocation))[prepare_slice_allocation_rev](BumpAllocator::prepare_slice_allocation_rev)</code>
    /// call.
    ///
    /// # Safety
    /// - `ptr - cap..ptr` must be the pointer range returned from
    ///   <code>([try_](BumpAllocator::try_prepare_slice_allocation))[prepare_slice_allocation_rev](BumpAllocator::prepare_slice_allocation_rev)</code>.
    /// - no allocation, grow, shrink or deallocate must have taken place since then
    /// - no resets must have taken place since then
    /// - `len` must be less than or equal to `cap`
    ///
    /// [`allocate_prepared_rev`]: crate::traits::BumpAllocatorCore::allocate_prepared_rev
    unsafe fn spec_alloc_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>;
}
unsafe impl<A: Sealed> Sealed for &A {
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        A::spec_alloc(self, layout)
    }

    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        A::spec_alloc_sized(self)
    }

    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        A::spec_alloc_slice(self, len)
    }

    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        A::spec_alloc_slice_for(self, slice)
    }

    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { A::spec_shrink_slice(self, ptr, old_len, new_len) }
    }

    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        A::spec_prepare_slice_allocation(self, cap)
    }

    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { A::spec_alloc_prepared_slice(self, start, len, cap) }
    }

    fn spec_prepare_slice_allocation_rev<E: ErrorBehavior, T>(&self, cap: usize) -> Result<(NonNull<T>, usize), E> {
        A::spec_prepare_slice_allocation_rev(self, cap)
    }

    unsafe fn spec_alloc_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { A::spec_alloc_prepared_slice_rev(self, ptr, len, cap) }
    }
}
unsafe impl<A: Sealed> Sealed for &mut A {
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        A::spec_alloc(self, layout)
    }

    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        A::spec_alloc_sized(self)
    }

    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        A::spec_alloc_slice(self, len)
    }

    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        A::spec_alloc_slice_for(self, slice)
    }

    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { A::spec_shrink_slice(self, ptr, old_len, new_len) }
    }

    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        A::spec_prepare_slice_allocation(self, cap)
    }

    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { A::spec_alloc_prepared_slice(self, start, len, cap) }
    }

    fn spec_prepare_slice_allocation_rev<E: ErrorBehavior, T>(&self, cap: usize) -> Result<(NonNull<T>, usize), E> {
        A::spec_prepare_slice_allocation_rev(self, cap)
    }

    unsafe fn spec_alloc_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { A::spec_alloc_prepared_slice_rev(self, ptr, len, cap) }
    }
}
unsafe impl<A: Allocator, const C: BumpConfig> Sealed for Bump<A, C> {
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        self.as_bump_scope().spec_alloc(layout)
    }

    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        self.as_bump_scope().spec_alloc_sized()
    }

    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        self.as_bump_scope().spec_alloc_slice(len)
    }

    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        self.as_bump_scope().spec_alloc_slice_for(slice)
    }

    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { self.as_bump_scope().spec_shrink_slice(ptr, old_len, new_len) }
    }

    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        self.as_bump_scope().spec_prepare_slice_allocation(cap)
    }

    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { self.as_bump_scope().spec_alloc_prepared_slice(start, len, cap) }
    }

    fn spec_prepare_slice_allocation_rev<E: ErrorBehavior, T>(&self, cap: usize) -> Result<(NonNull<T>, usize), E> {
        self.as_bump_scope().spec_prepare_slice_allocation_rev(cap)
    }

    unsafe fn spec_alloc_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { self.as_bump_scope().spec_alloc_prepared_slice_rev(ptr, len, cap) }
    }
}
unsafe impl<A: Allocator, const C: BumpConfig> Sealed for BumpScope<'_, A, C> {
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        self.raw.allocate(layout)
    }

    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        self.raw.allocate_sized()
    }

    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        self.raw.allocate_slice(len)
    }

    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        self.raw.allocate_slice_for(slice)
    }

    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        if !config!(shrinks) {
            return None;
        }

        let old_ptr = ptr.cast::<u8>();
        let old_size = old_len * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_len * T::SIZE; // its less than the capacity so this can't overflow

        // Adapted from `Allocator::shrink`.
        unsafe {
            let chunk = self.raw.chunk.get();

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
                let old_addr_new_end = NonZeroUsize::new_unchecked(old_addr.get() + new_size);

                let new_ptr = old_ptr.with_addr(new_addr);
                let overlaps = old_addr_new_end > new_addr;

                if overlaps {
                    old_ptr.copy_to(new_ptr, new_size);
                } else {
                    old_ptr.copy_to_nonoverlapping(new_ptr, new_size);
                }

                chunk.set_pos(new_ptr);
                Some(new_ptr.cast())
            }
        }
    }

    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        self.raw.prepare_slice_allocation(cap)
    }

    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe {
            // a successful `prepare_allocation` guarantees a non-dummy-chunk
            let chunk = self.raw.chunk.get().as_allocated_unchecked();

            let end = start.add(len);

            if config!(upwards) {
                chunk.set_pos_addr_and_align_from(end.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(start, len)
            } else {
                let dst_end = start.add(cap);
                let dst = dst_end.sub(len);
                start.copy_to(dst, len);
                chunk.set_pos_addr_and_align_from(dst.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            }
        }
    }

    fn spec_prepare_slice_allocation_rev<E: ErrorBehavior, T>(&self, cap: usize) -> Result<(NonNull<T>, usize), E> {
        self.raw.prepare_slice_allocation_rev(cap)
    }

    unsafe fn spec_alloc_prepared_slice_rev<T>(&self, end: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe {
            // a successful `prepare_allocation` guarantees a non-dummy-chunk
            let chunk = self.raw.chunk.get().as_allocated_unchecked();

            if config!(upwards) {
                let dst = end.sub(cap);
                let dst_end = dst.add(len);

                let src = end.sub(len);

                src.copy_to(dst, len);

                chunk.set_pos_addr_and_align_from(dst_end.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            } else {
                let dst = end.sub(len);
                chunk.set_pos_addr_and_align_from(dst.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            }
        }
    }
}
unsafe impl Sealed for &Unmanaged {
    #[inline(always)]
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        _ = layout;
        Err(E::unmanaged())
    }

    #[inline(always)]
    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        Err(E::unmanaged())
    }

    #[inline(always)]
    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        _ = len;
        Err(E::unmanaged())
    }

    #[inline(always)]
    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        _ = slice;
        Err(E::unmanaged())
    }

    #[inline(always)]
    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        _ = (ptr, old_len, new_len);
        None
    }

    #[inline(always)]
    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        _ = cap;
        Err(E::unmanaged())
    }

    #[inline(always)]
    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        _ = (start, len, cap);
        NonNull::slice_from_raw_parts(NonNull::dangling(), 0)
    }

    #[inline(always)]
    fn spec_prepare_slice_allocation_rev<E: ErrorBehavior, T>(&self, cap: usize) -> Result<(NonNull<T>, usize), E> {
        _ = cap;
        Err(E::unmanaged())
    }

    #[inline(always)]
    unsafe fn spec_alloc_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        _ = (ptr, len, cap);
        NonNull::slice_from_raw_parts(NonNull::dangling(), 0)
    }
}

pub unsafe trait BumpAllocator: Allocator + Sealed {
    /// Creates a checkpoint of the current bump position.
    ///
    /// The bump position can be reset to this checkpoint with [`reset_to`].
    ///
    /// [`reset_to`]: BumpAllocatorCore::reset_to
    #[must_use]
    fn checkpoint(&self) -> Checkpoint;

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
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.stats().allocated(), 5);
    ///     # _ = hello;
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    unsafe fn reset_to(&self, checkpoint: Checkpoint);
}
unsafe impl<T: BumpAllocator> BumpAllocator for &T {
    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        T::checkpoint(self)
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe { T::reset_to(self, checkpoint) };
    }
}
unsafe impl<T: BumpAllocator> BumpAllocator for &mut T {
    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        T::checkpoint(self)
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe { T::reset_to(self, checkpoint) };
    }
}
unsafe impl<A: Allocator, const C: BumpConfig> BumpAllocator for Bump<A, C> {
    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        self.raw.checkpoint()
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe {
            self.raw.reset_to(checkpoint);
        };
    }
}
unsafe impl<A: Allocator, const C: BumpConfig> BumpAllocator for BumpScope<'_, A, C> {
    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        self.raw.checkpoint()
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe {
            self.raw.reset_to(checkpoint);
        };
    }
}
unsafe impl BumpAllocator for &Unmanaged {
    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        Checkpoint::INVALID
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        _ = checkpoint;
    }
}

/// TODO
pub unsafe trait BumpAllocatorScope<'a>: BumpAllocator + Sealed {}
unsafe impl<'a, T: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for &T {}
unsafe impl<'a, T: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for &mut T {}
unsafe impl<'a, A: Allocator, const C: BumpConfig> BumpAllocatorScope<'a> for &'a Bump<A, C> {}
unsafe impl<'a, A: Allocator, const C: BumpConfig> BumpAllocatorScope<'a> for &'a mut Bump<A, C> {}
unsafe impl<'a, A: Allocator, const C: BumpConfig> BumpAllocatorScope<'a> for BumpScope<'a, A, C> {}
unsafe impl<'a> BumpAllocatorScope<'a> for &'a Unmanaged {}

/// TODO
pub unsafe trait MutBumpAllocator: BumpAllocator + Sealed {}
unsafe impl<T: MutBumpAllocator> MutBumpAllocator for &mut T {}
unsafe impl<A: Allocator, const C: BumpConfig> MutBumpAllocator for Bump<A, C> {}
unsafe impl<A: Allocator, const C: BumpConfig> MutBumpAllocator for BumpScope<'_, A, C> {}

/// TODO
pub unsafe trait MutBumpAllocatorScope<'a>: MutBumpAllocator + BumpAllocatorScope<'a> + Sealed {}
unsafe impl<'a, T: MutBumpAllocator + BumpAllocatorScope<'a>> MutBumpAllocatorScope<'a> for T {}

/// TODO
pub trait AsBumpScope<'a, A, const C: BumpConfig> {
    /// TODO
    fn as_bump_scope(&self) -> &BumpScope<'a, A, C>;
}

impl<'a, A: Allocator, const C: BumpConfig> AsBumpScope<'a, A, C> for &'a Bump<A, C> {
    fn as_bump_scope(&self) -> &BumpScope<'a, A, C> {
        unsafe { transmute_ref::<Bump<A, C>, BumpScope<'a, A, C>>(*self) }
    }
}

impl<'a, A: Allocator, const C: BumpConfig> AsBumpScope<'a, A, C> for BumpScope<'a, A, C> {
    fn as_bump_scope(&self) -> &BumpScope<'a, A, C> {
        self
    }
}

pub trait AsMutBumpScope<'a, A, const C: BumpConfig> {
    fn as_mut_bump_scope(&mut self) -> &mut BumpScope<'a, A, C>;
}

impl<'a, A: Allocator, const C: BumpConfig> AsMutBumpScope<'a, A, C> for &'a mut Bump<A, C> {
    fn as_mut_bump_scope(&mut self) -> &mut BumpScope<'a, A, C> {
        unsafe { transmute_mut::<Bump<A, C>, BumpScope<'a, A, C>>(*self) }
    }
}

impl<'a, A: Allocator, const C: BumpConfig> AsMutBumpScope<'a, A, C> for BumpScope<'a, A, C> {
    fn as_mut_bump_scope(&mut self) -> &mut BumpScope<'a, A, C> {
        self
    }
}

macro_rules! assert_implements {
    ([$($what:tt)*] $($ty:ty)*) => {
        #[cfg(test)]
        const _: () = {
            #[allow(dead_code)]
            type A = crate::NoopAllocator;
            #[allow(dead_code)]
            type Bump = crate::Bump<A>;
            #[allow(dead_code)]
            type BumpScope<'a> = crate::BumpScope<'a, A>;
            #[allow(clippy::extra_unused_lifetimes)]
            const fn implements<'a, What: $($what)*>() {}
            $(
                #[allow(clippy::mut_mut)]
                implements::<$ty>();
            )*
        };
    };
}

assert_implements! {
    [BumpAllocator + ?Sized]

    Bump
    &Bump
    &&Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &BumpScope
    &&BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

assert_implements! {
    [BumpAllocatorScope<'a> + ?Sized]

    &Bump
    &&Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &BumpScope
    &&BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

assert_implements! {
    [MutBumpAllocator + ?Sized]

    Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

assert_implements! {
    [MutBumpAllocatorScope<'a> + ?Sized]

    &mut Bump
    &mut &mut Bump

    BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}
