#![forbid(unsafe_code)]
//! This file intentionally doesn't import anything other than `core`
//! to make it easy to fuzz and debug.

use core::{alloc::Layout, num::NonZeroUsize};

pub const ASSUMED_PAGE_SIZE: usize = 0x1000;
pub const MIN_CHUNK_ALIGN: usize = 16;

#[derive(Clone, Copy)]
pub struct ChunkSizeConfig {
    pub up: bool,
    pub assumed_malloc_overhead_layout: Layout,
    pub chunk_header_layout: Layout,
}

/// Alternative to using the `?` operator that works in const functions.
macro_rules! attempt {
    ($expr:expr) => {
        match $expr {
            Some(some) => some,
            None => return None,
        }
    };
}

impl ChunkSizeConfig {
    /// This function must be called to align the size of the allocation returned
    /// by the allocator.
    ///
    /// # Context
    /// The final chunk size must always be a multiple of `MIN_CHUNK_ALIGN`.
    /// We use optimizations in `alloc` that make use of that invariant.
    ///
    /// When downwards allocating, the final chunk size must also be aligned to the chunk header
    /// alignment so we can put the header at `ptr.byte_add(size).cast::<ChunkHeader<A>>().sub(1)`.
    ///
    /// This aligning must not result in a size smaller than the layout's size
    /// because we need `size` to be a size that [fits] the allocated memory block.
    /// (The size must be between the original requested size and the size the allocator actually returned.)
    ///
    /// This is ensured by already having `align_size`'d the layout's size in `calc_size_from_hint`.
    /// A downwards alignment of a size greater or equal than the layout's size, which
    /// the allocation's size is, cannot result in a size smaller than the layout's.
    ///
    /// [fits]: https://doc.rust-lang.org/std/alloc/trait.Allocator.html#memory-fitting
    #[inline(always)]
    pub const fn align_size(self, size: usize) -> usize {
        let Self {
            up, chunk_header_layout, ..
        } = self;

        down_align(
            size,
            if up {
                MIN_CHUNK_ALIGN
            } else {
                max(MIN_CHUNK_ALIGN, chunk_header_layout.align())
            },
        )
    }

    #[inline(always)]
    pub const fn calc_size_from_hint(self, size_hint: usize) -> Option<NonZeroUsize> {
        let Self {
            assumed_malloc_overhead_layout,
            chunk_header_layout,
            ..
        } = self;

        let min = {
            let mut offset = 0;
            offset = attempt!(offset_add_layout(offset, assumed_malloc_overhead_layout));
            offset = attempt!(offset_add_layout(offset, chunk_header_layout));
            offset
        };

        let size_step = max(ASSUMED_PAGE_SIZE, chunk_header_layout.align());
        let size_hint = max(size_hint, min);

        let mut size = attempt!(if size_hint < size_step {
            // the name is misleading, this will return `size` if it is already a power of two
            size_hint.checked_next_power_of_two()
        } else {
            up_align(size_hint, size_step)
        });

        debug_assert!(size % chunk_header_layout.align() == 0);
        debug_assert!(size >= min);

        debug_assert!(if size < size_step {
            size.is_power_of_two()
        } else {
            size % size_step == 0
        });

        // When downwards allocating with a base allocator that has a higher alignment than 16
        // we don't subtract an assumed malloc overhead.
        //
        // Since we also need to align the size after subtraction we could end up with a size of `0`,
        // which would be illegal since the chunk header would not fit, or we could end up with a
        // size that would no longer have enough space for the given capacity (`calc_hint_from_capacity`).
        if self.up || self.chunk_header_layout.align() <= MIN_CHUNK_ALIGN {
            let size_without_overhead = size - assumed_malloc_overhead_layout.size();
            size = self.align_size(size_without_overhead);
        }

        NonZeroUsize::new(size)
    }

    #[inline(always)]
    pub const fn calc_hint_from_capacity(self, layout: Layout) -> Option<usize> {
        let Self { chunk_header_layout, .. } = self;

        let maximum_required_padding = layout.align().saturating_sub(chunk_header_layout.align());
        let required_size = attempt!(layout.size().checked_add(maximum_required_padding));
        self.calc_hint_from_capacity_bytes(required_size)
    }

    #[inline(always)]
    pub const fn calc_hint_from_capacity_bytes(self, bytes: usize) -> Option<usize> {
        let Self {
            up,
            assumed_malloc_overhead_layout,
            chunk_header_layout,
            ..
        } = self;

        let mut size = 0;

        if up {
            size = attempt!(offset_add_layout(size, assumed_malloc_overhead_layout));
            size = attempt!(offset_add_layout(size, chunk_header_layout));
            size = attempt!(size.checked_add(bytes));
        } else {
            size = attempt!(offset_add_layout(size, assumed_malloc_overhead_layout));
            size = attempt!(size.checked_add(bytes));
            size = attempt!(offset_add_layout(size, chunk_header_layout));
        }

        // The final size will be aligned to `MIN_CHUNK_ALIGN = 16`.
        // To make sure aligning the size downwards does not result in
        // a size that does not have space for `bytes` we add `MIN_CHUNK_ALIGN` here.
        size = attempt!(size.checked_add(MIN_CHUNK_ALIGN));

        Some(size)
    }
}

const fn max(lhs: usize, rhs: usize) -> usize {
    if lhs > rhs { lhs } else { rhs }
}

const fn offset_add_layout(mut offset: usize, layout: Layout) -> Option<usize> {
    offset = attempt!(up_align(offset, layout.align()));
    offset = attempt!(offset.checked_add(layout.size()));
    Some(offset)
}

#[inline(always)]
const fn up_align(addr: usize, align: usize) -> Option<usize> {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    let addr_plus_mask = attempt!(addr.checked_add(mask));
    let aligned = addr_plus_mask & !mask;
    Some(aligned)
}

#[inline(always)]
const fn down_align(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}
