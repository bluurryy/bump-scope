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
    chunk::RawChunk,
    maybe_default_allocator,
    settings::{BumpAllocatorSettings, BumpSettings, False, True},
};

#[cfg(debug_assertions)]
use crate::chunk::ChunkHeader;

mod any;

pub use any::{AnyChunk, AnyChunkNextIter, AnyChunkPrevIter, AnyStats};

macro_rules! make_type {
    ($($allocator_parameter:tt)*) => {
        /// Provides statistics about the memory usage of the bump allocator.
        ///
        /// This is returned from [`Bump(Scope)::stats`](crate::Bump::stats).
        pub struct Stats<'a, $($allocator_parameter)*, S = BumpSettings>
        where
            S: BumpAllocatorSettings
        {
            chunk: RawChunk<A, S>,
            marker: PhantomData<&'a ()>,
        }
    };
}

maybe_default_allocator!(make_type);

impl<A, S> Clone for Stats<'_, A, S>
where
    S: BumpAllocatorSettings,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for Stats<'_, A, S> where S: BumpAllocatorSettings {}

impl<A, S> PartialEq for Stats<'_, A, S>
where
    S: BumpAllocatorSettings,
{
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<A, S> Eq for Stats<'_, A, S> where S: BumpAllocatorSettings {}

impl<A, S> Debug for Stats<'_, A, S>
where
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(*self).debug_format("Stats", f)
    }
}

impl<'a, A, S> Stats<'a, A, S>
where
    S: BumpAllocatorSettings,
{
    #[inline]
    pub(crate) fn from_raw_chunk(chunk: RawChunk<A, S>) -> Self {
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
    pub fn small_to_big(self) -> ChunkNextIter<'a, A, S::WithGuaranteedAllocated<true>> {
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
    pub fn big_to_small(self) -> ChunkPrevIter<'a, A, S::WithGuaranteedAllocated<true>> {
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
    pub fn guaranteed_allocated(self) -> Option<Stats<'a, A, S::WithGuaranteedAllocated<true>>> {
        Some(self.chunk.guaranteed_allocated()?.stats())
    }

    /// Turns this `Stats` into a `Stats` where `GUARANTEED_ALLOCATED = false`.
    #[inline]
    #[must_use]
    pub fn not_guaranteed_allocated(self) -> Stats<'a, A, S::WithGuaranteedAllocated<false>> {
        self.chunk.not_guaranteed_allocated().stats()
    }

    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn get_current_chunk(self) -> Option<Chunk<'a, A, S::WithGuaranteedAllocated<true>>> {
        Some(Chunk {
            chunk: self.chunk.guaranteed_allocated()?,
            marker: self.marker,
        })
    }
}

impl<'a, A, S> Stats<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn current_chunk(self) -> Chunk<'a, A, S> {
        Chunk {
            chunk: self.chunk,
            marker: self.marker,
        }
    }
}

impl<A, S> Stats<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = False>,
{
    pub(crate) fn unallocated() -> Self {
        Self {
            chunk: RawChunk::UNALLOCATED,
            marker: PhantomData,
        }
    }
}

impl<'a, A, S> From<Chunk<'a, A, S>> for Stats<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn from(chunk: Chunk<'a, A, S>) -> Self {
        Stats {
            chunk: chunk.chunk,
            marker: PhantomData,
        }
    }
}

impl<A, S> Default for Stats<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = False>,
{
    fn default() -> Self {
        Self::unallocated()
    }
}

/// Refers to a chunk of memory that was allocated by the bump allocator.
///
/// See [`Stats`].
#[repr(transparent)]
pub struct Chunk<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    chunk: RawChunk<A, S>,
    marker: PhantomData<&'a ()>,
}

impl<A, S> Clone for Chunk<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for Chunk<'_, A, S> where S: BumpAllocatorSettings<GuaranteedAllocated = True> {}

impl<A, S> PartialEq for Chunk<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<A, S> Eq for Chunk<'_, A, S> where S: BumpAllocatorSettings<GuaranteedAllocated = True> {}

impl<A, S> Debug for Chunk<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk")
            .field("allocated", &self.allocated())
            .field("capacity", &self.capacity())
            .finish()
    }
}

impl<'a, A, S> Chunk<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
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
    pub fn iter_prev(self) -> ChunkPrevIter<'a, A, S> {
        ChunkPrevIter { chunk: self.prev() }
    }

    /// Returns an iterator over all next (bigger) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_next(self) -> ChunkNextIter<'a, A, S> {
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
pub struct ChunkPrevIter<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    #[expect(missing_docs)]
    pub chunk: Option<Chunk<'a, A, S>>,
}

impl<A, S> Clone for ChunkPrevIter<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for ChunkPrevIter<'_, A, S> where S: BumpAllocatorSettings<GuaranteedAllocated = True> {}

impl<A, S> PartialEq for ChunkPrevIter<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<A, S> Eq for ChunkPrevIter<'_, A, S> where S: BumpAllocatorSettings<GuaranteedAllocated = True> {}

impl<A, S> Default for ChunkPrevIter<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl<'a, A, S> Iterator for ChunkPrevIter<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    type Item = Chunk<'a, A, S>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.prev();
        Some(chunk)
    }
}

impl<A, S> FusedIterator for ChunkPrevIter<'_, A, S> where S: BumpAllocatorSettings<GuaranteedAllocated = True> {}

impl<A, S> Debug for ChunkPrevIter<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

/// Iterator that iterates over next chunks by continuously calling [`Chunk::next`].
pub struct ChunkNextIter<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    #[expect(missing_docs)]
    pub chunk: Option<Chunk<'a, A, S>>,
}

impl<A, S> Clone for ChunkNextIter<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for ChunkNextIter<'_, A, S> where S: BumpAllocatorSettings<GuaranteedAllocated = True> {}

impl<A, S> PartialEq for ChunkNextIter<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<A, S> Eq for ChunkNextIter<'_, A, S> where S: BumpAllocatorSettings<GuaranteedAllocated = True> {}

impl<A, S> Default for ChunkNextIter<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl<'a, A, S> Iterator for ChunkNextIter<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    type Item = Chunk<'a, A, S>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.next();
        Some(chunk)
    }
}

impl<A, S> FusedIterator for ChunkNextIter<'_, A, S> where S: BumpAllocatorSettings<GuaranteedAllocated = True> {}

impl<A, S> Debug for ChunkNextIter<'_, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.map(Chunk::size)).finish()
    }
}
