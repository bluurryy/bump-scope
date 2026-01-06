use crate::{
    BaseAllocator, Bump, BumpScope, WithoutDealloc, WithoutShrink,
    settings::BumpAllocatorSettings,
    traits::{BumpAllocatorCore, assert_implements},
};

/// A mutable bump allocator.
///
/// # Safety
///
/// Implementors must have exclusive access to to the bump allocator.
pub unsafe trait MutBumpAllocatorCore: BumpAllocatorCore {}

assert_implements! {
    [MutBumpAllocatorCore + ?Sized]

    Bump
    &mut Bump

    BumpScope
    &mut BumpScope

    dyn MutBumpAllocatorCore
    &mut dyn MutBumpAllocatorCore
}

unsafe impl<A: MutBumpAllocatorCore + ?Sized> MutBumpAllocatorCore for &mut A {}

unsafe impl<A: MutBumpAllocatorCore> MutBumpAllocatorCore for WithoutDealloc<A> {}
unsafe impl<A: MutBumpAllocatorCore> MutBumpAllocatorCore for WithoutShrink<A> {}

unsafe impl<A, S> MutBumpAllocatorCore for Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
}

unsafe impl<A, S> MutBumpAllocatorCore for BumpScope<'_, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
}
