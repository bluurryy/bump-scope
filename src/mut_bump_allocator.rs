use core::ptr::NonNull;

use crate::{
    alloc_reexport::alloc::AllocError, polyfill::nonnull, BaseAllocator, Bump, BumpAllocator, BumpScope, MinimumAlignment,
    SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// A [`BumpAllocator`] who has exclusive access to allocation.
#[allow(clippy::missing_safety_doc)] // TODO
pub unsafe trait MutBumpAllocator: BumpAllocator {
    /// Does not allocate, just returns a slice of `T` that are currently available.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[doc(hidden)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized;

    /// Does not allocate, just returns a slice of `T` that are currently available.
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    #[doc(hidden)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized;

    /// Allocate part of a valid free slice returned by `(try_)prepare_slice_allocation`.
    ///
    /// # Safety
    ///
    /// - `ptr + cap` must be a slice returned by `(try_)prepare_slice_allocation`. No allocation,
    ///   grow, shrink or deallocate must have been called since then.
    /// - `len` must be less than or equal to `cap`
    #[doc(hidden)]
    unsafe fn use_prepared_slice_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized;

    /// Does not allocate, just returns a slice of `T` that are currently available.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[doc(hidden)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation_rev<T>(&mut self, len: usize) -> (NonNull<T>, usize)
    where
        Self: Sized;

    /// Does not allocate, just returns a slice of `T` that are currently available.
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    #[doc(hidden)]
    fn try_prepare_slice_allocation_rev<T>(&mut self, len: usize) -> Result<(NonNull<T>, usize), AllocError>
    where
        Self: Sized;

    /// Allocate part of a valid free slice returned by `(try_)prepare_slice_allocation`.
    ///
    /// # Safety
    ///
    /// - `ptr + cap` must be a slice returned by `(try_)prepare_slice_allocation`. No allocation,
    ///   grow, shrink or deallocate must have been called since then.
    /// - `len` must be less than or equal to `cap`
    #[doc(hidden)]
    unsafe fn use_prepared_slice_allocation_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized;
}

unsafe impl<A: MutBumpAllocator> MutBumpAllocator for WithoutDealloc<A> {
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        A::prepare_slice_allocation(&mut self.0, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        A::try_prepare_slice_allocation(&mut self.0, len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        A::use_prepared_slice_allocation(&mut self.0, ptr, len, cap)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation_rev<T>(&mut self, len: usize) -> (NonNull<T>, usize)
    where
        Self: Sized,
    {
        A::prepare_slice_allocation_rev(&mut self.0, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation_rev<T>(&mut self, len: usize) -> Result<(NonNull<T>, usize), AllocError>
    where
        Self: Sized,
    {
        A::try_prepare_slice_allocation_rev(&mut self.0, len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        A::use_prepared_slice_allocation_rev(&mut self.0, ptr, len, cap)
    }
}

unsafe impl<A: MutBumpAllocator> MutBumpAllocator for WithoutShrink<A> {
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        A::prepare_slice_allocation(&mut self.0, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        A::try_prepare_slice_allocation(&mut self.0, len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        A::use_prepared_slice_allocation(&mut self.0, ptr, len, cap)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation_rev<T>(&mut self, len: usize) -> (NonNull<T>, usize)
    where
        Self: Sized,
    {
        A::prepare_slice_allocation_rev(&mut self.0, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation_rev<T>(&mut self, len: usize) -> Result<(NonNull<T>, usize), AllocError>
    where
        Self: Sized,
    {
        A::try_prepare_slice_allocation_rev(&mut self.0, len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        A::use_prepared_slice_allocation_rev(&mut self.0, ptr, len, cap)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        let (ptr, len) = panic_on_error(BumpScope::generic_prepare_slice_allocation(self, len));
        nonnull::slice_from_raw_parts(ptr, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        let (ptr, len) = BumpScope::generic_prepare_slice_allocation(self, len)?;
        Ok(nonnull::slice_from_raw_parts(ptr, len))
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        BumpScope::use_prepared_slice_allocation(self, ptr, len, cap)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation_rev<T>(&mut self, len: usize) -> (NonNull<T>, usize)
    where
        Self: Sized,
    {
        panic_on_error(BumpScope::generic_prepare_slice_allocation_rev(self, len))
    }

    #[inline(always)]
    fn try_prepare_slice_allocation_rev<T>(&mut self, len: usize) -> Result<(NonNull<T>, usize), AllocError>
    where
        Self: Sized,
    {
        BumpScope::generic_prepare_slice_allocation_rev(self, len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        BumpScope::use_prepared_slice_allocation_rev(self, ptr, len, cap)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        self.as_mut_scope().prepare_slice_allocation(len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        self.as_mut_scope().try_prepare_slice_allocation(len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        self.as_mut_scope().use_prepared_slice_allocation(ptr, len, cap)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation_rev<T>(&mut self, len: usize) -> (NonNull<T>, usize)
    where
        Self: Sized,
    {
        self.as_mut_scope().prepare_slice_allocation_rev(len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation_rev<T>(&mut self, len: usize) -> Result<(NonNull<T>, usize), AllocError>
    where
        Self: Sized,
    {
        self.as_mut_scope().try_prepare_slice_allocation_rev(len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        self.as_mut_scope().use_prepared_slice_allocation_rev(ptr, len, cap)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocator
    for &mut BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        BumpScope::prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        BumpScope::try_prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        BumpScope::use_prepared_slice_allocation(self, ptr, len, cap)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation_rev<T>(&mut self, len: usize) -> (NonNull<T>, usize)
    where
        Self: Sized,
    {
        BumpScope::prepare_slice_allocation_rev(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation_rev<T>(&mut self, len: usize) -> Result<(NonNull<T>, usize), AllocError>
    where
        Self: Sized,
    {
        BumpScope::try_prepare_slice_allocation_rev(self, len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        BumpScope::use_prepared_slice_allocation_rev(self, ptr, len, cap)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocator
    for &mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        Bump::prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        Bump::try_prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        Bump::use_prepared_slice_allocation(self, ptr, len, cap)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation_rev<T>(&mut self, len: usize) -> (NonNull<T>, usize)
    where
        Self: Sized,
    {
        Bump::prepare_slice_allocation_rev(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation_rev<T>(&mut self, len: usize) -> Result<(NonNull<T>, usize), AllocError>
    where
        Self: Sized,
    {
        Bump::try_prepare_slice_allocation_rev(self, len)
    }

    #[inline(always)]
    unsafe fn use_prepared_slice_allocation_rev<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        Bump::use_prepared_slice_allocation_rev(self, ptr, len, cap)
    }
}
