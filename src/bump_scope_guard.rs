use core::{alloc::Allocator, fmt::Debug, num::NonZeroUsize};

use crate::{
    BumpScope,
    chunk_types::r#abstract::{Chunk, ErasedChunk},
    config::{BumpConfig, DEFAULT_BUMP_CONFIG},
    polyfill::transmute_mut,
    raw_bump::RawBump,
    stats::AnyStats,
};

/// This is returned from [`checkpoint`] and used for [`reset_to`].
///
/// [`checkpoint`]: crate::traits::BumpAllocator::checkpoint
/// [`reset_to`]: crate::traits::BumpAllocator::reset_to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checkpoint {
    pub(crate) chunk: ErasedChunk,
    pub(crate) address: NonZeroUsize,
}

impl Checkpoint {
    pub(crate) const INVALID: Self = {
        Self {
            chunk: ErasedChunk::INVALID,
            address: NonZeroUsize::MAX,
        }
    };

    pub(crate) fn new<A, const C: BumpConfig>(chunk: Chunk<A, C>) -> Self {
        let address = chunk.pos().addr();
        let chunk = ErasedChunk::erase(chunk);
        Checkpoint { chunk, address }
    }
}

/// Returned from [`BumpScope::scope_guard`].
///
/// [`BumpScope::scope_guard`]: crate::BumpScope::scope_guard
#[must_use]
pub struct BumpScopeGuard<'a, A, const C: BumpConfig = DEFAULT_BUMP_CONFIG>
where
    A: Allocator,
{
    bump: &'a mut RawBump<A, C>,
    checkpoint: Checkpoint,
}

impl<A, const C: BumpConfig> Debug for BumpScopeGuard<'_, A, C>
where
    A: Allocator,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        AnyStats::from(self.bump.stats()).debug_format("BumpScopeGuard", f)
    }
}

impl<A, const C: BumpConfig> Drop for BumpScopeGuard<'_, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, const C: BumpConfig> BumpScopeGuard<'a, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    pub(crate) fn new(bump: &'a mut RawBump<A, C>) -> Self {
        let checkpoint = bump.checkpoint();
        Self { bump, checkpoint }
    }

    /// Returns a new `BumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> &mut BumpScope<'_, A, C> {
        unsafe { transmute_mut(self.bump) }
    }

    /// Frees the memory taken up by allocations made since creation of this bump scope guard.
    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe { self.bump.reset_to(self.checkpoint) }
    }
}
