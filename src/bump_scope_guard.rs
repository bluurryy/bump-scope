use core::{fmt::Debug, num::NonZeroUsize, ptr::NonNull};

use crate::{
    BumpScope,
    alloc::Allocator,
    chunk::ChunkHeader,
    polyfill::transmute_mut,
    raw_bump::{RawBump, RawChunk},
    settings::{BumpAllocatorSettings, BumpSettings},
    stats::AnyStats,
};

/// This is returned from [`checkpoint`] and used for [`reset_to`].
///
/// [`checkpoint`]: crate::traits::BumpAllocatorCore::checkpoint
/// [`reset_to`]: crate::traits::BumpAllocatorCore::reset_to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checkpoint {
    pub(crate) chunk: NonNull<ChunkHeader>,
    pub(crate) address: NonZeroUsize,
}

impl Checkpoint {
    pub(crate) fn new<S: BumpAllocatorSettings>(chunk: RawChunk<S>) -> Self {
        let address = chunk.pos().addr();
        let chunk = chunk.header.cast();
        Checkpoint { chunk, address }
    }

    pub(crate) unsafe fn reset_within_chunk(self) {
        unsafe {
            let ptr = self.chunk.cast::<u8>().with_addr(self.address);
            self.chunk.as_ref().pos.set(ptr);
        }
    }
}

/// Returned from [`BumpAllocator::scope_guard`].
///
/// [`BumpAllocator::scope_guard`]: crate::traits::BumpAllocator::scope_guard
pub struct BumpScopeGuard<'a, A, S = BumpSettings>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    bump: &'a mut RawBump<A, S>,
    checkpoint: Checkpoint,
}

impl<A, S> Debug for BumpScopeGuard<'_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        AnyStats::from(self.bump.stats()).debug_format("BumpScopeGuard", f)
    }
}

impl<A, S> Drop for BumpScopeGuard<'_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, S> BumpScopeGuard<'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    pub(crate) fn new(bump: &'a mut RawBump<A, S>) -> Self {
        let checkpoint = bump.checkpoint();
        Self { bump, checkpoint }
    }

    /// Returns a new `BumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> &mut BumpScope<'_, A, S> {
        unsafe { transmute_mut(self.bump) }
    }

    /// Frees the memory taken up by allocations made since creation of this bump scope guard.
    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe { self.bump.reset_to(self.checkpoint) }
    }
}
