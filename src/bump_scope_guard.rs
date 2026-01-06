use core::{fmt::Debug, marker::PhantomData, num::NonZeroUsize, ptr::NonNull};

use crate::{
    Bump, BumpScope, RawChunk,
    alloc::Allocator,
    chunk_header::ChunkHeader,
    settings::{BumpAllocatorSettings, BumpSettings, True},
    stats::{AnyStats, Stats},
};

/// This is returned from [`checkpoint`](Bump::checkpoint) and used for [`reset_to`](Bump::reset_to).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checkpoint {
    pub(crate) chunk: NonNull<ChunkHeader>,
    pub(crate) address: NonZeroUsize,
}

impl Checkpoint {
    pub(crate) fn new<A, S>(chunk: RawChunk<A, S>) -> Self
    where
        S: BumpAllocatorSettings,
    {
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
pub struct BumpScopeGuard<'a, A, S = BumpSettings>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    pub(crate) chunk: RawChunk<A, S>,
    address: usize,
    marker: PhantomData<&'a mut ()>,
}

impl<A, S> Debug for BumpScopeGuard<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        AnyStats::from(self.stats()).debug_format("BumpScopeGuard", f)
    }
}

impl<A, S> Drop for BumpScopeGuard<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, S> BumpScopeGuard<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    #[inline(always)]
    pub(crate) fn new(bump: &'a mut BumpScope<'_, A, S>) -> Self {
        unsafe { Self::new_unchecked(bump.chunk.get()) }
    }

    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<A, S>) -> Self {
        Self {
            chunk,
            address: chunk.pos().addr().get(),
            marker: PhantomData,
        }
    }

    /// Returns a new `BumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> BumpScope<'_, A, S> {
        unsafe { BumpScope::new_unchecked(self.chunk) }
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
    pub fn stats(&self) -> Stats<'a, A, S> {
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
/// This fulfills the same purpose as [`BumpScopeGuard`], but it does not need to store
/// the address which the bump pointer needs to be reset to. It simply resets the bump pointer to the very start.
///
/// It is allowed to do so because it takes a `&mut Bump` to create this guard. This means that no
/// allocations can be live when the guard is created.
pub struct BumpScopeGuardRoot<'b, A, S = BumpSettings>
where
    A: Allocator,
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    pub(crate) chunk: RawChunk<A, S>,
    marker: PhantomData<&'b mut ()>,
}

impl<A, S> Debug for BumpScopeGuardRoot<'_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        AnyStats::from(self.stats()).debug_format("BumpScopeGuardRoot", f)
    }
}

impl<A, S> Drop for BumpScopeGuardRoot<'_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, S> BumpScopeGuardRoot<'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    #[inline(always)]
    pub(crate) fn new(bump: &'a mut Bump<A, S>) -> Self {
        unsafe { Self::new_unchecked(bump.chunk.get()) }
    }

    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<A, S>) -> Self {
        Self {
            chunk,
            marker: PhantomData,
        }
    }

    /// Returns a new `BumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> BumpScope<'_, A, S> {
        unsafe { BumpScope::new_unchecked(self.chunk) }
    }

    /// Frees the memory taken up by allocations made since creation of this bump scope guard.
    #[inline(always)]
    pub fn reset(&mut self) {
        self.chunk.reset();
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, A, S> {
        self.chunk.stats()
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &'a A {
        self.stats().current_chunk().allocator()
    }
}
