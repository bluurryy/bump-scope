use core::ops::{Deref, DerefMut};

use crate::{
    BumpScope,
    alloc::Allocator,
    chunk::RawChunk,
    settings::{BumpAllocatorSettings, BumpSettings, True},
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
    pub(crate) claimed: BumpScope<'a, A, S>,
}

impl<'b, 'a, A, S> BumpClaimGuard<'b, 'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    pub(crate) fn new(original: &'b BumpScope<'a, A, S>) -> Self
    where
        S: BumpAllocatorSettings<Claimable = True>,
    {
        let chunk = original.chunk.replace(RawChunk::<A, S>::CLAIMED);
        let claimed = unsafe { BumpScope::from_raw(chunk.header().cast()) };
        Self { original, claimed }
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
        &self.claimed
    }
}

impl<A, S> DerefMut for BumpClaimGuard<'_, '_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.claimed
    }
}

impl<A, S> Drop for BumpClaimGuard<'_, '_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.original.chunk.set(self.claimed.chunk.get());
    }
}
