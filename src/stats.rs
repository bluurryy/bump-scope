use crate::{BaseAllocator, BumpScope, FmtFn, MinimumAlignment, RawChunk, SupportedMinimumAlignment};
use core::{
    fmt::{self, Debug},
    iter::FusedIterator,
    marker::PhantomData,
    ptr::NonNull,
};

/// Provides statistics about the memory usage of the bump allocator.
///
/// This is returned from the `guaranteed_allocated_stats` method of `Bump`, `BumpScope`, `BumpScopeGuard`, `BumpVec`, ...
#[repr(transparent)]
pub struct GuaranteedAllocatedStats<'a, const UP: bool> {
    /// This is the chunk we are currently allocating on.
    pub current: Chunk<'a, UP>,
}

impl<const UP: bool> Copy for GuaranteedAllocatedStats<'_, UP> {}

#[allow(clippy::expl_impl_clone_on_copy)]
impl<const UP: bool> Clone for GuaranteedAllocatedStats<'_, UP> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<const UP: bool> PartialEq for GuaranteedAllocatedStats<'_, UP> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
    }

    #[inline(always)]
    fn ne(&self, other: &Self) -> bool {
        self.current != other.current
    }
}

impl<const UP: bool> Eq for GuaranteedAllocatedStats<'_, UP> {}

impl<const UP: bool> Debug for GuaranteedAllocatedStats<'_, UP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_format("GuaranteedAllocatedStats", f)
    }
}

impl<'a, const UP: bool> GuaranteedAllocatedStats<'a, UP> {
    /// Returns the number of chunks.
    #[must_use]
    pub fn count(self) -> usize {
        let mut sum = 1;
        self.current.iter_prev().for_each(|_| sum += 1);
        self.current.iter_next().for_each(|_| sum += 1);
        sum
    }

    /// Returns the total size of all chunks.
    #[must_use]
    pub fn size(self) -> usize {
        let mut sum = self.current.size();
        self.current.iter_prev().for_each(|chunk| sum += chunk.size());
        self.current.iter_next().for_each(|chunk| sum += chunk.size());
        sum
    }

    /// Returns the total capacity of all chunks.
    #[must_use]
    pub fn capacity(self) -> usize {
        let mut sum = self.current.capacity();
        self.current.iter_prev().for_each(|chunk| sum += chunk.capacity());
        self.current.iter_next().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns the amount of allocated bytes. This includes padding and wasted space due to reallocations.
    #[must_use]
    pub fn allocated(self) -> usize {
        let mut sum = self.current.allocated();
        self.current.iter_prev().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns the total remaining capacity of all chunks.
    #[must_use]
    pub fn remaining(self) -> usize {
        let mut sum = self.current.remaining();
        self.current.iter_next().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns an iterator from smallest to biggest chunk.
    #[must_use]
    pub fn small_to_big(self) -> ChunkNextIter<'a, UP> {
        let mut start = self.current;

        while let Some(chunk) = start.prev() {
            start = chunk;
        }

        ChunkNextIter { chunk: Some(start) }
    }

    /// Returns an iterator from biggest to smallest chunk.
    #[must_use]
    pub fn big_to_small(self) -> ChunkPrevIter<'a, UP> {
        let mut start = self.current;

        while let Some(chunk) = start.next() {
            start = chunk;
        }

        ChunkPrevIter { chunk: Some(start) }
    }

    pub(crate) fn debug_format(self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fmt_list_compact<T: Debug>(iter: impl Iterator<Item = T> + Clone) -> impl Debug {
            let list = FmtFn(move |f| f.debug_list().entries(iter.clone()).finish());
            FmtFn(move |f| write!(f, "{list:?}"))
        }

        let alternate = f.alternate();
        let mut debug = f.debug_struct(name);

        if alternate {
            // low-level, verbose
            let (index, first) = match self.current.iter_prev().enumerate().last() {
                Some((i, chunk)) => (i + 1, chunk),
                None => (0, self.current),
            };
            let chunks = FmtFn(move |f| f.debug_list().entries(ChunkNextIter { chunk: Some(first) }).finish());

            debug.field("current", &index);
            debug.field("chunks", &chunks);
        } else {
            // high-level
            let chunk = self.current;
            debug.field("prev", &fmt_list_compact(chunk.iter_prev().map(Chunk::size)));
            debug.field("next", &fmt_list_compact(chunk.iter_next().map(Chunk::size)));
            debug.field("curr", &chunk);
        }

        debug.finish()
    }
}

/// Provides statistics about the memory usage of the bump allocator.
///
/// This is returned from the `stats` method of `Bump`, `BumpScope`, `BumpScopeGuard`, `BumpVec`, ...
pub struct Stats<'a, const UP: bool> {
    /// This is the chunk we are currently allocating on.
    pub current: Option<Chunk<'a, UP>>,
}

impl<const UP: bool> Copy for Stats<'_, UP> {}

#[allow(clippy::expl_impl_clone_on_copy)]
impl<const UP: bool> Clone for Stats<'_, UP> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<const UP: bool> PartialEq for Stats<'_, UP> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
    }

    #[inline(always)]
    fn ne(&self, other: &Self) -> bool {
        self.current != other.current
    }
}

impl<const UP: bool> Eq for Stats<'_, UP> {}

impl<const UP: bool> Debug for Stats<'_, UP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_format("Stats", f)
    }
}

impl<'a, const UP: bool> From<GuaranteedAllocatedStats<'a, UP>> for Stats<'a, UP> {
    fn from(value: GuaranteedAllocatedStats<'a, UP>) -> Self {
        Stats {
            current: Some(value.current),
        }
    }
}

impl<'a, const UP: bool> Stats<'a, UP> {
    /// Returns the number of chunks.
    #[must_use]
    pub fn count(self) -> usize {
        let current = match self.current {
            Some(current) => current,
            None => return 0,
        };

        let mut sum = 1;
        current.iter_prev().for_each(|_| sum += 1);
        current.iter_next().for_each(|_| sum += 1);
        sum
    }

    /// Returns the total size of all chunks.
    #[must_use]
    pub fn size(self) -> usize {
        let current = match self.current {
            Some(current) => current,
            None => return 0,
        };

        let mut sum = current.size();
        current.iter_prev().for_each(|chunk| sum += chunk.size());
        current.iter_next().for_each(|chunk| sum += chunk.size());
        sum
    }

    /// Returns the total capacity of all chunks.
    #[must_use]
    pub fn capacity(self) -> usize {
        let current = match self.current {
            Some(current) => current,
            None => return 0,
        };

        let mut sum = current.capacity();
        current.iter_prev().for_each(|chunk| sum += chunk.capacity());
        current.iter_next().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns the amount of allocated bytes. This includes padding and wasted space due to reallocations.
    #[must_use]
    pub fn allocated(self) -> usize {
        let current = match self.current {
            Some(current) => current,
            None => return 0,
        };

        let mut sum = current.allocated();
        current.iter_prev().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns the total remaining capacity of all chunks.
    #[must_use]
    pub fn remaining(self) -> usize {
        let current = match self.current {
            Some(current) => current,
            None => return 0,
        };

        let mut sum = current.remaining();
        current.iter_next().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns an iterator from smallest to biggest chunk.
    #[must_use]
    pub fn small_to_big(self) -> ChunkNextIter<'a, UP> {
        let mut start = match self.current {
            Some(start) => start,
            None => return ChunkNextIter { chunk: None },
        };

        while let Some(chunk) = start.prev() {
            start = chunk;
        }

        ChunkNextIter { chunk: Some(start) }
    }

    /// Returns an iterator from biggest to smallest chunk.
    #[must_use]
    pub fn big_to_small(self) -> ChunkPrevIter<'a, UP> {
        let mut start = match self.current {
            Some(start) => start,
            None => return ChunkPrevIter { chunk: None },
        };

        while let Some(chunk) = start.next() {
            start = chunk;
        }

        ChunkPrevIter { chunk: Some(start) }
    }

    /// Converts this `Stats` into `Some(GuaranteedAllocatedStats)` or `None` if the current chunk is `None`.
    #[inline]
    #[must_use]
    pub fn to_guaranteed_allocated_stats(self) -> Option<GuaranteedAllocatedStats<'a, UP>> {
        self.current.map(|current| GuaranteedAllocatedStats { current })
    }

    pub(crate) fn debug_format(self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(stats) = self.to_guaranteed_allocated_stats() {
            stats.debug_format(name, f)
        } else {
            let alternate = f.alternate();
            let mut debug = f.debug_struct(name);

            const NONE: Option<()> = None::<()>;
            const EMPTY: &[()] = &[];

            if alternate {
                debug.field("current", &NONE);
                debug.field("chunks", &EMPTY);
            } else {
                debug.field("curr", &NONE);
            }

            debug.finish()
        }
    }
}

/// Refers to a chunk of memory that was allocated by the bump allocator.
///
/// See [`Stats`] / [`GuaranteedAllocatedStats`].
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a, const UP: bool> {
    pub(crate) chunk: RawChunk<UP, ()>,
    marker: PhantomData<&'a ()>,
}

impl<const UP: bool> Debug for Chunk<'_, UP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // low-level
            f.debug_struct("Chunk")
                .field("content_start", &self.content_start())
                .field("content_end", &self.content_end())
                .field("pos", &self.chunk.pos())
                .field("start", &self.chunk_start())
                .field("end", &self.chunk_end())
                .finish()
        } else {
            // high-level
            f.debug_struct("Chunk")
                .field("allocated", &self.allocated())
                .field("capacity", &self.capacity())
                .finish()
        }
    }
}

impl<'a, const UP: bool> Chunk<'a, UP> {
    #[inline(always)]
    pub(crate) fn new<'b, const MIN_ALIGN: usize, const GUARANTEED_ALLOCATED: bool, A>(
        bump: &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
    ) -> Option<Self>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        A: BaseAllocator<GUARANTEED_ALLOCATED>,
        'a: 'b,
    {
        if bump.is_unallocated() {
            return None;
        }

        Some(Self {
            chunk: bump.chunk.get().without_allocator(),
            marker: PhantomData,
        })
    }

    #[inline(always)]
    pub(crate) fn new_guaranteed_allocated<'b, const MIN_ALIGN: usize, A>(bump: &'b BumpScope<'a, A, MIN_ALIGN, UP>) -> Self
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        'a: 'b,
    {
        Self {
            chunk: bump.chunk.get().without_allocator(),
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn from_raw<A>(chunk: RawChunk<UP, A>) -> Self {
        Self {
            chunk: chunk.without_allocator(),
            marker: PhantomData,
        }
    }

    /// Returns the previous (smaller) chunk.
    #[must_use]
    #[inline(always)]
    pub fn prev(self) -> Option<Self> {
        self.chunk.prev().map(|c| unsafe { Chunk::from_raw(c) })
    }

    /// Returns the next (bigger) chunk.
    #[must_use]
    #[inline(always)]
    pub fn next(self) -> Option<Self> {
        self.chunk.next().map(|c| unsafe { Chunk::from_raw(c) })
    }

    /// Returns an iterator over all previous (smaller) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_prev(self) -> ChunkPrevIter<'a, UP> {
        ChunkPrevIter { chunk: self.prev() }
    }

    /// Returns an iterator over all next (bigger) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_next(self) -> ChunkNextIter<'a, UP> {
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

    /// Returns the amount of allocated bytes. This includes padding and wasted space due to reallocations.
    #[must_use]
    #[inline]
    pub fn allocated(self) -> usize {
        self.chunk.allocated()
    }

    /// Returns the remaining capacity.
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
    #[must_use]
    #[inline]
    pub fn bump_position(self) -> NonNull<u8> {
        self.chunk.pos()
    }
}

/// Iterator that iterates over previous chunks by continuously calling [`Chunk::prev`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ChunkPrevIter<'a, const UP: bool> {
    #[allow(missing_docs)]
    pub chunk: Option<Chunk<'a, UP>>,
}

impl<'a, const UP: bool> Iterator for ChunkPrevIter<'a, UP> {
    type Item = Chunk<'a, UP>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.prev();
        Some(chunk)
    }
}

impl<const UP: bool> FusedIterator for ChunkPrevIter<'_, UP> {}

impl<const UP: bool> Debug for ChunkPrevIter<'_, UP> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.map(Chunk::size)).finish()
    }
}

/// Iterator that iterates over next chunks by continuously calling [`Chunk::next`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ChunkNextIter<'a, const UP: bool> {
    #[allow(missing_docs)]
    pub chunk: Option<Chunk<'a, UP>>,
}

impl<'a, const UP: bool> Iterator for ChunkNextIter<'a, UP> {
    type Item = Chunk<'a, UP>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.next();
        Some(chunk)
    }
}

impl<const UP: bool> FusedIterator for ChunkNextIter<'_, UP> {}

impl<const UP: bool> Debug for ChunkNextIter<'_, UP> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.map(Chunk::size)).finish()
    }
}
