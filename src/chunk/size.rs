use core::{alloc::Layout, marker::PhantomData, num::NonZeroUsize};

use crate::{
    chunk::{ChunkHeader, ChunkSizeConfig, MIN_CHUNK_ALIGN},
    settings::BumpAllocatorSettings,
};

const _: () = assert!(MIN_CHUNK_ALIGN == crate::bumping::MIN_CHUNK_ALIGN);

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [*const u8; 2];

pub const fn config<A, S>() -> ChunkSizeConfig
where
    S: BumpAllocatorSettings,
{
    ChunkSizeConfig {
        up: S::UP,
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

pub struct ChunkSize<A, S> {
    size: NonZeroUsize,
    marker: PhantomData<fn() -> (A, S)>,
}

impl<A, S> Clone for ChunkSize<A, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for ChunkSize<A, S> {}

impl<A, S> ChunkSize<A, S>
where
    S: BumpAllocatorSettings,
{
    pub const DEFAULT: Self = ChunkSizeHint::DEFAULT.calc_size().unwrap();

    pub const fn from_hint(size_hint: usize) -> Option<Self> {
        ChunkSizeHint::new(size_hint).calc_size()
    }

    pub const fn from_capacity(layout: Layout) -> Option<Self> {
        attempt!(ChunkSizeHint::for_capacity(layout)).calc_size()
    }

    /// See [`chunk_size_config::ChunkSizeConfig::align_size`].
    pub const fn align_allocation_size(self, size: usize) -> usize {
        _ = self;
        config::<A, S>().align_size(size)
    }

    pub const fn layout(self) -> Option<Layout> {
        let size = self.size.get();
        let align = core::mem::align_of::<ChunkHeader<A>>();
        match Layout::from_size_align(size, align) {
            Ok(ok) => Some(ok),
            Err(_) => None,
        }
    }
}
pub struct ChunkSizeHint<A, S>(usize, PhantomData<fn() -> (A, S)>);

impl<A, S> Clone for ChunkSizeHint<A, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for ChunkSizeHint<A, S> {}

impl<A, S> ChunkSizeHint<A, S>
where
    S: BumpAllocatorSettings,
{
    pub const DEFAULT: Self = Self::new(512);

    pub const fn new(size_hint: usize) -> Self {
        Self(size_hint, PhantomData)
    }

    pub const fn for_capacity(layout: Layout) -> Option<Self> {
        Some(Self(attempt!(config::<A, S>().calc_hint_from_capacity(layout)), PhantomData))
    }

    pub const fn calc_size(self) -> Option<ChunkSize<A, S>> {
        Some(ChunkSize {
            size: attempt!(config::<A, S>().calc_size_from_hint(self.0)),
            marker: PhantomData,
        })
    }

    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 { self } else { other }
    }
}
