use core::{fmt, iter::FusedIterator, marker::PhantomData, mem, ptr::NonNull};

use crate::ChunkHeader;

use super::{Chunk, ChunkNextIter, ChunkPrevIter, Stats};

/// Provides statistics about the memory usage of the bump allocator.
///
/// This is returned from the `stats` method of [`BumpAllocator`](crate::BumpAllocator), strings and vectors.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct AnyStats<'a> {
    chunk: Option<AnyChunk<'a>>,
}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> From<Stats<'_, A, UP, GUARANTEED_ALLOCATED>> for AnyStats<'_> {
    fn from(value: Stats<'_, A, UP, GUARANTEED_ALLOCATED>) -> Self {
        Self {
            chunk: value.get_current_chunk().map(Into::into),
        }
    }
}

impl fmt::Debug for AnyStats<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_format("AnyStats", f)
    }
}

impl<'a> AnyStats<'a> {
    /// Returns the number of chunks.
    #[must_use]
    pub fn count(self) -> usize {
        let Some(current) = self.chunk else { return 0 };

        let mut sum = 1;
        current.iter_prev().for_each(|_| sum += 1);
        current.iter_next().for_each(|_| sum += 1);
        sum
    }

    /// Returns the total size of all chunks.
    #[must_use]
    pub fn size(self) -> usize {
        let Some(current) = self.chunk else { return 0 };

        let mut sum = current.size();
        current.iter_prev().for_each(|chunk| sum += chunk.size());
        current.iter_next().for_each(|chunk| sum += chunk.size());
        sum
    }

    /// Returns the total capacity of all chunks.
    #[must_use]
    pub fn capacity(self) -> usize {
        let Some(current) = self.chunk else { return 0 };

        let mut sum = current.capacity();
        current.iter_prev().for_each(|chunk| sum += chunk.capacity());
        current.iter_next().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns the amount of allocated bytes.
    /// This includes padding and wasted space due to reallocations.
    ///
    /// This is equal to the `allocated` bytes of the current chunk
    /// plus the `capacity` of all previous chunks.
    #[must_use]
    pub fn allocated(self) -> usize {
        let Some(current) = self.chunk else { return 0 };

        let mut sum = current.allocated();
        current.iter_prev().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns the remaining capacity in bytes.
    ///
    /// This is equal to the `remaining` capacity of the current chunk
    /// plus the `capacity` of all following chunks.
    #[must_use]
    pub fn remaining(self) -> usize {
        let Some(current) = self.chunk else { return 0 };

        let mut sum = current.remaining();
        current.iter_next().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns an iterator from smallest to biggest chunk.
    #[must_use]
    pub fn small_to_big(self) -> AnyChunkNextIter<'a> {
        let Some(mut start) = self.chunk else {
            return AnyChunkNextIter { chunk: None };
        };

        while let Some(chunk) = start.prev() {
            start = chunk;
        }

        AnyChunkNextIter { chunk: Some(start) }
    }

    /// Returns an iterator from biggest to smallest chunk.
    #[must_use]
    pub fn big_to_small(self) -> AnyChunkPrevIter<'a> {
        let Some(mut start) = self.chunk else {
            return AnyChunkPrevIter { chunk: None };
        };

        while let Some(chunk) = start.next() {
            start = chunk;
        }

        AnyChunkPrevIter { chunk: Some(start) }
    }

    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn current_chunk(self) -> Option<AnyChunk<'a>> {
        self.chunk
    }

    pub(crate) fn debug_format(self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(name)
            .field("allocated", &self.allocated())
            .field("capacity", &self.capacity())
            .finish()
    }
}

impl<'a> From<AnyChunk<'a>> for AnyStats<'a> {
    fn from(chunk: AnyChunk<'a>) -> Self {
        Self { chunk: Some(chunk) }
    }
}

/// Refers to a chunk of memory that was allocated by the bump allocator.
///
/// See [`AnyStats`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnyChunk<'a> {
    header: NonNull<ChunkHeader>,
    header_size: usize,
    marker: PhantomData<&'a ()>,
}

impl<A, const UP: bool> From<Chunk<'_, A, UP>> for AnyChunk<'_> {
    fn from(value: Chunk<'_, A, UP>) -> Self {
        Self {
            header: value.chunk.header().cast(),
            header_size: mem::size_of::<ChunkHeader<A>>(),
            marker: PhantomData,
        }
    }
}

impl fmt::Debug for AnyChunk<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk")
            .field("allocated", &self.allocated())
            .field("capacity", &self.capacity())
            .finish()
    }
}

impl<'a> AnyChunk<'a> {
    fn header(&self) -> &ChunkHeader {
        unsafe { self.header.as_ref() }
    }

    #[inline]
    pub(crate) fn is_upwards_allocating(self) -> bool {
        let header = self.header.addr();
        let end = self.header().end.addr();
        end > header
    }

    /// Returns the previous (smaller) chunk.
    #[must_use]
    #[inline(always)]
    pub fn prev(self) -> Option<Self> {
        Some(AnyChunk {
            header: self.header().prev.get()?,
            header_size: self.header_size,
            marker: PhantomData,
        })
    }

    /// Returns the next (bigger) chunk.
    #[must_use]
    #[inline(always)]
    pub fn next(self) -> Option<Self> {
        Some(AnyChunk {
            header: self.header().next.get()?,
            header_size: self.header_size,
            marker: PhantomData,
        })
    }

    /// Returns an iterator over all previous (smaller) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_prev(self) -> AnyChunkPrevIter<'a> {
        AnyChunkPrevIter { chunk: self.prev() }
    }

    /// Returns an iterator over all next (bigger) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_next(self) -> AnyChunkNextIter<'a> {
        AnyChunkNextIter { chunk: self.next() }
    }

    /// Returns the size of this chunk in bytes.
    #[must_use]
    #[inline]
    pub fn size(self) -> usize {
        let start = self.chunk_start();
        let end = self.chunk_end();
        end.addr().get() - start.addr().get()
    }

    /// Returns the capacity of this chunk in bytes.
    #[must_use]
    #[inline]
    pub fn capacity(self) -> usize {
        let start = self.content_start();
        let end = self.content_end();
        end.addr().get() - start.addr().get()
    }

    /// Returns the amount of allocated bytes.
    /// This includes padding and wasted space due to reallocations.
    ///
    /// This property can be misleading for chunks that come after the current chunk because
    /// their `bump_position` and consequently the `allocated` property is not reset until
    /// they become the current chunk again.
    #[must_use]
    #[inline]
    pub fn allocated(self) -> usize {
        if self.is_upwards_allocating() {
            let start = self.content_start();
            let end = self.bump_position();
            end.addr().get() - start.addr().get()
        } else {
            let start = self.bump_position();
            let end = self.content_end();
            end.addr().get() - start.addr().get()
        }
    }

    /// Returns the remaining capacity.
    ///
    /// This property can be misleading for chunks that come after the current chunk because
    /// their `bump_position` and consequently the `remaining` property is not reset until
    /// they become the current chunk again.
    #[must_use]
    #[inline]
    pub fn remaining(self) -> usize {
        if self.is_upwards_allocating() {
            let start = self.bump_position();
            let end = self.content_end();
            end.addr().get() - start.addr().get()
        } else {
            let start = self.content_start();
            let end = self.bump_position();
            end.addr().get() - start.addr().get()
        }
    }

    /// Returns a pointer to the start of the chunk.
    #[must_use]
    #[inline]
    pub fn chunk_start(self) -> NonNull<u8> {
        if self.is_upwards_allocating() {
            self.header.cast()
        } else {
            self.header().end
        }
    }

    /// Returns a pointer to the end of the chunk.
    #[must_use]
    #[inline]
    pub fn chunk_end(self) -> NonNull<u8> {
        if self.is_upwards_allocating() {
            self.header().end
        } else {
            self.after_header()
        }
    }

    /// Returns a pointer to the start of the chunk's content.
    #[must_use]
    #[inline]
    pub fn content_start(self) -> NonNull<u8> {
        if self.is_upwards_allocating() {
            self.after_header()
        } else {
            self.chunk_start()
        }
    }

    /// Returns a pointer to the end of the chunk's content.
    #[must_use]
    #[inline]
    pub fn content_end(self) -> NonNull<u8> {
        if self.is_upwards_allocating() {
            self.chunk_end()
        } else {
            self.header.cast()
        }
    }

    /// Returns the bump pointer. It lies within the chunk's content range.
    ///
    /// This property can be misleading for chunks that come after the current chunk because
    /// their `bump_position` is not reset until they become the current chunk again.
    #[must_use]
    #[inline]
    pub fn bump_position(self) -> NonNull<u8> {
        self.header().pos.get()
    }

    fn after_header(self) -> NonNull<u8> {
        unsafe { self.header.byte_add(self.header_size).cast() }
    }
}

/// Iterator that iterates over previous chunks by continuously calling [`AnyChunk::prev`].
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct AnyChunkPrevIter<'a> {
    #[allow(missing_docs)]
    pub chunk: Option<AnyChunk<'a>>,
}

impl<A, const UP: bool> From<ChunkPrevIter<'_, A, UP>> for AnyChunkPrevIter<'_> {
    fn from(value: ChunkPrevIter<'_, A, UP>) -> Self {
        Self {
            chunk: value.chunk.map(Into::into),
        }
    }
}

impl<'a> Iterator for AnyChunkPrevIter<'a> {
    type Item = AnyChunk<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.prev();
        Some(chunk)
    }
}

impl FusedIterator for AnyChunkPrevIter<'_> {}

impl fmt::Debug for AnyChunkPrevIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

/// Iterator that iterates over next chunks by continuously calling [`AnyChunk::next`].
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct AnyChunkNextIter<'a> {
    #[allow(missing_docs)]
    pub chunk: Option<AnyChunk<'a>>,
}

impl<A, const UP: bool> From<ChunkNextIter<'_, A, UP>> for AnyChunkNextIter<'_> {
    fn from(value: ChunkNextIter<'_, A, UP>) -> Self {
        Self {
            chunk: value.chunk.map(Into::into),
        }
    }
}

impl<'a> Iterator for AnyChunkNextIter<'a> {
    type Item = AnyChunk<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.next();
        Some(chunk)
    }
}

impl FusedIterator for AnyChunkNextIter<'_> {}

impl fmt::Debug for AnyChunkNextIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.map(AnyChunk::size)).finish()
    }
}

#[test]
fn check_from_impls() {
    #![allow(dead_code, clippy::needless_lifetimes, clippy::elidable_lifetime_names)]

    use crate::{BaseAllocator, BumpScope, MinimumAlignment, SupportedMinimumAlignment};

    fn accepting_any_stats(_: AnyStats) {}
    fn accepting_any_chunk(_: AnyChunk) {}
    fn accepting_any_chunk_prev_iter(_: AnyChunkPrevIter) {}
    fn accepting_any_chunk_next_iter(_: AnyChunkNextIter) {}

    fn generic_bump<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>(
        bump: &BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
    ) where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        A: BaseAllocator<GUARANTEED_ALLOCATED>,
    {
        let stats = bump.stats();
        accepting_any_stats(stats.into());
        accepting_any_chunk(stats.not_guaranteed_allocated().current_chunk().unwrap().into());
        accepting_any_chunk_next_iter(stats.small_to_big().into());
        accepting_any_chunk_prev_iter(stats.big_to_small().into());
    }
}
