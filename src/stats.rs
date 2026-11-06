//! Types for inspecting memory usage in bump allocators.

use core::{
    fmt::{self, Debug},
    iter::FusedIterator,
    marker::PhantomData,
    ptr::NonNull,
};

use alloc::alloc::Global;

use crate::{
    chunk_types::r#abstract::{AllocatedChunk, Chunk as RawChunk},
    config::{BumpConfig, DEFAULT_BUMP_CONFIG},
};

/// Provides statistics about the memory usage of the bump allocator.
///
/// This is returned from [`Bump(Scope)::stats`](crate::Bump::stats).
pub struct Stats<'a, const C: BumpConfig = DEFAULT_BUMP_CONFIG, A = Global> {
    chunk: Option<Chunk<'a, C, A>>,
}

impl<const C: BumpConfig, A> Clone for Stats<'_, C, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const C: BumpConfig, A> Copy for Stats<'_, C, A> {}

impl<const C: BumpConfig, A> PartialEq for Stats<'_, C, A> {
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<const C: BumpConfig, A> Eq for Stats<'_, C, A> {}

impl<const C: BumpConfig, A> Debug for Stats<'_, C, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_format("Stats", f)
    }
}

impl<'a, const C: BumpConfig, A> Stats<'a, C, A> {
    #[inline]
    pub(crate) fn from_raw_chunk(chunk: RawChunk<C, A>) -> Self {
        Self {
            chunk: chunk.as_allocated().map(|chunk| Chunk {
                chunk,
                marker: PhantomData,
            }),
        }
    }

    /// Returns the number of chunks.
    #[must_use]
    pub fn count(self) -> usize {
        let Some(current) = self.current_chunk() else { return 0 };

        let mut sum = 1;
        current.iter_prev().for_each(|_| sum += 1);
        current.iter_next().for_each(|_| sum += 1);
        sum
    }

    /// Returns the total size of all chunks.
    #[must_use]
    pub fn size(self) -> usize {
        let Some(current) = self.current_chunk() else { return 0 };

        let mut sum = current.size();
        current.iter_prev().for_each(|chunk| sum += chunk.size());
        current.iter_next().for_each(|chunk| sum += chunk.size());
        sum
    }

    /// Returns the total capacity of all chunks.
    #[must_use]
    pub fn capacity(self) -> usize {
        let Some(current) = self.current_chunk() else { return 0 };

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
        let Some(current) = self.current_chunk() else { return 0 };

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
        let Some(current) = self.current_chunk() else { return 0 };

        let mut sum = current.remaining();
        current.iter_next().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns an iterator from smallest to biggest chunk.
    #[must_use]
    pub fn small_to_big(self) -> ChunkNextIter<'a, C, A> {
        let Some(mut start) = self.current_chunk() else {
            return ChunkNextIter { chunk: None };
        };

        while let Some(chunk) = start.prev() {
            start = chunk;
        }

        ChunkNextIter { chunk: Some(start) }
    }

    /// Returns an iterator from biggest to smallest chunk.
    #[must_use]
    pub fn big_to_small(self) -> ChunkPrevIter<'a, C, A> {
        let Some(mut start) = self.current_chunk() else {
            return ChunkPrevIter { chunk: None };
        };

        while let Some(chunk) = start.next() {
            start = chunk;
        }

        ChunkPrevIter { chunk: Some(start) }
    }

    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn current_chunk(self) -> Option<Chunk<'a, C, A>> {
        self.chunk
    }

    /// Returns the bump pointer. It lies within the current chunk's content range.
    ///
    /// This is equivalent to `.current_chunk().map(|c| c.bump_position())`.
    #[inline]
    #[must_use]
    pub fn bump_position(self) -> Option<NonNull<u8>> {
        self.current_chunk().map(Chunk::bump_position)
    }

    /// Returns a reference to the base allocator.
    #[inline]
    #[must_use]
    pub fn allocator(self) -> Option<&'a A> {
        Some(self.current_chunk()?.allocator())
    }

    pub(crate) fn debug_format(self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(name)
            .field("allocated", &self.allocated())
            .field("capacity", &self.capacity())
            .finish()
    }
}

impl<'a, const C: BumpConfig, A> From<Chunk<'a, C, A>> for Stats<'a, C, A> {
    fn from(chunk: Chunk<'a, C, A>) -> Self {
        Stats { chunk: Some(chunk) }
    }
}

impl<const C: BumpConfig, A> Default for Stats<'_, C, A> {
    fn default() -> Self {
        Self { chunk: None }
    }
}

/// Refers to a chunk of memory that was allocated by the bump allocator.
///
/// See [`Stats`].
#[repr(transparent)]
pub struct Chunk<'a, const C: BumpConfig = DEFAULT_BUMP_CONFIG, A = Global> {
    pub(crate) chunk: AllocatedChunk<C, A>,
    marker: PhantomData<&'a ()>,
}

impl<const C: BumpConfig, A> Clone for Chunk<'_, C, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const C: BumpConfig, A> Copy for Chunk<'_, C, A> {}

impl<const C: BumpConfig, A> PartialEq for Chunk<'_, C, A> {
    fn eq(&self, other: &Self) -> bool {
        self.chunk.as_ptr() == other.chunk.as_ptr()
    }
}

impl<const C: BumpConfig, A> Eq for Chunk<'_, C, A> {}

impl<const C: BumpConfig, A> Debug for Chunk<'_, C, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk")
            .field("allocated", &self.allocated())
            .field("capacity", &self.capacity())
            .finish()
    }
}

impl<'a, const C: BumpConfig, A> Chunk<'a, C, A> {
    #[cfg(debug_assertions)]
    pub(crate) fn header(self) -> NonNull<()> {
        self.chunk.as_ptr()
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
    pub fn iter_prev(self) -> ChunkPrevIter<'a, C, A> {
        ChunkPrevIter { chunk: self.prev() }
    }

    /// Returns an iterator over all next (bigger) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_next(self) -> ChunkNextIter<'a, C, A> {
        ChunkNextIter { chunk: self.next() }
    }

    /// Returns the size of this chunk in bytes.
    #[must_use]
    #[inline]
    pub fn size(self) -> usize {
        self.chunk.size().get()
    }

    /// Returns the capacity of this chunk in bytes.
    #[inline]
    #[must_use]
    pub fn capacity(self) -> usize {
        self.chunk.capacity()
    }

    /// Returns the amount of allocated bytes.
    /// This includes padding and wasted space due to reallocations.
    ///
    /// This property can be misleading for chunks that come after the current chunk because
    /// their `bump_position` and consequently the `allocated` property is not reset until
    /// they become the current chunk again.
    #[inline]
    #[must_use]
    pub fn allocated(self) -> usize {
        self.chunk.allocated()
    }

    /// Returns the remaining capacity.
    ///
    /// This property can be misleading for chunks that come after the current chunk because
    /// their `bump_position` and consequently the `remaining` property is not reset until
    /// they become the current chunk again.
    #[inline]
    #[must_use]
    pub fn remaining(self) -> usize {
        self.chunk.remaining()
    }

    /// Returns a pointer to the start of the chunk.
    #[inline]
    #[must_use]
    pub fn chunk_start(self) -> NonNull<u8> {
        self.chunk.chunk_start()
    }

    /// Returns a pointer to the end of the chunk.
    #[inline]
    #[must_use]
    pub fn chunk_end(self) -> NonNull<u8> {
        self.chunk.chunk_end()
    }

    /// Returns a pointer to the start of the chunk's content.
    #[inline]
    #[must_use]
    pub fn content_start(self) -> NonNull<u8> {
        self.chunk.content_start()
    }

    /// Returns a pointer to the end of the chunk's content.
    #[inline]
    #[must_use]
    pub fn content_end(self) -> NonNull<u8> {
        self.chunk.content_end()
    }

    /// Returns the bump pointer. It lies within the chunk's content range.
    ///
    /// This property can be misleading for chunks that come after the current chunk because
    /// their `bump_position` is not reset until they become the current chunk again.
    #[inline]
    #[must_use]
    pub fn bump_position(self) -> NonNull<u8> {
        self.chunk.pos()
    }

    /// Returns a reference to the base allocator.
    #[inline]
    #[must_use]
    pub fn allocator(self) -> &'a A {
        unsafe { self.chunk.allocator_ptr().as_ref() }
    }
}

/// Iterator that iterates over previous chunks by continuously calling [`Chunk::prev`].
pub struct ChunkPrevIter<'a, const C: BumpConfig, A> {
    #[expect(missing_docs)]
    pub chunk: Option<Chunk<'a, C, A>>,
}

impl<const C: BumpConfig, A> Clone for ChunkPrevIter<'_, C, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const C: BumpConfig, A> Copy for ChunkPrevIter<'_, C, A> {}

impl<const C: BumpConfig, A> PartialEq for ChunkPrevIter<'_, C, A> {
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<const C: BumpConfig, A> Eq for ChunkPrevIter<'_, C, A> {}

impl<const C: BumpConfig, A> Default for ChunkPrevIter<'_, C, A> {
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl<'a, const C: BumpConfig, A> Iterator for ChunkPrevIter<'a, C, A> {
    type Item = Chunk<'a, C, A>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.prev();
        Some(chunk)
    }
}

impl<const C: BumpConfig, A> FusedIterator for ChunkPrevIter<'_, C, A> {}

impl<const C: BumpConfig, A> Debug for ChunkPrevIter<'_, C, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

/// Iterator that iterates over next chunks by continuously calling [`Chunk::next`].
pub struct ChunkNextIter<'a, const C: BumpConfig, A> {
    #[expect(missing_docs)]
    pub chunk: Option<Chunk<'a, C, A>>,
}

impl<const C: BumpConfig, A> Clone for ChunkNextIter<'_, C, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const C: BumpConfig, A> Copy for ChunkNextIter<'_, C, A> {}

impl<const C: BumpConfig, A> PartialEq for ChunkNextIter<'_, C, A> {
    fn eq(&self, other: &Self) -> bool {
        self.chunk == other.chunk
    }
}

impl<const C: BumpConfig, A> Eq for ChunkNextIter<'_, C, A> {}

impl<const C: BumpConfig, A> Default for ChunkNextIter<'_, C, A> {
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl<'a, const C: BumpConfig, A> Iterator for ChunkNextIter<'a, C, A> {
    type Item = Chunk<'a, C, A>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.next();
        Some(chunk)
    }
}

impl<const C: BumpConfig, A> FusedIterator for ChunkNextIter<'_, C, A> {}

impl<const C: BumpConfig, A> Debug for ChunkNextIter<'_, C, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.map(Chunk::size)).finish()
    }
}
