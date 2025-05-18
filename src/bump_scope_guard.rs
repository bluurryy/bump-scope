use core::{fmt::Debug, marker::PhantomData, num::NonZeroUsize, ptr::NonNull};

use crate::{
    chunk_header::ChunkHeader, polyfill::nonnull, BaseAllocator, Bump, BumpScope, MinimumAlignment, RawChunk, Stats,
    SupportedMinimumAlignment,
};

/// This is returned from [`checkpoint`](Bump::checkpoint) and used for [`reset_to`](Bump::reset_to).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checkpoint {
    pub(crate) chunk: NonNull<ChunkHeader>,
    pub(crate) address: NonZeroUsize,
}

impl Checkpoint {
    pub(crate) fn new<const UP: bool, A>(chunk: RawChunk<UP, A>) -> Self {
        let address = nonnull::addr(chunk.pos());
        let chunk = chunk.header_ptr().cast();
        Checkpoint { chunk, address }
    }

    pub(crate) unsafe fn reset_within_chunk(self) {
        let ptr = nonnull::with_addr(self.chunk.cast::<u8>(), self.address);
        self.chunk.as_ref().pos.set(ptr);
    }
}

/// Returned from [`BumpScope::scope_guard`].
pub struct BumpScopeGuard<'a, A, const MIN_ALIGN: usize = 1, const UP: bool = true>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    pub(crate) chunk: RawChunk<UP, A>,
    address: usize,
    marker: PhantomData<&'a mut ()>,
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Debug for BumpScopeGuard<'_, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.stats().debug_format("BumpScopeGuard", f)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Drop for BumpScopeGuard<'_, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScopeGuard<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    #[inline(always)]
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub(crate) fn new<'parent>(bump: &'a mut BumpScope<'parent, A, MIN_ALIGN, UP>) -> Self {
        unsafe { Self::new_unchecked(bump.chunk.get()) }
    }

    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<UP, A>) -> Self {
        Self {
            chunk,
            address: nonnull::addr(chunk.pos()).get(),
            marker: PhantomData,
        }
    }

    /// Returns a new `BumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> BumpScope<A, MIN_ALIGN, UP> {
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
    pub fn stats(&self) -> Stats<'a, A, UP, true> {
        let header = self.chunk.header_ptr().cast();
        // SAFETY: `header` points to a valid chunk header which is guaranteed allocated
        unsafe { Stats::from_header_unchecked(header) }
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        unsafe { self.chunk.allocator().as_ref() }
    }
}

/// Returned from [`Bump::scope_guard`].
///
/// This fulfills the same purpose as [`BumpScopeGuard`], but it does not need to store
/// the address which the bump pointer needs to be reset to. It simply resets the bump pointer to the very start.
///
/// It is allowed to do so because it takes a `&mut Bump` to create this guard. This means that no
/// allocations can be live when the guard is created.
pub struct BumpScopeGuardRoot<'b, A, const MIN_ALIGN: usize = 1, const UP: bool = true>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    pub(crate) chunk: RawChunk<UP, A>,
    marker: PhantomData<&'b mut ()>,
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Debug for BumpScopeGuardRoot<'_, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.stats().debug_format("BumpScopeGuardRoot", f)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Drop for BumpScopeGuardRoot<'_, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScopeGuardRoot<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    #[inline(always)]
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub(crate) fn new(bump: &'a mut Bump<A, MIN_ALIGN, UP>) -> Self {
        unsafe { Self::new_unchecked(bump.chunk.get()) }
    }

    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<UP, A>) -> Self {
        Self {
            chunk,
            marker: PhantomData,
        }
    }

    /// Returns a new `BumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> BumpScope<A, MIN_ALIGN, UP> {
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
    pub fn stats(&self) -> Stats<A, UP> {
        let header = self.chunk.header_ptr().cast();
        // SAFETY: `header` points to a valid chunk header which is guaranteed allocated
        unsafe { Stats::from_header_unchecked(header) }
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        unsafe { self.chunk.allocator().as_ref() }
    }
}
