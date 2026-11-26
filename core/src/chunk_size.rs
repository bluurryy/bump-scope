use core::{alloc::Layout, fmt, marker::PhantomData};

use crate::{
    chunk_size_config::ChunkSizeConfig,
    chunk_types::AllocatedChunk,
    config::{BumpConfig, config},
};

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

pub(crate) trait ChunkLayout: Copy + fmt::Debug {
    fn layout<const C: BumpConfig, A>(self) -> Option<Layout>;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSizeDefault;

impl ChunkLayout for ChunkSizeDefault {
    fn layout<const C: BumpConfig, A>(self) -> Option<Layout> {
        Some(Constants::<C, A>::DEFAULT_LAYOUT)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSize(pub(crate) usize);

impl ChunkSize {
    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 { self } else { other }
    }
}

impl ChunkLayout for ChunkSize {
    fn layout<const C: BumpConfig, A>(self) -> Option<Layout> {
        let config = Constants::<C, A>::CONFIG;
        let hint = self.0;
        let size = attempt!(config.calc_layout_size(hint)).get();
        let align = AllocatedChunk::<C>::HEADER_LAYOUT.align();
        Layout::from_size_align(size, align).ok()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSizeCapacity(pub(crate) Layout);

impl ChunkSizeCapacity {
    pub(crate) const fn to_hint<const C: BumpConfig, A>(self) -> Option<ChunkSize> {
        Some(ChunkSize(attempt!(Constants::<C, A>::CONFIG.calc_size_from_capacity(self.0))))
    }
}

impl ChunkLayout for ChunkSizeCapacity {
    fn layout<const C: BumpConfig, A>(self) -> Option<Layout> {
        self.to_hint::<C, A>()?.layout::<C, A>()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSizeCapacityFirst(pub(crate) Layout);

impl ChunkLayout for ChunkSizeCapacityFirst {
    fn layout<const C: BumpConfig, A>(self) -> Option<Layout> {
        let config = Constants::<C, A>::CONFIG;
        let hint =
            attempt!(Constants::<C, A>::CONFIG.calc_size_from_capacity(self.0)).max(config!(default_initial_chunk_size));
        let size = attempt!(config.calc_layout_size(hint)).get();
        let align = AllocatedChunk::<C>::HEADER_LAYOUT.align();
        Layout::from_size_align(size, align).ok()
    }
}

struct Constants<const C: BumpConfig, A>(PhantomData<fn() -> A>);

impl<const C: BumpConfig, A> Constants<C, A> {
    const CONFIG: ChunkSizeConfig = ChunkSizeConfig {
        up: config!(upwards),
        chunk_header_layout: AllocatedChunk::<C>::HEADER_LAYOUT,
    };

    const DEFAULT_LAYOUT: Layout = {
        let size = match Self::CONFIG.calc_layout_size(config!(default_initial_chunk_size)) {
            Some(some) => some.get(),
            None => panic!("failed to calculate minimum chunk size"),
        };

        match Layout::from_size_align(size, AllocatedChunk::<C>::HEADER_LAYOUT.align()) {
            Ok(ok) => ok,
            Err(_) => panic!("failed to calculate minimum chunk layout"),
        }
    };
}
