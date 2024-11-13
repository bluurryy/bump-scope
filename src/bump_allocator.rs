use allocator_api2::alloc::Allocator;

use crate::{BaseAllocator, Bump, BumpScope, MinimumAlignment, SupportedMinimumAlignment};

/// An allocator that allows `grow(_zeroed)`, `shrink` and `deallocate` calls with pointers that were not allocated by this allocator.
///
/// This trait is used for [`BumpBox::into_box`][into_box] to allow safely converting a `BumpBox` into a `Box`.
///
/// # Safety
///
/// This trait must only be implemented when
/// - `grow(_zeroed)`, `shrink` and `deallocate` can be called with a pointer that was not allocated by this Allocator
/// - `deallocate` can be called with any pointer or alignment when the size is `0`
///
/// [into_box]: crate::BumpBox::into_box
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

/// An allocator that makes allocations with a lifetime of `'a`.
///
/// # Safety
///
/// This trait must only be implemented when allocations live for `'a`.
/// In other words this function must be sound:
///
/// ```
/// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
/// # #![allow(dead_code)]
/// use bump_scope::BumpAllocatorScope;
/// use core::alloc::Layout;
///
/// fn allocate_zeroed_bytes<'a>(allocator: impl BumpAllocatorScope<'a>, len: usize) -> &'a [u8] {
///     let layout = Layout::array::<u8>(len).unwrap();
///     let ptr = allocator.allocate_zeroed(layout).unwrap();
///     unsafe { ptr.as_ref() }
/// }
/// ```
pub unsafe trait BumpAllocatorScope<'a>: BumpAllocator {}

unsafe impl<'a, A: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for &A {}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for &'a Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}
