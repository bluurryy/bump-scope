use core::{fmt::Debug, num::NonZeroUsize, ptr::NonNull};

use crate::{
    BumpScope, MinimumAlignment, RawChunk, SupportedMinimumAlignment,
    chunk_header::ChunkHeader,
    stats::{AnyStats, Stats},
};

/// This is returned from [`checkpoint`] and used for [`reset_to`].
///
/// [`checkpoint`]: crate::Bump::checkpoint
/// [`reset_to`]: crate::Bump::reset_to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checkpoint {
    pub(crate) chunk: NonNull<ChunkHeader>,
    pub(crate) address: NonZeroUsize,
}

impl Checkpoint {
    pub(crate) fn new<A, const UP: bool, const GUARANTEED_ALLOCATED: bool>(
        chunk: RawChunk<A, UP, GUARANTEED_ALLOCATED>,
    ) -> Self {
        let address = chunk.pos().addr();
        let chunk = chunk.header().cast();
        Checkpoint { chunk, address }
    }

    pub(crate) unsafe fn reset_within_chunk(self) {
        unsafe {
            let ptr = self.chunk.cast::<u8>().with_addr(self.address);
            self.chunk.as_ref().pos.set(ptr);
        }
    }
}

/// Returned from [`BumpScope::scope_guard`].
pub struct BumpScopeGuard<'a, A, const MIN_ALIGN: usize = 1, const UP: bool = true, const DEALLOCATES: bool = true>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) parent: &'a BumpScope<'a, A, MIN_ALIGN, UP, true, DEALLOCATES>,
    checkpoint: Checkpoint,
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const DEALLOCATES: bool> Debug
    for BumpScopeGuard<'_, A, MIN_ALIGN, UP, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        AnyStats::from(self.stats()).debug_format("BumpScopeGuard", f)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const DEALLOCATES: bool> Drop
    for BumpScopeGuard<'_, A, MIN_ALIGN, UP, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const DEALLOCATES: bool>
    BumpScopeGuard<'a, A, MIN_ALIGN, UP, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    pub(crate) fn new(parent: &'a BumpScope<'_, A, MIN_ALIGN, UP, true, DEALLOCATES>) -> Self {
        let checkpoint = parent.checkpoint();
        parent.disable();
        Self { parent, checkpoint }
    }

    /// Returns a new `BumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> BumpScope<'_, A, MIN_ALIGN, UP, true, DEALLOCATES> {
        unsafe { BumpScope::new_unchecked(self.raw_chunk()) }
    }

    /// Frees the memory taken up by allocations made since creation of this bump scope guard.
    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe { self.parent.reset_to(self.checkpoint) };
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, A, UP> {
        self.raw_chunk().stats()
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> Option<&'a A> {
        self.stats().current_chunk().map(|c| c.allocator())
    }

    #[must_use]
    #[inline(always)]
    fn raw_chunk(&self) -> RawChunk<A, UP, true> {
        unsafe { RawChunk::from_header(self.checkpoint.chunk.cast()) }
    }
}
