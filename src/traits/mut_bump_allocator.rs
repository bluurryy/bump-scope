use crate::{
    BaseAllocator, Bump, BumpAllocator, BumpScope, MinimumAlignment, SupportedMinimumAlignment, WithoutDealloc,
    WithoutShrink, traits::assert_implements,
};

/// A marker trait for [`BumpAllocator`]s who have exclusive access to allocation.
///
/// # Safety
///
/// Implementors must have exclusive access to to the bump allocator.
pub unsafe trait MutBumpAllocator: BumpAllocator {}

assert_implements! {
    [MutBumpAllocator + ?Sized]

    Bump
    &mut Bump

    BumpScope
    &mut BumpScope

    dyn MutBumpAllocator
    &mut dyn MutBumpAllocator
}

unsafe impl<A: MutBumpAllocator + ?Sized> MutBumpAllocator for &mut A {}

unsafe impl<A: MutBumpAllocator> MutBumpAllocator for WithoutDealloc<A> {}
unsafe impl<A: MutBumpAllocator> MutBumpAllocator for WithoutShrink<A> {}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}
