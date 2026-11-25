use core::{
    alloc::Allocator,
    ops::{Deref, DerefMut},
};

use crate::{
    BumpScope,
    chunk_types::Chunk,
    config::{BumpConfig, config},
};

/// Returned from [`BumpScope::claim`].
#[must_use]
pub struct BumpClaimGuard<'a, const C: BumpConfig, A: Allocator> {
    pub(crate) original: &'a BumpScope<C, A>,
    pub(crate) claimant: BumpScope<C, A>,
}

impl<const C: BumpConfig, A: Allocator> Drop for BumpClaimGuard<'_, C, A> {
    #[inline(always)]
    fn drop(&mut self) {
        self.original.set_chunk(self.claimant.chunk());
    }
}

impl<const C: BumpConfig, A: Allocator> Deref for BumpClaimGuard<'_, C, A> {
    type Target = BumpScope<C, A>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.claimant
    }
}

impl<const C: BumpConfig, A: Allocator> DerefMut for BumpClaimGuard<'_, C, A> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.claimant
    }
}

impl<'a, const C: BumpConfig, A: Allocator> BumpClaimGuard<'a, C, A> {
    #[inline(always)]
    pub(crate) fn new(original: &'a BumpScope<C, A>) -> Self {
        assert!(config!(claimable), "`claim` is only available when `claimable` is enabled");

        #[cold]
        #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
        fn already_claimed() {
            panic!("bump allocator is already claimed");
        }

        if original.chunk().is_claimed() {
            already_claimed();
        }

        let claimant = BumpScope {
            chunk: original.chunk.clone(),
            allocator: original.allocator.create_ref(),
        };

        original.set_chunk(Chunk::CLAIMED);

        Self { original, claimant }
    }
}
