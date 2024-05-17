use core::{fmt::Debug, marker::PhantomData, num::NonZeroUsize, ptr::NonNull};

use allocator_api2::alloc::Allocator;

use crate::{
    chunk_header::ChunkHeader, polyfill::nonnull, Bump, BumpScope, Chunk, GuaranteedAllocatedStats, MinimumAlignment,
    RawChunk, Stats, SupportedMinimumAlignment,
};

/// This is returned from [`checkpoint`](Bump::checkpoint) and used for [`reset_to`](Bump::reset_to).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checkpoint {
    pub(crate) chunk: NonNull<ChunkHeader<()>>,
    pub(crate) address: NonZeroUsize,
}

impl Checkpoint {
    pub(crate) fn new<const UP: bool, A>(chunk: RawChunk<UP, A>) -> Self {
        let address = nonnull::addr(chunk.pos());
        let chunk = chunk.header_ptr().cast();
        Checkpoint { chunk, address }
    }

    pub(crate) unsafe fn reset_within_chunk(mut self) {
        let ptr = nonnull::with_addr(self.chunk.cast::<u8>(), self.address);
        self.chunk.as_mut().pos.set(ptr);
    }
}

/// Returned from [`BumpScope::scope_guard`].
pub struct ScopeGuard<'a, A, const MIN_ALIGN: usize = 1, const UP: bool = true>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
    pub(crate) chunk: RawChunk<UP, A>,
    address: usize,
    marker: PhantomData<&'a mut ()>,
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Debug for ScopeGuard<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.stats().debug_format("ScopeGuard", f)
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Drop for ScopeGuard<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> ScopeGuard<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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

    #[doc = crate::doc_fn_scope!()]
    #[inline(always)]
    pub fn scope(&mut self) -> BumpScope<A, MIN_ALIGN, UP> {
        unsafe { BumpScope::new_unchecked(self.chunk) }
    }

    #[doc = crate::doc_fn_reset!()]
    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe {
            self.chunk.reset_to(self.address);
        }
    }

    #[doc = crate::doc_fn_stats!(Stats)]
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<UP> {
        Stats {
            current: Some(unsafe { Chunk::from_raw(self.chunk) }),
        }
    }

    #[doc = crate::doc_fn_stats!(GuaranteedAllocatedStats)]
    #[must_use]
    #[inline(always)]
    pub fn guaranteed_allocated_stats(&self) -> GuaranteedAllocatedStats<UP> {
        GuaranteedAllocatedStats {
            current: unsafe { Chunk::from_raw(self.chunk) },
        }
    }

    #[doc = crate::doc_fn_allocator!()]
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        unsafe { self.chunk.allocator().as_ref() }
    }
}

/// Returned from [`Bump::scope_guard`].
///
/// This fulfills the same purpose as [`ScopeGuard`], but it does not need to store
/// the address which the bump pointer needs to be reset to. It simply resets the bump pointer to the very start.
pub struct ScopeGuardRoot<'b, A, const MIN_ALIGN: usize = 1, const UP: bool = true>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
    pub(crate) chunk: RawChunk<UP, A>,
    marker: PhantomData<&'b mut ()>,
}

impl<'b, A, const MIN_ALIGN: usize, const UP: bool> Debug for ScopeGuardRoot<'b, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.guaranteed_allocated_stats().debug_format("ScopeGuardRoot", f)
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Drop for ScopeGuardRoot<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> ScopeGuardRoot<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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

    #[doc = crate::doc_fn_scope!()]
    #[inline(always)]
    pub fn scope(&mut self) -> BumpScope<A, MIN_ALIGN, UP> {
        unsafe { BumpScope::new_unchecked(self.chunk) }
    }

    #[doc = crate::doc_fn_reset!()]
    #[inline(always)]
    pub fn reset(&mut self) {
        self.chunk.reset();
    }

    #[doc = crate::doc_fn_stats!(Stats)]
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<UP> {
        Stats {
            current: Some(unsafe { Chunk::from_raw(self.chunk) }),
        }
    }

    #[doc = crate::doc_fn_stats!(GuaranteedAllocatedStats)]
    #[must_use]
    #[inline(always)]
    pub fn guaranteed_allocated_stats(&self) -> GuaranteedAllocatedStats<UP> {
        GuaranteedAllocatedStats {
            current: unsafe { Chunk::from_raw(self.chunk) },
        }
    }

    #[doc = crate::doc_fn_allocator!()]
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        unsafe { self.chunk.allocator().as_ref() }
    }
}
