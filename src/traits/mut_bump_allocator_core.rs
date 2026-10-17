use core::alloc::Allocator;

use crate::{
    Bump, BumpScope, WithoutDealloc, WithoutShrink,
    config::BumpConfig,
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

unsafe impl<A, const C: BumpConfig> MutBumpAllocatorCore for Bump<A, C> where A: Allocator {}

unsafe impl<A, const C: BumpConfig> MutBumpAllocatorCore for BumpScope<'_, A, C> where A: Allocator {}
