use crate::{
    chunk_header::ChunkHeader, polyfill::nonnull, BaseAllocator, BumpScope, FmtFn, MinimumAlignment, RawChunk,
    SupportedMinimumAlignment,
};
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
pub struct GuaranteedAllocatedStats<'a> {
    /// This is the chunk we are currently allocating on.
    pub current: Chunk<'a>,
}

impl Copy for GuaranteedAllocatedStats<'_> {}

#[allow(clippy::expl_impl_clone_on_copy)]
impl Clone for GuaranteedAllocatedStats<'_> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl PartialEq for GuaranteedAllocatedStats<'_> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
    }

    #[inline(always)]
    fn ne(&self, other: &Self) -> bool {
        self.current != other.current
    }
}

impl Eq for GuaranteedAllocatedStats<'_> {}

impl Debug for GuaranteedAllocatedStats<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_format("GuaranteedAllocatedStats", f)
    }
}

impl<'a> GuaranteedAllocatedStats<'a> {
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
    pub fn small_to_big(self) -> ChunkNextIter<'a> {
        let mut start = self.current;

        while let Some(chunk) = start.prev() {
            start = chunk;
        }

        ChunkNextIter { chunk: Some(start) }
    }

    /// Returns an iterator from biggest to smallest chunk.
    #[must_use]
    pub fn big_to_small(self) -> ChunkPrevIter<'a> {
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
pub struct Stats<'a> {
    /// This is the chunk we are currently allocating on.
    pub current: Option<Chunk<'a>>,
}

impl Copy for Stats<'_> {}

#[allow(clippy::expl_impl_clone_on_copy)]
impl Clone for Stats<'_> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl PartialEq for Stats<'_> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
    }

    #[inline(always)]
    fn ne(&self, other: &Self) -> bool {
        self.current != other.current
    }
}

impl Eq for Stats<'_> {}

impl Debug for Stats<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_format("Stats", f)
    }
}

impl<'a> From<GuaranteedAllocatedStats<'a>> for Stats<'a> {
    fn from(value: GuaranteedAllocatedStats<'a>) -> Self {
        Stats {
            current: Some(value.current),
        }
    }
}

impl<'a> Stats<'a> {
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
    pub fn small_to_big(self) -> ChunkNextIter<'a> {
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
    pub fn big_to_small(self) -> ChunkPrevIter<'a> {
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
    pub fn to_guaranteed_allocated_stats(self) -> Option<GuaranteedAllocatedStats<'a>> {
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
pub struct Chunk<'a> {
    pub(crate) chunk: NonNull<ChunkHeader<()>>,
    marker: PhantomData<&'a ()>,
}

impl Debug for Chunk<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // low-level
            f.debug_struct("Chunk")
                .field("content_start", &self.content_start())
                .field("content_end", &self.content_end())
                .field("pos", &self.bump_position())
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
    pub(crate) fn is_upwards_allocating(self) -> bool {
        let header = nonnull::addr(self.chunk);
        let end = nonnull::addr(unsafe { self.chunk.as_ref().end });
        end > header
    }

    pub(crate) fn raw<const UP: bool>(self) -> RawChunk<UP, ()> {
        debug_assert_eq!(UP, self.is_upwards_allocating());
        unsafe { RawChunk::from_header(self.chunk) }
    }

    #[inline(always)]
    pub(crate) fn new<'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>(
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
            chunk: bump.chunk.get().header_ptr().cast(),
            marker: PhantomData,
        })
    }

    #[inline(always)]
    pub(crate) fn new_guaranteed_allocated<'b, const MIN_ALIGN: usize, const UP: bool, A>(
        bump: &'b BumpScope<'a, A, MIN_ALIGN, UP>,
    ) -> Self
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        'a: 'b,
    {
        Self {
            chunk: bump.chunk.get().header_ptr().cast(),
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn from_raw<const UP: bool, A>(chunk: RawChunk<UP, A>) -> Self {
        Self {
            chunk: chunk.header_ptr().cast(),
            marker: PhantomData,
        }
    }

    /// Returns the previous (smaller) chunk.
    #[must_use]
    #[inline(always)]
    pub fn prev(self) -> Option<Self> {
        unsafe {
            Some(Chunk {
                chunk: self.chunk.as_ref().prev?,
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
                chunk: self.chunk.as_ref().next.get()?,
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
#[derive(Clone, Copy, PartialEq, Eq)]
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
#[derive(Clone, Copy, PartialEq, Eq)]
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
