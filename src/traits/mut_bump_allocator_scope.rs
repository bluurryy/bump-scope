use crate::{
    BaseAllocator, Bump, BumpAllocatorScope, BumpScope, MinimumAlignment, MutBumpAllocator, SupportedMinimumAlignment,
    WithoutDealloc, WithoutShrink,
};

/// A trait as a shorthand for <code>[MutBumpAllocator] + [BumpAllocatorScope]<'a></code>
pub trait MutBumpAllocatorScope<'a>: MutBumpAllocator + BumpAllocatorScope<'a> {}

impl<'a, B: MutBumpAllocatorScope<'a> + ?Sized> MutBumpAllocatorScope<'a> for &mut B {}

impl<'a, B: MutBumpAllocatorScope<'a>> MutBumpAllocatorScope<'a> for WithoutDealloc<B> {}
impl<'a, B: MutBumpAllocatorScope<'a>> MutBumpAllocatorScope<'a> for WithoutShrink<B> {}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocatorScope<'a>
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocatorScope<'a>
    for &'a mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}
