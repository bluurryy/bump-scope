use allocator_api2::alloc::Allocator;

use crate::{BaseAllocator, Bump, BumpScope, MinimumAlignment, SupportedMinimumAlignment};

/// An allocator that allows `grow(_zeroed)`, `shrink` and `deallocate` calls with pointers that were not allocated by this allocator.
///
/// This trait is used for [`BumpBox::into_box`](BumpBox::into_box) to allow safely converting a `BumpBox` into a `Box`.
///
/// # Safety
/// - `grow(_zeroed)`, `shrink` and `deallocate` must be ok to be called with a pointer that was not allocated by this Allocator
pub unsafe trait BumpAllocator: Allocator {}

unsafe impl<A: BumpAllocator> BumpAllocator for &A {}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}
