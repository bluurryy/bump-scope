use core::{fmt::Debug, marker::PhantomData, num::NonZeroUsize, ptr::NonNull};

use crate::{
    Bump, MinimumAlignment, MutBumpScope, RawChunk, SupportedMinimumAlignment,
    alloc::Allocator,
    chunk_header::ChunkHeader,
    stats::{AnyStats, Stats},
};

/// This is returned from [`checkpoint`](Bump::checkpoint) and used for [`reset_to`](Bump::reset_to).
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

/// Returned from [`MutBumpScope::scope_guard`].
pub struct MutBumpScopeGuard<'a, A, const MIN_ALIGN: usize = 1, const UP: bool = true, const DEALLOCATES: bool = true>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) chunk: RawChunk<A, UP, true>,
    address: usize,
    marker: PhantomData<&'a mut ()>,
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const DEALLOCATES: bool> Debug
    for MutBumpScopeGuard<'_, A, MIN_ALIGN, UP, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        AnyStats::from(self.stats()).debug_format("MutBumpScopeGuard", f)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const DEALLOCATES: bool> Drop
    for MutBumpScopeGuard<'_, A, MIN_ALIGN, UP, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const DEALLOCATES: bool>
    MutBumpScopeGuard<'a, A, MIN_ALIGN, UP, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    pub(crate) fn new(bump: &'a mut MutBumpScope<'_, A, MIN_ALIGN, UP, true, DEALLOCATES>) -> Self {
        unsafe { Self::new_unchecked(bump.chunk.get()) }
    }

    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<A, UP, true>) -> Self {
        Self {
            chunk,
            address: chunk.pos().addr().get(),
            marker: PhantomData,
        }
    }

    /// Returns a new `MutBumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> MutBumpScope<'_, A, MIN_ALIGN, UP, true, DEALLOCATES> {
        unsafe { MutBumpScope::new_unchecked(self.chunk) }
    }

    /// Frees the memory taken up by allocations made since creation of this bump scope guard.
    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe {
            self.chunk.set_pos_addr(self.address);
        }
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, A, UP> {
        self.chunk.stats()
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &'a A {
        self.stats().current_chunk().allocator()
    }
}

/// Returned from [`Bump::scope_guard`].
///
/// This fulfills the same purpose as [`MutBumpScopeGuard`], but it does not need to store
/// the address which the bump pointer needs to be reset to. It simply resets the bump pointer to the very start.
///
/// It is allowed to do so because it takes a `&mut Bump` to create this guard. This means that no
/// allocations can be live when the guard is created.
pub struct MutBumpScopeGuardRoot<'b, A, const MIN_ALIGN: usize = 1, const UP: bool = true, const DEALLOCATES: bool = true>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator,
{
    pub(crate) chunk: RawChunk<A, UP, true>,
    marker: PhantomData<&'b mut ()>,
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const DEALLOCATES: bool> Debug
    for MutBumpScopeGuardRoot<'_, A, MIN_ALIGN, UP, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        AnyStats::from(self.stats()).debug_format("MutBumpScopeGuardRoot", f)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const DEALLOCATES: bool> Drop
    for MutBumpScopeGuardRoot<'_, A, MIN_ALIGN, UP, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const DEALLOCATES: bool>
    MutBumpScopeGuardRoot<'a, A, MIN_ALIGN, UP, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator,
{
    #[inline(always)]
    pub(crate) fn new(bump: &'a mut Bump<A, MIN_ALIGN, UP, true, DEALLOCATES>) -> Self {
        unsafe { Self::new_unchecked(bump.chunk.get()) }
    }

    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<A, UP, true>) -> Self {
        Self {
            chunk,
            marker: PhantomData,
        }
    }

    /// Returns a new `MutBumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> MutBumpScope<'_, A, MIN_ALIGN, UP, true, DEALLOCATES> {
        unsafe { MutBumpScope::new_unchecked(self.chunk) }
    }

    /// Frees the memory taken up by allocations made since creation of this bump scope guard.
    #[inline(always)]
    pub fn reset(&mut self) {
        self.chunk.reset();
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, A, UP> {
        self.chunk.stats()
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &'a A {
        self.stats().current_chunk().allocator()
    }
}
