use core::{
    alloc::Allocator,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    BumpScope,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG},
};

// For docs.
#[allow(unused_imports)]
use crate::traits::*;

/// Returned from [`BumpScope::claim`].
#[must_use]
pub struct BumpClaimGuard<'b, 'a, A, const C: BumpConfig = DEFAULT_BUMP_CONFIG>
where
    A: Allocator,
{
    pub(crate) original: &'b BumpScope<'a, A, C>,
    pub(crate) claimant: BumpScope<'a, A, C>,
}

impl<'b, 'a, A, const C: BumpConfig> BumpClaimGuard<'b, 'a, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    pub(crate) fn new(original: &'b BumpScope<'a, A, C>) -> Self {
        let claimed = original.raw.claim();

        let claimant = BumpScope {
            raw: claimed,
            marker: PhantomData,
        };

        Self { original, claimant }
    }
}

impl<'a, A, const C: BumpConfig> Deref for BumpClaimGuard<'_, 'a, A, C>
where
    A: Allocator,
{
    type Target = BumpScope<'a, A, C>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.claimant
    }
}

impl<A, const C: BumpConfig> DerefMut for BumpClaimGuard<'_, '_, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.claimant
    }
}

impl<A, const C: BumpConfig> Drop for BumpClaimGuard<'_, '_, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.original.raw.reclaim(&self.claimant.raw);
    }
}
