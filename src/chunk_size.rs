use core::{alloc::Layout, marker::PhantomData, num::NonZeroUsize};

use chunk_size_calc::ChunkLayoutConfig;

use crate::{polyfill::const_unwrap, ChunkHeader};

mod chunk_size_calc;

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [*const u8; 2];

pub const fn config<A, const UP: bool>() -> ChunkLayoutConfig {
    ChunkLayoutConfig {
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
    pub const DEFAULT_START: Self = const_unwrap(Self::for_size_hint(512));

    pub const fn for_size_hint(size_hint: usize) -> Option<Self> {
        Some(Self {
            size: attempt!(config::<A, UP>().calculate_for_size_hint(size_hint)),
            marker: PhantomData,
        })
    }

    pub const fn for_capacity(layout: Layout) -> Option<Self> {
        Some(Self {
            size: attempt!(config::<A, UP>().calculate_for_capacity(layout)),
            marker: PhantomData,
        })
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
        if self.size.get() > other.size.get() {
            self
        } else {
            other
        }
    }
}
