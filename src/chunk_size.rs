use core::{
    alloc::Layout,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    mem::{align_of, size_of},
    num::NonZeroUsize,
};

use chunk_size_calc::ChunkSizeConfig;

use crate::{down_align_usize, polyfill::const_unwrap, ChunkHeader, CHUNK_ALIGN_MIN};

mod chunk_size_calc;

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [*const u8; 2];

/// The actual size used for allocation (see [`layout`](Self::layout)) is this size minus <code>size_of::<[AssumedMallocOverhead]>()</code>.
///
/// Invariants:
/// - is never zero
/// - is a multiple of <code>align_of::<[`ChunkHeader<A>`](ChunkHeader)>()</code>.
/// - is at least [`Self::MIN`]
/// - if smaller than [`Self::SIZE_STEP`] it is a power of two
/// - if larger than [`Self::SIZE_STEP`] it is a multiple of [`Self::SIZE_STEP`]
pub(crate) struct ChunkSize<const UP: bool, A>(pub(crate) NonZeroUsize, PhantomData<*const A>);

impl<const UP: bool, A> Debug for ChunkSize<UP, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.0.get(), f)
    }
}

impl<const UP: bool, A> Clone for ChunkSize<UP, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const UP: bool, A> Copy for ChunkSize<UP, A> {}

macro_rules! attempt {
    ($expr:expr) => {
        match $expr {
            Some(some) => some,
            None => return None,
        }
    };
}

impl<const UP: bool, A> ChunkSize<UP, A> {
    pub(crate) const DEFAULT_START: Self = const_unwrap(Self::new(512));

    const CONFIG: ChunkSizeConfig = ChunkSizeConfig {
        up: UP,
        assumed_malloc_overhead_layout: Layout::new::<AssumedMallocOverhead>(),
        chunk_header_layout: Layout::new::<ChunkHeader<A>>(),
    };

    #[inline]
    pub(crate) const fn new(size_hint: usize) -> Option<Self> {
        let size = attempt!(Self::CONFIG.calculate_for_size_hint(size_hint));
        let size = attempt!(NonZeroUsize::new(size));
        Some(Self(size, PhantomData))
    }

    #[inline]
    pub(crate) fn for_capacity(layout: Layout) -> Option<Self> {
        let size = attempt!(Self::CONFIG.calculate_for_capacity(layout));
        let size = attempt!(NonZeroUsize::new(size));
        Some(Self(size, PhantomData))
    }

    #[inline(always)]
    pub(crate) fn layout(self) -> Layout {
        // we checked in `new` that we can create a layout from this size

        let size_without_overhead = self.0.get() - size_of::<AssumedMallocOverhead>();

        let downwards_align = if UP {
            CHUNK_ALIGN_MIN
        } else {
            CHUNK_ALIGN_MIN.max(align_of::<ChunkHeader<A>>())
        };

        let size_for_layout = down_align_usize(size_without_overhead, downwards_align);
        let align = align_of::<ChunkHeader<A>>();

        unsafe { Layout::from_size_align_unchecked(size_for_layout, align) }
    }

    #[inline]
    pub(crate) const fn max(self, other: Self) -> Self {
        if self.0.get() > other.0.get() {
            self
        } else {
            other
        }
    }
}
