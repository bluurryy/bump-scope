use core::{fmt::Debug, marker::PhantomData, num::NonZeroUsize, ptr::NonNull};

use allocator_api2::alloc::{Allocator, Global};

use crate::{
    chunk_header::ChunkHeader, polyfill::nonnull, Bump, BumpScope, Chunk, MinimumAlignment, RawChunk, Stats,
    SupportedMinimumAlignment,
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
        unsafe {
            let ptr = nonnull::with_addr(self.chunk.cast::<u8>(), self.address);
            unsafe { self.chunk.as_mut().pos.set(ptr) };
        }
    }
}

/// Returned from [`BumpScope::scope_guard`].
pub struct BumpScopeGuard<'a, const MIN_ALIGN: usize = 1, const UP: bool = true, A: Allocator + Clone = Global>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) chunk: RawChunk<UP, A>,
    address: usize,
    marker: PhantomData<&'a mut ()>,
}

impl<'a, const MIN_ALIGN: usize, const UP: bool, A: Allocator + Clone> Debug for BumpScopeGuard<'a, MIN_ALIGN, UP, A>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.stats().debug_format("BumpScopeGuard", f)
    }
}

impl<'a, const MIN_ALIGN: usize, const UP: bool, A: Allocator + Clone> Drop for BumpScopeGuard<'a, MIN_ALIGN, UP, A>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, const MIN_ALIGN: usize, const UP: bool, A: Allocator + Clone> BumpScopeGuard<'a, MIN_ALIGN, UP, A>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub(crate) fn new<'parent>(bump: &'a mut BumpScope<'parent, MIN_ALIGN, UP, A>) -> Self {
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
    pub fn scope(&mut self) -> BumpScope<MIN_ALIGN, UP, A> {
        unsafe { BumpScope::new_unchecked(self.chunk) }
    }

    #[doc = crate::doc_fn_reset!()]
    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe {
            self.chunk.reset_to(self.address);
        }
    }

    #[doc = crate::doc_fn_stats!()]
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<UP> {
        Stats {
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
/// This fulfills the same purpose as [`BumpScopeGuard`], but it does not need to store
/// the address which the bump pointer needs to be reset to. It simply resets the bump pointer to the very start.
pub struct BumpScopeGuardRoot<'b, const MIN_ALIGN: usize = 1, const UP: bool = true, A: Allocator + Clone = Global>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) chunk: RawChunk<UP, A>,
    marker: PhantomData<&'b mut ()>,
}

impl<'b, const MIN_ALIGN: usize, const UP: bool, A: Allocator + Clone> Debug for BumpScopeGuardRoot<'b, MIN_ALIGN, UP, A>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.stats().debug_format("BumpScopeGuardRoot", f)
    }
}

impl<'a, const MIN_ALIGN: usize, const UP: bool, A: Allocator + Clone> Drop for BumpScopeGuardRoot<'a, MIN_ALIGN, UP, A>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, const MIN_ALIGN: usize, const UP: bool, A: Allocator + Clone> BumpScopeGuardRoot<'a, MIN_ALIGN, UP, A>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub(crate) fn new(bump: &'a mut Bump<MIN_ALIGN, UP, A>) -> Self {
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
    pub fn scope(&mut self) -> BumpScope<MIN_ALIGN, UP, A> {
        unsafe { BumpScope::new_unchecked(self.chunk) }
    }

    #[doc = crate::doc_fn_reset!()]
    #[inline(always)]
    pub fn reset(&mut self) {
        self.chunk.reset();
    }

    #[doc = crate::doc_fn_stats!()]
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<UP> {
        Stats {
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
