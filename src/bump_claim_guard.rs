use core::{
    alloc::Allocator,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use alloc::alloc::Global;

use crate::{
    BumpScope,
    chunk_types::Chunk,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG, config},
};

// For docs.
#[allow(unused_imports)]
use crate::traits::*;

/// Returned from [`BumpScope::claim`].
#[must_use]
pub struct BumpClaimGuard<'b, 'a, const C: BumpConfig = DEFAULT_BUMP_CONFIG, A: Allocator = Global> {
    pub(crate) original: &'b BumpScope<'a, C, A>,
    pub(crate) claimant: BumpScope<'a, C, A>,
}

impl<'b, 'a, const C: BumpConfig, A: Allocator> BumpClaimGuard<'b, 'a, C, A> {
    #[inline(always)]
    pub(crate) fn new(original: &'b BumpScope<'a, C, A>) -> Self {
        assert!(config!(claimable), "`claim` is only available when `claimable` is enabled");

        #[cold]
        #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
        fn already_claimed() {
            panic!("bump allocator is already claimed");
        }

        if original.chunk.get().is_claimed() {
            already_claimed();
        }

        let claimant = BumpScope {
            chunk: original.chunk.clone(),
            allocator_ptr: original.allocator_ptr.clone(),
            marker: PhantomData,
        };

        original.chunk.set(Chunk::CLAIMED);

        Self { original, claimant }
    }
}

impl<'a, const C: BumpConfig, A: Allocator> Deref for BumpClaimGuard<'_, 'a, C, A> {
    type Target = BumpScope<'a, C, A>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.claimant
    }
}

impl<const C: BumpConfig, A: Allocator> DerefMut for BumpClaimGuard<'_, '_, C, A> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.claimant
    }
}

impl<const C: BumpConfig, A: Allocator> Drop for BumpClaimGuard<'_, '_, C, A> {
    #[inline(always)]
    fn drop(&mut self) {
        self.original.chunk.set(self.claimant.chunk.get());
    }
}
