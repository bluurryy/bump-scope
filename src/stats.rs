//! Contains types for inspecting memory usage in bump allocators.
//!
//! This module defines both generic types like [`Stats`] and type-erased counterparts prefixed
//! with `Any*`. The generic types are slightly more efficient to use.
//! You can turn the generic types into their `Any*` variants using `from` and `into`.
//!
//! The `Any*` types are returned by the [`BumpAllocatorCore::any_stats`](crate::traits::BumpAllocatorCore::any_stats) trait
//! whereas `Stats` is returned from [`Bump(Scope)::stats`](crate::Bump::stats).

use core::{
    fmt::{self, Debug},
    iter::FusedIterator,
    marker::PhantomData,
    ptr::NonNull,
};

use crate::{
    raw_bump::{NonDummyChunk, RawChunk},
    settings::{BumpAllocatorSettings, BumpSettings, False, True},
};

#[cfg(debug_assertions)]
use crate::chunk::ChunkHeader;

mod any;

pub use any::{AnyChunk, AnyChunkNextIter, AnyChunkPrevIter, AnyStats};

/// Provides statistics about the memory usage of the bump allocator.
///
/// This is returned from [`Bump(Scope)::stats`](crate::Bump::stats).
pub struct Stats<'a, S = BumpSettings>
where
    S: BumpAllocatorSettings,
{
    chunk: RawChunk<S>,
    marker: PhantomData<&'a ()>,
}

impl<S> Clone for Stats<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Stats<'_, S> where S: BumpAllocatorSettings {}

impl<S> PartialEq for Stats<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn eq(&self, other: &Self) -> bool {
        self.chunk.header() == other.chunk.header()
    }
}

impl<S> Eq for Stats<'_, S> where S: BumpAllocatorSettings {}

impl<S> Debug for Stats<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(*self).debug_format("Stats", f)
    }
}

impl<'a, S> Stats<'a, S>
where
    S: BumpAllocatorSettings,
{
    #[inline]
    pub(crate) fn from_raw_chunk(chunk: RawChunk<S>) -> Self {
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
    pub fn small_to_big(self) -> ChunkNextIter<'a, S> {
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
    pub fn big_to_small(self) -> ChunkPrevIter<'a, S> {
        let Some(mut start) = self.get_current_chunk() else {
            return ChunkPrevIter { chunk: None };
        };

        while let Some(chunk) = start.next() {
            start = chunk;
        }

        ChunkPrevIter { chunk: Some(start) }
    }

    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn get_current_chunk(self) -> Option<Chunk<'a, S>> {
        Some(Chunk {
            chunk: self.chunk.as_non_dummy()?,
            marker: self.marker,
        })
    }
}

impl<'a, S> Stats<'a, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True, Claimable = False>,
{
    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn current_chunk(self) -> Chunk<'a, S> {
        Chunk {
            chunk: self.chunk.non_dummy(),
            marker: self.marker,
        }
    }
}

impl<'a, S> From<Chunk<'a, S>> for Stats<'a, S>
where
    S: BumpAllocatorSettings,
{
    fn from(chunk: Chunk<'a, S>) -> Self {
        Stats {
            chunk: *chunk.chunk,
            marker: PhantomData,
        }
    }
}

impl<S> Default for Stats<'_, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = False>,
{
    fn default() -> Self {
        Self {
            chunk: RawChunk::UNALLOCATED,
            marker: PhantomData,
        }
    }
}

/// Refers to a chunk of memory that was allocated by the bump allocator.
///
/// See [`Stats`].
#[repr(transparent)]
pub struct Chunk<'a, S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) chunk: NonDummyChunk<S>,
    marker: PhantomData<&'a ()>,
}

impl<S> Clone for Chunk<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Chunk<'_, S> where S: BumpAllocatorSettings {}

impl<S> PartialEq for Chunk<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn eq(&self, other: &Self) -> bool {
        self.chunk.header() == other.chunk.header()
    }
}

impl<S> Eq for Chunk<'_, S> where S: BumpAllocatorSettings {}

impl<S> Debug for Chunk<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk")
            .field("allocated", &self.allocated())
            .field("capacity", &self.capacity())
            .finish()
    }
}

impl<'a, S> Chunk<'a, S>
where
    S: BumpAllocatorSettings,
{
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
    pub fn iter_prev(self) -> ChunkPrevIter<'a, S> {
        ChunkPrevIter { chunk: self.prev() }
    }

    /// Returns an iterator over all next (bigger) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_next(self) -> ChunkNextIter<'a, S> {
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
}

/// Iterator that iterates over previous chunks by continuously calling [`Chunk::prev`].
pub struct ChunkPrevIter<'a, S>
where
    S: BumpAllocatorSettings,
{
    #[expect(missing_docs)]
    pub chunk: Option<Chunk<'a, S>>,
}

impl<S> Clone for ChunkPrevIter<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ChunkPrevIter<'_, S> where S: BumpAllocatorSettings {}

impl<S> PartialEq for ChunkPrevIter<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<S> Eq for ChunkPrevIter<'_, S> where S: BumpAllocatorSettings {}

impl<S> Default for ChunkPrevIter<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl<'a, S> Iterator for ChunkPrevIter<'a, S>
where
    S: BumpAllocatorSettings,
{
    type Item = Chunk<'a, S>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.prev();
        Some(chunk)
    }
}

impl<S> FusedIterator for ChunkPrevIter<'_, S> where S: BumpAllocatorSettings {}

impl<S> Debug for ChunkPrevIter<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

/// Iterator that iterates over next chunks by continuously calling [`Chunk::next`].
pub struct ChunkNextIter<'a, S>
where
    S: BumpAllocatorSettings,
{
    #[expect(missing_docs)]
    pub chunk: Option<Chunk<'a, S>>,
}

impl<S> Clone for ChunkNextIter<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ChunkNextIter<'_, S> where S: BumpAllocatorSettings {}

impl<S> PartialEq for ChunkNextIter<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<S> Eq for ChunkNextIter<'_, S> where S: BumpAllocatorSettings {}

impl<S> Default for ChunkNextIter<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl<'a, S> Iterator for ChunkNextIter<'a, S>
where
    S: BumpAllocatorSettings,
{
    type Item = Chunk<'a, S>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.next();
        Some(chunk)
    }
}

impl<S> FusedIterator for ChunkNextIter<'_, S> where S: BumpAllocatorSettings {}

impl<S> Debug for ChunkNextIter<'_, S>
where
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.map(Chunk::size)).finish()
    }
}
