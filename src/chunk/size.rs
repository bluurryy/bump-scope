use core::{alloc::Layout, marker::PhantomData};

use crate::{
    chunk::{AllocatedChunkHeader, ChunkSizeConfig, MIN_CHUNK_ALIGN},
    settings::BumpAllocatorSettings,
};

const _: () = assert!(MIN_CHUNK_ALIGN == crate::bumping::MIN_CHUNK_ALIGN);

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [usize; 2];

/// See [`ChunkSizeConfig::align_size`].
pub const fn align_allocation_size<A, S>(size: usize) -> usize
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

pub trait ChunkSize: Copy {
    fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings;
}

#[derive(Clone, Copy)]
pub struct ChunkSizeMinimum;

impl ChunkSize for ChunkSizeMinimum {
    fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings,
    {
        Some(Constants::<A, S>::MINIMUM_LAYOUT)
    }
}

#[derive(Clone, Copy)]
pub struct ChunkSizeHint(pub usize);

impl ChunkSizeHint {
    pub const fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings,
    {
        let hint = max(self.0, S::MINIMUM_CHUNK_SIZE);
        let size = attempt!(Constants::<A, S>::CONFIG.calc_size_from_hint(hint)).get();
        let align = core::mem::align_of::<AllocatedChunkHeader<A>>();
        match Layout::from_size_align(size, align) {
            Ok(ok) => Some(ok),
            Err(_) => None,
        }
    }

    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 { self } else { other }
    }
}

impl ChunkSize for ChunkSizeHint {
    fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings,
    {
        self.layout::<A, S>()
    }
}

#[derive(Clone, Copy)]
pub struct ChunkSizeCapacity(pub Layout);

impl ChunkSizeCapacity {
    pub const fn to_hint<A, S>(self) -> Option<ChunkSizeHint>
    where
        S: BumpAllocatorSettings,
    {
        Some(ChunkSizeHint(attempt!(
            Constants::<A, S>::CONFIG.calc_hint_from_capacity(self.0)
        )))
    }

    pub const fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings,
    {
        attempt!(self.to_hint::<A, S>()).layout::<A, S>()
    }
}

impl ChunkSize for ChunkSizeCapacity {
    fn layout<A, S>(self) -> Option<Layout>
    where
        S: BumpAllocatorSettings,
    {
        self.layout::<A, S>()
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
        chunk_header_layout: Layout::new::<AllocatedChunkHeader<A>>(),
    };

    const MINIMUM_LAYOUT: Layout = {
        let size = match Self::CONFIG.calc_size_from_hint(S::MINIMUM_CHUNK_SIZE) {
            Some(some) => some.get(),
            None => panic!("failed to calculate minimum chunk size"),
        };

        let align = core::mem::align_of::<AllocatedChunkHeader<A>>();

        match Layout::from_size_align(size, align) {
            Ok(ok) => ok,
            Err(_) => panic!("failed to calculate minimum chunk layout"),
        }
    };
}

const fn max(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}
