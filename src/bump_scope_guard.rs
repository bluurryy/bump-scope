use core::{alloc::Allocator, fmt::Debug, num::NonZeroUsize};

use alloc::alloc::Global;

use crate::{
    BumpScope,
    bump_imp::{AsBumpImp, BumpImpTrait as _},
    bump_scope::BumpScopeWithoutLifetime,
    chunk_types::{Chunk, ErasedChunk},
    config::{BumpConfig, DEFAULT_BUMP_CONFIG},
    util::transmute_mut,
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

    pub(crate) fn new<const C: BumpConfig>(chunk: Chunk<C>) -> Self {
        let address = chunk.pos().addr();
        let chunk = ErasedChunk::erase(chunk);
        Checkpoint { chunk, address }
    }
}

/// Returned from [`BumpScope::scope_guard`].
///
/// [`BumpScope::scope_guard`]: crate::BumpScope::scope_guard
#[must_use]
pub struct BumpScopeGuard<'a, const C: BumpConfig = DEFAULT_BUMP_CONFIG, A: Allocator = Global> {
    bump: &'a mut BumpScopeWithoutLifetime<C, A>,
    checkpoint: Checkpoint,
}

impl<const C: BumpConfig, A: Allocator> Debug for BumpScopeGuard<'_, C, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.bump.imp().stats().debug_format("BumpScopeGuard", f)
    }
}

impl<const C: BumpConfig, A: Allocator> Drop for BumpScopeGuard<'_, C, A> {
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, const C: BumpConfig, A: Allocator> BumpScopeGuard<'a, C, A> {
    #[inline(always)]
    pub(crate) fn new(bump: &'a mut BumpScopeWithoutLifetime<C, A>) -> Self {
        let checkpoint = bump.imp().checkpoint();
        Self { bump, checkpoint }
    }

    /// Returns a new `BumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> &mut BumpScope<'_, C, A> {
        unsafe { transmute_mut(self.bump) }
    }

    /// Frees the memory taken up by allocations made since creation of this bump scope guard.
    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe { self.bump.imp().reset_to(self.checkpoint) }
    }
}
