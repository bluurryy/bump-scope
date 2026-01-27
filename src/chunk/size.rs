use core::{alloc::Layout, marker::PhantomData, num::NonZeroUsize};

use crate::{
    chunk::{ChunkHeader, ChunkSizeConfig, MIN_CHUNK_ALIGN},
    settings::Boolean,
};

const _: () = assert!(MIN_CHUNK_ALIGN == crate::bumping::MIN_CHUNK_ALIGN);

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [*const u8; 2];

pub const fn config<Up>() -> ChunkSizeConfig
where
    Up: Boolean,
{
    ChunkSizeConfig {
        up: Up::VALUE,
        assumed_malloc_overhead_layout: Layout::new::<AssumedMallocOverhead>(),
        chunk_header_layout: Layout::new::<ChunkHeader>(),
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

pub struct ChunkSize<Up> {
    size: NonZeroUsize,
    marker: PhantomData<fn() -> Up>,
}

impl<Up> Clone for ChunkSize<Up> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Up> Copy for ChunkSize<Up> {}

impl<Up> ChunkSize<Up>
where
    Up: Boolean,
{
    pub const fn from_hint(size_hint: usize) -> Option<Self> {
        ChunkSizeHint::new(size_hint).calc_size()
    }

    pub const fn from_capacity(layout: Layout) -> Option<Self> {
        attempt!(ChunkSizeHint::for_capacity(layout)).calc_size()
    }

    /// See [`chunk_size_config::ChunkSizeConfig::align_size`].
    pub const fn align_allocation_size(self, size: usize) -> usize {
        _ = self;
        config::<Up>().align_size(size)
    }

    pub const fn layout(self) -> Option<Layout> {
        let size = self.size.get();
        let align = core::mem::align_of::<ChunkHeader>();
        match Layout::from_size_align(size, align) {
            Ok(ok) => Some(ok),
            Err(_) => None,
        }
    }
}
pub struct ChunkSizeHint<Up>(usize, PhantomData<fn() -> Up>);

impl<Up> Clone for ChunkSizeHint<Up> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Up> Copy for ChunkSizeHint<Up> {}

impl<Up> ChunkSizeHint<Up>
where
    Up: Boolean,
{
    pub const fn new(size_hint: usize) -> Self {
        Self(size_hint, PhantomData)
    }

    pub const fn for_capacity(layout: Layout) -> Option<Self> {
        Some(Self(attempt!(config::<Up>().calc_hint_from_capacity(layout)), PhantomData))
    }

    pub const fn calc_size(self) -> Option<ChunkSize<Up>> {
        Some(ChunkSize {
            size: attempt!(config::<Up>().calc_size_from_hint(self.0)),
            marker: PhantomData,
        })
    }

    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 { self } else { other }
    }
}
