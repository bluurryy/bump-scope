use core::{
    alloc::Allocator,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use alloc::alloc::Global;

use crate::{
    BumpScope,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG},
};

// For docs.
#[allow(unused_imports)]
use crate::traits::*;

/// Returned from [`BumpScope::claim`].
#[must_use]
pub struct BumpClaimGuard<'b, 'a, const C: BumpConfig = DEFAULT_BUMP_CONFIG, A = Global>
where
    A: Allocator,
{
    pub(crate) original: &'b BumpScope<'a, C, A>,
    pub(crate) claimant: BumpScope<'a, C, A>,
}

impl<'b, 'a, const C: BumpConfig, A> BumpClaimGuard<'b, 'a, C, A>
where
    A: Allocator,
{
    #[inline(always)]
    pub(crate) fn new(original: &'b BumpScope<'a, C, A>) -> Self {
        let claimed = original.raw.claim();

        let claimant = BumpScope {
            raw: claimed,
            marker: PhantomData,
        };

        Self { original, claimant }
    }
}

impl<'a, const C: BumpConfig, A> Deref for BumpClaimGuard<'_, 'a, C, A>
where
    A: Allocator,
{
    type Target = BumpScope<'a, C, A>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.claimant
    }
}

impl<const C: BumpConfig, A> DerefMut for BumpClaimGuard<'_, '_, C, A>
where
    A: Allocator,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.claimant
    }
}

impl<const C: BumpConfig, A> Drop for BumpClaimGuard<'_, '_, C, A>
where
    A: Allocator,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.original.raw.reclaim(&self.claimant.raw);
    }
}
