use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    BumpScope,
    alloc::Allocator,
    settings::{BumpAllocatorSettings, BumpSettings},
};

// For docs.
#[allow(unused_imports)]
use crate::traits::*;

/// Returned from [`BumpAllocatorScope::claim`].
pub struct BumpClaimGuard<'b, 'a, A, S = BumpSettings>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    pub(crate) original: &'b BumpScope<'a, A, S>,
    pub(crate) claimant: BumpScope<'a, A, S>,
}

impl<'b, 'a, A, S> BumpClaimGuard<'b, 'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    pub(crate) fn new(original: &'b BumpScope<'a, A, S>) -> Self {
        let claimed = original.raw.claim();

        let claimant = BumpScope {
            raw: claimed,
            marker: PhantomData,
        };

        Self { original, claimant }
    }
}

impl<'a, A, S> Deref for BumpClaimGuard<'_, 'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    type Target = BumpScope<'a, A, S>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.claimant
    }
}

impl<A, S> DerefMut for BumpClaimGuard<'_, '_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.claimant
    }
}

impl<A, S> Drop for BumpClaimGuard<'_, '_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.original.raw.reclaim(&self.claimant.raw);
    }
}
