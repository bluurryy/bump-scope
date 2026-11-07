use core::{alloc::Layout, fmt, marker::PhantomData};

use crate::{
    chunk_size_config::ChunkSizeConfig,
    chunk_types::AllocatedChunk,
    config::{BumpConfig, config},
};

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [usize; 2];

/// See [`ChunkSizeConfig::align_size`].
pub(crate) const fn align_allocation_size<const C: BumpConfig, A>(size: usize) -> usize {
    Constants::<C, A>::CONFIG.align_size(size)
}

macro_rules! attempt {
    ($expr:expr) => {
        match $expr {
            Some(some) => some,
            None => return None,
        }
    };
}

pub(crate) trait ChunkSize: Copy + fmt::Debug {
    fn layout<const C: BumpConfig, A>(self) -> Option<Layout>;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSizeMin;

impl ChunkSize for ChunkSizeMin {
    fn layout<const C: BumpConfig, A>(self) -> Option<Layout> {
        Some(Constants::<C, A>::MINIMUM_LAYOUT)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSizeHint(pub(crate) usize);

impl ChunkSizeHint {
    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 { self } else { other }
    }
}

impl ChunkSize for ChunkSizeHint {
    fn layout<const C: BumpConfig, A>(self) -> Option<Layout> {
        let config = Constants::<C, A>::CONFIG;
        let hint = max(self.0, config!(min_chunk_size));
        let size = attempt!(config.calc_size_from_hint(hint)).get();
        let align = AllocatedChunk::<C>::HEADER_LAYOUT.align();
        Layout::from_size_align(size, align).ok()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSizeCapacity(pub(crate) Layout);

impl ChunkSizeCapacity {
    pub(crate) const fn to_hint<const C: BumpConfig, A>(self) -> Option<ChunkSizeHint> {
        Some(ChunkSizeHint(attempt!(
            Constants::<C, A>::CONFIG.calc_hint_from_capacity(self.0)
        )))
    }
}

impl ChunkSize for ChunkSizeCapacity {
    fn layout<const C: BumpConfig, A>(self) -> Option<Layout> {
        self.to_hint::<C, A>()?.layout::<C, A>()
    }
}

struct Constants<const C: BumpConfig, A>(PhantomData<fn() -> A>);

impl<const C: BumpConfig, A> Constants<C, A> {
    const CONFIG: ChunkSizeConfig = ChunkSizeConfig {
        up: config!(upwards),
        assumed_malloc_overhead_layout: Layout::new::<AssumedMallocOverhead>(),
        chunk_header_layout: AllocatedChunk::<C>::HEADER_LAYOUT,
    };

    const MINIMUM_LAYOUT: Layout = {
        let size = match Self::CONFIG.calc_size_from_hint(config!(min_chunk_size)) {
            Some(some) => some.get(),
            None => panic!("failed to calculate minimum chunk size"),
        };

        match Layout::from_size_align(size, AllocatedChunk::<C>::HEADER_LAYOUT.align()) {
            Ok(ok) => ok,
            Err(_) => panic!("failed to calculate minimum chunk layout"),
        }
    };
}

const fn max(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}
