//! Contains types for inspecting memory usage in bump allocators.
//!
//! This module defines both generic types like [`Stats`] and type-erased counterparts prefixed
//! with `Any*` (like [`AnyStats`]). The generic types are slightly more efficient to use.
//! You can turn the generic types into their `Any*` variants using `from` and `into`.
//!
//! The `Any*` types are returned by the [`BumpAllocator`](crate::BumpAllocator) trait
//! and the `allocator_stats` method of collections whereas `Stats` is returned from [`Bump`](crate::Bump) and [`BumpScope`](crate::BumpScope).

use core::{
    fmt::{self, Debug},
    iter::FusedIterator,
    marker::PhantomData,
    ptr::NonNull,
};

use crate::{RawChunk, maybe_default_allocator};

#[cfg(debug_assertions)]
use crate::chunk_header::ChunkHeader;

mod any;

pub use any::{AnyChunk, AnyChunkNextIter, AnyChunkPrevIter, AnyStats};

macro_rules! make_type {
    ($($allocator_parameter:tt)*) => {
        /// Provides statistics about the memory usage of the bump allocator.
        ///
        /// This is returned from the `stats` method of [`Bump`](crate::Bump) and [`BumpScope`](crate::BumpScope).
        pub struct Stats<
            'a,
            $($allocator_parameter)*,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        > {
            chunk: RawChunk<A, UP, GUARANTEED_ALLOCATED>,
            marker: PhantomData<&'a ()>,
        }
    };
}

maybe_default_allocator!(make_type);

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> Clone for Stats<'_, A, UP, GUARANTEED_ALLOCATED> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> Copy for Stats<'_, A, UP, GUARANTEED_ALLOCATED> {}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> PartialEq for Stats<'_, A, UP, GUARANTEED_ALLOCATED> {
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> Eq for Stats<'_, A, UP, GUARANTEED_ALLOCATED> {}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> Debug for Stats<'_, A, UP, GUARANTEED_ALLOCATED> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(*self).debug_format("Stats", f)
    }
}

impl<'a, A, const UP: bool, const GUARANTEED_ALLOCATED: bool> Stats<'a, A, UP, GUARANTEED_ALLOCATED> {
    #[inline]
    pub(crate) fn from_raw_chunk(chunk: RawChunk<A, UP, GUARANTEED_ALLOCATED>) -> Self {
        Self {
            chunk,
            marker: PhantomData,
        }
    }

    /// Returns the number of chunks.
    #[must_use]
    pub fn count(self) -> usize {
        let Some(current) = self.get_current_chunk() else { return 0 };

        let mut sum = 1;
        current.iter_prev().for_each(|_| sum += 1);
        current.iter_next().for_each(|_| sum += 1);
        sum
    }

    /// Returns the total size of all chunks.
    #[must_use]
    pub fn size(self) -> usize {
        let Some(current) = self.get_current_chunk() else { return 0 };

        let mut sum = current.size();
        current.iter_prev().for_each(|chunk| sum += chunk.size());
        current.iter_next().for_each(|chunk| sum += chunk.size());
        sum
    }

    /// Returns the total capacity of all chunks.
    #[must_use]
    pub fn capacity(self) -> usize {
        let Some(current) = self.get_current_chunk() else { return 0 };

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
        let Some(current) = self.get_current_chunk() else { return 0 };

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
        let Some(current) = self.get_current_chunk() else { return 0 };

        let mut sum = current.remaining();
        current.iter_next().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns an iterator from smallest to biggest chunk.
    #[must_use]
    pub fn small_to_big(self) -> ChunkNextIter<'a, A, UP> {
        let Some(mut start) = self.get_current_chunk() else {
            return ChunkNextIter { chunk: None };
        };

        while let Some(chunk) = start.prev() {
            start = chunk;
        }

        ChunkNextIter { chunk: Some(start) }
    }

    /// Returns an iterator from biggest to smallest chunk.
    #[must_use]
    pub fn big_to_small(self) -> ChunkPrevIter<'a, A, UP> {
        let Some(mut start) = self.get_current_chunk() else {
            return ChunkPrevIter { chunk: None };
        };

        while let Some(chunk) = start.next() {
            start = chunk;
        }

        ChunkPrevIter { chunk: Some(start) }
    }

    /// Turns this `Stats` into a `Stats` where `GUARANTEED_ALLOCATED = true`.
    #[inline]
    #[must_use]
    pub fn guaranteed_allocated(self) -> Option<Stats<'a, A, UP, true>> {
        Some(self.chunk.guaranteed_allocated()?.stats())
    }

    /// Turns this `Stats` into a `Stats` where `GUARANTEED_ALLOCATED = false`.
    #[inline]
    #[must_use]
    pub fn not_guaranteed_allocated(self) -> Stats<'a, A, UP, false> {
        self.chunk.not_guaranteed_allocated().stats()
    }

    fn get_current_chunk(self) -> Option<Chunk<'a, A, UP>> {
        Some(Chunk {
            chunk: self.chunk.guaranteed_allocated()?,
            marker: PhantomData,
        })
    }
}

impl<'a, A, const UP: bool> Stats<'a, A, UP, true> {
    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn current_chunk(self) -> Chunk<'a, A, UP> {
        Chunk {
            chunk: self.chunk,
            marker: self.marker,
        }
    }
}

impl<'a, A, const UP: bool> Stats<'a, A, UP, false> {
    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn current_chunk(self) -> Option<Chunk<'a, A, UP>> {
        Some(Chunk {
            chunk: self.chunk.guaranteed_allocated()?,
            marker: self.marker,
        })
    }

    pub(crate) fn unallocated() -> Self {
        Self {
            chunk: RawChunk::UNALLOCATED,
            marker: PhantomData,
        }
    }
}

impl<'a, A, const UP: bool, const GUARANTEED_ALLOCATED: bool> From<Chunk<'a, A, UP>>
    for Stats<'a, A, UP, GUARANTEED_ALLOCATED>
{
    fn from(chunk: Chunk<'a, A, UP>) -> Self {
        Stats {
            chunk: chunk.chunk.coerce_guaranteed_allocated(),
            marker: PhantomData,
        }
    }
}

impl<A, const UP: bool> Default for Stats<'_, A, UP, false> {
    fn default() -> Self {
        Self::unallocated()
    }
}

/// Refers to a chunk of memory that was allocated by the bump allocator.
///
/// See [`Stats`].
#[repr(transparent)]
pub struct Chunk<'a, A, const UP: bool> {
    chunk: RawChunk<A, UP, true>,
    marker: PhantomData<&'a ()>,
}

impl<A, const UP: bool> Clone for Chunk<'_, A, UP> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const UP: bool> Copy for Chunk<'_, A, UP> {}

impl<A, const UP: bool> PartialEq for Chunk<'_, A, UP> {
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<A, const UP: bool> Eq for Chunk<'_, A, UP> {}

impl<A, const UP: bool> Debug for Chunk<'_, A, UP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk")
            .field("allocated", &self.allocated())
            .field("capacity", &self.capacity())
            .finish()
    }
}

impl<'a, A, const UP: bool> Chunk<'a, A, UP> {
    #[cfg(debug_assertions)]
    pub(crate) fn header(self) -> NonNull<ChunkHeader> {
        self.chunk.header().cast()
    }

    /// Returns the previous (smaller) chunk.
    #[must_use]
    #[inline(always)]
    pub fn prev(self) -> Option<Self> {
        Some(Chunk {
            chunk: self.chunk.prev()?,
            marker: PhantomData,
        })
    }

    /// Returns the next (bigger) chunk.
    #[must_use]
    #[inline(always)]
    pub fn next(self) -> Option<Self> {
        Some(Chunk {
            chunk: self.chunk.next()?,
            marker: PhantomData,
        })
    }

    /// Returns an iterator over all previous (smaller) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_prev(self) -> ChunkPrevIter<'a, A, UP> {
        ChunkPrevIter { chunk: self.prev() }
    }

    /// Returns an iterator over all next (bigger) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_next(self) -> ChunkNextIter<'a, A, UP> {
        ChunkNextIter { chunk: self.next() }
    }

    /// Returns the size of this chunk in bytes.
    #[must_use]
    #[inline]
    pub fn size(self) -> usize {
        self.chunk.size().get()
    }

    /// Returns the capacity of this chunk in bytes.
    #[must_use]
    #[inline]
    pub fn capacity(self) -> usize {
        self.chunk.capacity()
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
        self.chunk.allocated()
    }

    /// Returns the remaining capacity.
    ///
    /// This property can be misleading for chunks that come after the current chunk because
    /// their `bump_position` and consequently the `remaining` property is not reset until
    /// they become the current chunk again.
    #[must_use]
    #[inline]
    pub fn remaining(self) -> usize {
        self.chunk.remaining()
    }

    /// Returns a pointer to the start of the chunk.
    #[must_use]
    #[inline]
    pub fn chunk_start(self) -> NonNull<u8> {
        self.chunk.chunk_start()
    }

    /// Returns a pointer to the end of the chunk.
    #[must_use]
    #[inline]
    pub fn chunk_end(self) -> NonNull<u8> {
        self.chunk.chunk_end()
    }

    /// Returns a pointer to the start of the chunk's content.
    #[must_use]
    #[inline]
    pub fn content_start(self) -> NonNull<u8> {
        self.chunk.content_start()
    }

    /// Returns a pointer to the end of the chunk's content.
    #[must_use]
    #[inline]
    pub fn content_end(self) -> NonNull<u8> {
        self.chunk.content_end()
    }

    /// Returns the bump pointer. It lies within the chunk's content range.
    ///
    /// This property can be misleading for chunks that come after the current chunk because
    /// their `bump_position` is not reset until they become the current chunk again.
    #[must_use]
    #[inline]
    pub fn bump_position(self) -> NonNull<u8> {
        self.chunk.pos()
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &'a A {
        self.chunk.allocator()
    }

    #[cfg(debug_assertions)]
    pub(crate) fn contains_addr_or_end(self, addr: usize) -> bool {
        self.chunk.contains_addr_or_end(addr)
    }
}

/// Iterator that iterates over previous chunks by continuously calling [`Chunk::prev`].
pub struct ChunkPrevIter<'a, A, const UP: bool> {
    #[allow(missing_docs)]
    pub chunk: Option<Chunk<'a, A, UP>>,
}

impl<A, const UP: bool> Clone for ChunkPrevIter<'_, A, UP> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const UP: bool> Copy for ChunkPrevIter<'_, A, UP> {}

impl<A, const UP: bool> PartialEq for ChunkPrevIter<'_, A, UP> {
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<A, const UP: bool> Eq for ChunkPrevIter<'_, A, UP> {}

impl<A, const UP: bool> Default for ChunkPrevIter<'_, A, UP> {
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl<'a, A, const UP: bool> Iterator for ChunkPrevIter<'a, A, UP> {
    type Item = Chunk<'a, A, UP>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.prev();
        Some(chunk)
    }
}

impl<A, const UP: bool> FusedIterator for ChunkPrevIter<'_, A, UP> {}

impl<A, const UP: bool> Debug for ChunkPrevIter<'_, A, UP> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

/// Iterator that iterates over next chunks by continuously calling [`Chunk::next`].
pub struct ChunkNextIter<'a, A, const UP: bool> {
    #[allow(missing_docs)]
    pub chunk: Option<Chunk<'a, A, UP>>,
}

impl<A, const UP: bool> Clone for ChunkNextIter<'_, A, UP> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const UP: bool> Copy for ChunkNextIter<'_, A, UP> {}

impl<A, const UP: bool> PartialEq for ChunkNextIter<'_, A, UP> {
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<A, const UP: bool> Eq for ChunkNextIter<'_, A, UP> {}

impl<A, const UP: bool> Default for ChunkNextIter<'_, A, UP> {
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl<'a, A, const UP: bool> Iterator for ChunkNextIter<'a, A, UP> {
    type Item = Chunk<'a, A, UP>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.next();
        Some(chunk)
    }
}

impl<A, const UP: bool> FusedIterator for ChunkNextIter<'_, A, UP> {}

impl<A, const UP: bool> Debug for ChunkNextIter<'_, A, UP> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.map(Chunk::size)).finish()
    }
}
