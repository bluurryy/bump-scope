use core::{alloc::Layout, fmt, marker::PhantomData};

use crate::{
    chunk_size_config::ChunkSizeConfig, chunk_types::r#abstract::FirstAllocatedChunk, settings::BumpAllocatorSettings,
};

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [usize; 2];

/// See [`ChunkSizeConfig::align_size`].
pub(crate) const fn align_allocation_size<A, S>(size: usize) -> usize
where
    S: BumpAllocatorSettings,
{
    Constants::<A, S>::CONFIG.align_size(size)
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
    fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSizeMinimum;

impl ChunkSize for ChunkSizeMinimum {
    fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings,
    {
        Some(Constants::<A, S>::MINIMUM_LAYOUT)
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
    fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings,
    {
        let config = Constants::<A, S>::CONFIG;
        let hint = max(self.0, S::MINIMUM_CHUNK_SIZE);
        let size = attempt!(config.calc_size_from_hint(hint)).get();
        let align = FirstAllocatedChunk::<A, S>::HEADER_LAYOUT.align();
        Layout::from_size_align(size, align).ok()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSizeCapacity(pub(crate) Layout);

impl ChunkSizeCapacity {
    pub(crate) const fn to_hint<A, S>(self) -> Option<ChunkSizeHint>
    where
        S: BumpAllocatorSettings,
    {
        Some(ChunkSizeHint(attempt!(
            Constants::<A, S>::CONFIG.calc_hint_from_capacity(self.0)
        )))
    }
}

impl ChunkSize for ChunkSizeCapacity {
    fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings,
    {
        self.to_hint::<A, S>()?.layout::<A, S>()
    }
}

struct Constants<A, S>(PhantomData<fn() -> (A, S)>);

impl<A, S> Constants<A, S>
where
    S: BumpAllocatorSettings,
{
    const CONFIG: ChunkSizeConfig = ChunkSizeConfig {
        up: S::UP,
        assumed_malloc_overhead_layout: Layout::new::<AssumedMallocOverhead>(),
        chunk_header_layout: FirstAllocatedChunk::<A, S>::HEADER_LAYOUT,
    };

    const MINIMUM_LAYOUT: Layout = {
        let size = match Self::CONFIG.calc_size_from_hint(S::MINIMUM_CHUNK_SIZE) {
            Some(some) => some.get(),
            None => panic!("failed to calculate minimum chunk size"),
        };

        match Layout::from_size_align(size, FirstAllocatedChunk::<A, S>::HEADER_LAYOUT.align()) {
            Ok(ok) => ok,
            Err(_) => panic!("failed to calculate minimum chunk layout"),
        }
    };
}

const fn max(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}
