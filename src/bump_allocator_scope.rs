use crate::{
    BaseAllocator, Bump, BumpAllocator, BumpScope, MinimumAlignment, SupportedMinimumAlignment, WithoutDealloc,
    WithoutShrink,
};

/// A bump allocator scope.
///
/// This is a [`BumpAllocator`] which can make allocations that outlive itself.
/// Specifically, its allocations live for the lifetime `'a`.
///
/// # Safety
///
/// This trait must only be implemented when allocations live for `'a`.
/// For example this function must be sound:
///
/// ```
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
pub unsafe trait BumpAllocatorScope<'a>: BumpAllocator {
    // TODO: implement `stats` that live for `'a`?
}

unsafe impl<'a, A: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for &A {}
unsafe impl<'a, A: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for WithoutDealloc<A> {}
unsafe impl<'a, A: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for WithoutShrink<A> {}

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

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for &mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for &'a mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}
