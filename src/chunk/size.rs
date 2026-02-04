use core::{alloc::Layout, marker::PhantomData, num::NonZeroUsize};

use crate::{
    chunk::{ChunkHeader, ChunkSizeConfig, MIN_CHUNK_ALIGN},
    settings::Boolean,
};

const _: () = assert!(MIN_CHUNK_ALIGN == crate::bumping::MIN_CHUNK_ALIGN);

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [usize; 2];

pub const fn config<A, Up>() -> ChunkSizeConfig
where
    Up: Boolean,
{
    ChunkSizeConfig {
        up: Up::VALUE,
        assumed_malloc_overhead_layout: Layout::new::<AssumedMallocOverhead>(),
        chunk_header_layout: Layout::new::<ChunkHeader<A>>(),
    }
}

macro_rules! attempt {
    ($expr:expr) => {
        match $expr {
            Some(some) => some,
            None => return None,
        }
    };
}

pub struct ChunkSize<A, Up> {
    size: NonZeroUsize,
    marker: PhantomData<fn() -> (A, Up)>,
}

impl<A, Up> Clone for ChunkSize<A, Up> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, Up> Copy for ChunkSize<A, Up> {}

impl<A, Up> ChunkSize<A, Up>
where
    Up: Boolean,
{
    pub const ZERO: Self = match Self::from_hint(0) {
        Some(some) => some,
        None => panic!("failed to calculate zero chunk size"),
    };

    pub const fn from_hint(size_hint: usize) -> Option<Self> {
        ChunkSizeHint::new(size_hint).calc_size()
    }

    pub const fn from_capacity(layout: Layout) -> Option<Self> {
        attempt!(ChunkSizeHint::for_capacity(layout)).calc_size()
    }

    /// See [`chunk_size_config::ChunkSizeConfig::align_size`].
    pub const fn align_allocation_size(self, size: usize) -> usize {
        _ = self;
        config::<A, Up>().align_size(size)
    }

    pub const fn layout(self) -> Option<Layout> {
        let size = self.size.get();
        let align = core::mem::align_of::<ChunkHeader<A>>();
        match Layout::from_size_align(size, align) {
            Ok(ok) => Some(ok),
            Err(_) => None,
        }
    }

    pub const fn max(self, other: Self) -> Self {
        if self.size.get() > other.size.get() { self } else { other }
    }
}
pub struct ChunkSizeHint<A, Up>(usize, PhantomData<fn() -> (A, Up)>);

impl<A, Up> Clone for ChunkSizeHint<A, Up> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, Up> Copy for ChunkSizeHint<A, Up> {}

impl<A, Up> ChunkSizeHint<A, Up>
where
    Up: Boolean,
{
    pub const fn new(size_hint: usize) -> Self {
        Self(size_hint, PhantomData)
    }

    pub const fn for_capacity(layout: Layout) -> Option<Self> {
        Some(Self(attempt!(config::<A, Up>().calc_hint_from_capacity(layout)), PhantomData))
    }

    pub const fn calc_size(self) -> Option<ChunkSize<A, Up>> {
        Some(ChunkSize {
            size: attempt!(config::<A, Up>().calc_size_from_hint(self.0)),
            marker: PhantomData,
        })
    }

    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 { self } else { other }
    }
}
