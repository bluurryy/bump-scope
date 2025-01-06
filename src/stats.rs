use core::{
    fmt::{self, Debug},
    iter::FusedIterator,
    marker::PhantomData,
    ptr::NonNull,
};

use crate::{
    chunk_header::{unallocated_chunk_header, ChunkHeader},
    polyfill::nonnull,
    FmtFn, RawChunk,
};

/// Provides statistics about the memory usage of the bump allocator.
///
/// This is returned from the `stats` method of `Bump`, `BumpScope`, `BumpScopeGuard`, `BumpVec`, ...
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Stats<'a, const GUARANTEED_ALLOCATED: bool = false> {
    header: NonNull<ChunkHeader>,
    marker: PhantomData<&'a ()>,
}

impl<const GUARANTEED_ALLOCATED: bool> Debug for Stats<'_, GUARANTEED_ALLOCATED> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_format("Stats", f)
    }
}

impl<'a, const GUARANTEED_ALLOCATED: bool> Stats<'a, GUARANTEED_ALLOCATED> {
    /// Returns the number of chunks.
    #[must_use]
    pub fn count(self) -> usize {
        let current = match self.get_current_chunk() {
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
        let current = match self.get_current_chunk() {
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
        let current = match self.get_current_chunk() {
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
        let current = match self.get_current_chunk() {
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
        let current = match self.get_current_chunk() {
            Some(current) => current,
            None => return 0,
        };

        let mut sum = current.remaining();
        current.iter_next().for_each(|chunk| sum += chunk.capacity());
        sum
    }

    /// Returns an iterator from smallest to biggest chunk.
    #[must_use]
    pub fn small_to_big(self) -> ChunkNextIter<'a> {
        let mut start = match self.get_current_chunk() {
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
    pub fn big_to_small(self) -> ChunkPrevIter<'a> {
        let mut start = match self.get_current_chunk() {
            Some(start) => start,
            None => return ChunkPrevIter { chunk: None },
        };

        while let Some(chunk) = start.next() {
            start = chunk;
        }

        ChunkPrevIter { chunk: Some(start) }
    }

    pub(crate) fn debug_format(self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alternate = f.alternate();
        let mut debug = f.debug_struct(name);

        if let Some(current) = self.get_current_chunk() {
            fn fmt_list_compact<T: Debug>(iter: impl Iterator<Item = T> + Clone) -> impl Debug {
                let list = FmtFn(move |f| f.debug_list().entries(iter.clone()).finish());
                FmtFn(move |f| write!(f, "{list:?}"))
            }

            if alternate {
                // low-level, verbose
                let (index, first) = match current.iter_prev().enumerate().last() {
                    Some((i, chunk)) => (i + 1, chunk),
                    None => (0, current),
                };
                let chunks = FmtFn(move |f| f.debug_list().entries(ChunkNextIter { chunk: Some(first) }).finish());

                debug.field("current", &index);
                debug.field("chunks", &chunks);
            } else {
                // high-level
                debug.field("prev", &fmt_list_compact(current.iter_prev().map(Chunk::size)));
                debug.field("next", &fmt_list_compact(current.iter_next().map(Chunk::size)));
                debug.field("curr", &current);
            }

            debug.finish()
        } else {
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

    #[inline]
    pub(crate) unsafe fn from_header_unchecked(header: NonNull<ChunkHeader>) -> Self {
        if GUARANTEED_ALLOCATED {
            debug_assert_ne!(header, unallocated_chunk_header());
        }

        Self {
            header,
            marker: PhantomData,
        }
    }

    /// Turns this `Stats` into a `Stats` where `GUARANTEED_ALLOCATED = true`.
    #[inline]
    #[must_use]
    pub fn guaranteed_allocated(self) -> Option<Stats<'a, true>> {
        if GUARANTEED_ALLOCATED {
            return Some(unsafe { Stats::from_header_unchecked(self.header) });
        }

        if self.header == unallocated_chunk_header() {
            return None;
        }

        Some(unsafe { Stats::from_header_unchecked(self.header) })
    }

    /// Turns this `Stats` into a `Stats` where `GUARANTEED_ALLOCATED = false`.
    #[inline]
    #[must_use]
    pub fn not_guaranteed_allocated(self) -> Stats<'a, false> {
        unsafe { Stats::from_header_unchecked(self.header) }
    }

    fn get_current_chunk(self) -> Option<Chunk<'a>> {
        unsafe { Chunk::from_header(self.header) }
    }
}

impl<'a> Stats<'a, true> {
    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn current_chunk(self) -> Chunk<'a> {
        unsafe { Chunk::from_header_unchecked(self.header) }
    }
}

impl<'a> Stats<'a, false> {
    /// This is the chunk we are currently allocating on.
    #[must_use]
    pub fn current_chunk(self) -> Option<Chunk<'a>> {
        unsafe { Chunk::from_header(self.header) }
    }

    pub(crate) fn unallocated() -> Self {
        unsafe { Self::from_header_unchecked(unallocated_chunk_header()) }
    }
}

impl<'a, const GUARANTEED_ALLOCATED: bool> From<Chunk<'a>> for Stats<'a, GUARANTEED_ALLOCATED> {
    fn from(chunk: Chunk<'a>) -> Self {
        unsafe { Stats::from_header_unchecked(chunk.header) }
    }
}

impl Default for Stats<'_, false> {
    fn default() -> Self {
        Self::unallocated()
    }
}

/// Refers to a chunk of memory that was allocated by the bump allocator.
///
/// See [`Stats`].
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub(crate) header: NonNull<ChunkHeader>,
    marker: PhantomData<&'a ()>,
}

impl Debug for Chunk<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // low-level
            f.debug_struct("Chunk")
                .field("content_start", &self.content_start())
                .field("content_end", &self.content_end())
                .field("bump_position", &self.bump_position())
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

impl<'a> Chunk<'a> {
    #[inline]
    pub(crate) unsafe fn from_header(header: NonNull<ChunkHeader>) -> Option<Self> {
        if header == unallocated_chunk_header() {
            None
        } else {
            Some(unsafe { Self::from_header_unchecked(header) })
        }
    }

    #[inline]
    pub(crate) unsafe fn from_header_unchecked(header: NonNull<ChunkHeader>) -> Self {
        debug_assert_ne!(header, unallocated_chunk_header());
        Self {
            header,
            marker: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn is_upwards_allocating(self) -> bool {
        let header = nonnull::addr(self.header);
        let end = nonnull::addr(unsafe { self.header.as_ref().end });
        end > header
    }

    pub(crate) fn raw<const UP: bool>(self) -> RawChunk<UP, ()> {
        assert_eq!(UP, self.is_upwards_allocating());
        unsafe { RawChunk::from_header(self.header) }
    }

    /// Returns the previous (smaller) chunk.
    #[must_use]
    #[inline(always)]
    pub fn prev(self) -> Option<Self> {
        unsafe {
            Some(Chunk {
                header: self.header.as_ref().prev?,
                marker: PhantomData,
            })
        }
    }

    /// Returns the next (bigger) chunk.
    #[must_use]
    #[inline(always)]
    pub fn next(self) -> Option<Self> {
        unsafe {
            Some(Chunk {
                header: self.header.as_ref().next.get()?,
                marker: PhantomData,
            })
        }
    }

    /// Returns an iterator over all previous (smaller) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_prev(self) -> ChunkPrevIter<'a> {
        ChunkPrevIter { chunk: self.prev() }
    }

    /// Returns an iterator over all next (bigger) chunks.
    #[must_use]
    #[inline(always)]
    pub fn iter_next(self) -> ChunkNextIter<'a> {
        ChunkNextIter { chunk: self.next() }
    }

    /// Returns the size of this chunk in bytes.
    #[must_use]
    #[inline]
    pub fn size(self) -> usize {
        raw!(self.size().get())
    }

    /// Returns the capacity of this chunk in bytes.
    #[must_use]
    #[inline]
    pub fn capacity(self) -> usize {
        raw!(self.capacity())
    }

    /// Returns the amount of allocated bytes. This includes padding and wasted space due to reallocations.
    #[must_use]
    #[inline]
    pub fn allocated(self) -> usize {
        raw!(self.allocated())
    }

    /// Returns the remaining capacity.
    #[must_use]
    #[inline]
    pub fn remaining(self) -> usize {
        raw!(self.remaining())
    }

    /// Returns a pointer to the start of the chunk.
    #[must_use]
    #[inline]
    pub fn chunk_start(self) -> NonNull<u8> {
        raw!(self.chunk_start())
    }

    /// Returns a pointer to the end of the chunk.
    #[must_use]
    #[inline]
    pub fn chunk_end(self) -> NonNull<u8> {
        raw!(self.chunk_end())
    }

    /// Returns a pointer to the start of the chunk's content.
    #[must_use]
    #[inline]
    pub fn content_start(self) -> NonNull<u8> {
        raw!(self.content_start())
    }

    /// Returns a pointer to the end of the chunk's content.
    #[must_use]
    #[inline]
    pub fn content_end(self) -> NonNull<u8> {
        raw!(self.content_end())
    }

    /// Returns the bump pointer. It lies within the chunk's content range.
    #[must_use]
    #[inline]
    pub fn bump_position(self) -> NonNull<u8> {
        raw!(self.pos())
    }
}

/// Iterator that iterates over previous chunks by continuously calling [`Chunk::prev`].
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct ChunkPrevIter<'a> {
    #[allow(missing_docs)]
    pub chunk: Option<Chunk<'a>>,
}

impl<'a> Iterator for ChunkPrevIter<'a> {
    type Item = Chunk<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.prev();
        Some(chunk)
    }
}

impl FusedIterator for ChunkPrevIter<'_> {}

impl Debug for ChunkPrevIter<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.map(Chunk::size)).finish()
    }
}

/// Iterator that iterates over next chunks by continuously calling [`Chunk::next`].
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct ChunkNextIter<'a> {
    #[allow(missing_docs)]
    pub chunk: Option<Chunk<'a>>,
}

impl<'a> Iterator for ChunkNextIter<'a> {
    type Item = Chunk<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk?;
        self.chunk = chunk.next();
        Some(chunk)
    }
}

impl FusedIterator for ChunkNextIter<'_> {}

impl Debug for ChunkNextIter<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.map(Chunk::size)).finish()
    }
}

macro_rules! raw {
    ($self:ident $($tt:tt)*) => {
        if $self.is_upwards_allocating() {
            $self.raw::<true>() $($tt)*
        } else {
            $self.raw::<false>() $($tt)*
        }
    };
}

pub(crate) use raw;
