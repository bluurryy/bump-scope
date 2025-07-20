use core::{alloc::Layout, ops::Range, ptr::NonNull};

use crate::{
    alloc::{AllocError, Allocator},
    polyfill::non_null,
    BaseAllocator, Bump, BumpAllocatorExt, BumpScope, MinimumAlignment, MutBumpAllocator, SizedTypeProperties,
    SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// A trait that adds specialized methods for [`MutBumpAllocator`]s.
pub unsafe trait MutBumpAllocatorExt: MutBumpAllocator + BumpAllocatorExt {
    /// Does not allocate, just returns a slice of `T` that are currently available.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> Range<NonNull<T>>;

    /// Does not allocate, just returns a slice of `T` that are currently available.
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<Range<NonNull<T>>, AllocError>;

    /// Allocate part of a valid free slice returned by `(try_)prepare_slice_allocation`.
    ///
    /// # Safety
    ///
    /// - `ptr + cap` must be a slice returned by `(try_)prepare_slice_allocation`. No allocation,
    ///   grow, shrink or deallocate must have been called since then.
    /// - `len` must be less than or equal to `cap`
    unsafe fn allocate_prepared_slice<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>;

    /// Allocate part of a valid free slice returned by `(try_)prepare_slice_allocation`.
    ///
    /// # Safety
    ///
    /// - `ptr + cap` must be a slice returned by `(try_)prepare_slice_allocation`. No allocation,
    ///   grow, shrink or deallocate must have been called since then.
    /// - `len` must be less than or equal to `cap`
    unsafe fn allocate_prepared_slice_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>;
}

unsafe impl MutBumpAllocatorExt for dyn MutBumpAllocator + '_ {
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> Range<NonNull<T>> {
        self.try_prepare_slice_allocation(len).unwrap()
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<Range<NonNull<T>>, AllocError> {
        match self.prepare_allocation(Layout::array::<T>(len).unwrap()) {
            Ok(range) => Ok(range.start.cast()..range.end.cast()),
            Err(err) => Err(err),
        }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        let range = non_null::cast_range(ptr..non_null::add(ptr, cap));
        let layout = Layout::from_size_align_unchecked(core::mem::size_of::<T>() * len, T::ALIGN);
        let data = self.allocate_prepared(layout, range).cast();
        non_null::slice_from_raw_parts(data, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        let range = non_null::cast_range(non_null::sub(ptr, cap)..ptr);
        let layout = Layout::from_size_align_unchecked(core::mem::size_of::<T>() * len, T::ALIGN);
        let data = self.allocate_prepared_rev(layout, range).cast();
        non_null::slice_from_raw_parts(data, len)
    }
}

unsafe impl<A: MutBumpAllocatorExt> MutBumpAllocatorExt for &mut A
where
    for<'a> &'a mut A: Allocator,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> Range<NonNull<T>> {
        A::prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<Range<NonNull<T>>, AllocError> {
        A::try_prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        A::allocate_prepared_slice(self, ptr, len, cap)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        A::allocate_prepared_slice_rev(self, ptr, len, cap)
    }
}

unsafe impl<A: MutBumpAllocatorExt> MutBumpAllocatorExt for WithoutDealloc<A> {
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> Range<NonNull<T>> {
        A::prepare_slice_allocation(&mut self.0, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<Range<NonNull<T>>, AllocError> {
        A::try_prepare_slice_allocation(&mut self.0, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        A::allocate_prepared_slice(&mut self.0, ptr, len, cap)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        A::allocate_prepared_slice_rev(&mut self.0, ptr, len, cap)
    }
}
unsafe impl<A: MutBumpAllocatorExt> MutBumpAllocatorExt for WithoutShrink<A> {
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> Range<NonNull<T>> {
        A::prepare_slice_allocation(&mut self.0, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<Range<NonNull<T>>, AllocError> {
        A::try_prepare_slice_allocation(&mut self.0, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        A::allocate_prepared_slice(&mut self.0, ptr, len, cap)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        A::allocate_prepared_slice_rev(&mut self.0, ptr, len, cap)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocatorExt
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> Range<NonNull<T>> {
        BumpScope::prepare_slice_allocation(self.as_mut_scope(), len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<Range<NonNull<T>>, AllocError> {
        BumpScope::try_prepare_slice_allocation(self.as_mut_scope(), len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        BumpScope::use_prepared_slice_allocation(self.as_mut_scope(), ptr, len, cap)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        BumpScope::use_prepared_slice_allocation_rev(self.as_mut_scope(), ptr, len, cap)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocatorExt
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> Range<NonNull<T>> {
        panic_on_error(BumpScope::prepare_allocation_range::<_, T>(self, len))
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<Range<NonNull<T>>, AllocError> {
        BumpScope::prepare_allocation_range::<_, T>(self, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        BumpScope::use_prepared_slice_allocation(self, ptr, len, cap)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        BumpScope::use_prepared_slice_allocation_rev(self, ptr, len, cap)
    }
}
