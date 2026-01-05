use crate::{
    BaseAllocator, Bump, BumpAllocator, BumpScope, WithoutDealloc, WithoutShrink, settings::BumpAllocatorSettings,
    traits::assert_implements,
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

unsafe impl<A, S> MutBumpAllocator for Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
}

unsafe impl<A, S> MutBumpAllocator for BumpScope<'_, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
}
