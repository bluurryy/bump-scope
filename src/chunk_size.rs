use core::{alloc::Layout, marker::PhantomData, num::NonZeroUsize};

use chunk_size_calc::ChunkSizeConfig;

use crate::{polyfill::const_unwrap, ChunkHeader};

mod chunk_size_calc;

const _: () = assert!(chunk_size_calc::MIN_CHUNK_ALIGN == crate::bumping::MIN_CHUNK_ALIGN);

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [*const u8; 2];

pub const fn config<A, const UP: bool>() -> ChunkSizeConfig {
    ChunkSizeConfig {
        up: UP,
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

pub struct ChunkSize<A, const UP: bool> {
    size: NonZeroUsize,
    marker: PhantomData<*const A>,
}

impl<A, const UP: bool> Clone for ChunkSize<A, UP> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const UP: bool> Copy for ChunkSize<A, UP> {}

impl<A, const UP: bool> ChunkSize<A, UP> {
    pub const DEFAULT: Self = const_unwrap(ChunkSizeHint::DEFAULT.calc_size());

    pub const fn from_hint(size_hint: usize) -> Option<Self> {
        ChunkSizeHint::new(size_hint).calc_size()
    }

    pub const fn from_capacity(layout: Layout) -> Option<Self> {
        attempt!(ChunkSizeHint::from_capacity(layout)).calc_size()
    }

    /// See [`chunk_size_calc::ChunkSizeConfig::align_size`].
    pub const fn align_allocation_len(self, size: usize) -> usize {
        config::<A, UP>().align_size(size)
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
pub struct ChunkSizeHint<A, const UP: bool>(usize, PhantomData<*const A>);

impl<A, const UP: bool> Clone for ChunkSizeHint<A, UP> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const UP: bool> Copy for ChunkSizeHint<A, UP> {}

impl<A, const UP: bool> ChunkSizeHint<A, UP> {
    pub const DEFAULT: Self = Self::new(512);

    pub const fn new(size_hint: usize) -> Self {
        Self(size_hint, PhantomData)
    }

    pub const fn from_capacity(layout: Layout) -> Option<Self> {
        Some(Self(attempt!(config::<A, UP>().calc_hint_from_capacity(layout)), PhantomData))
    }

    pub const fn calc_size(self) -> Option<ChunkSize<A, UP>> {
        Some(ChunkSize {
            size: attempt!(config::<A, UP>().calc_size_from_hint(self.0)),
            marker: PhantomData,
        })
    }

    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 {
            self
        } else {
            other
        }
    }
}
