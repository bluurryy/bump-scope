#![forbid(unsafe_code)]
//! This file intentionally doesn't import anything other than `core`
//! to make it easy to fuzz and debug.

use core::{alloc::Layout, num::NonZeroUsize};

pub(crate) const ASSUMED_PAGE_SIZE: usize = 0x1000;
pub(crate) const MIN_CHUNK_ALIGN: usize = 16;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkSizeConfig {
    pub(crate) up: bool,
    pub(crate) chunk_header_layout: Layout,
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
    /// This aligning must not result in a size smaller than the size of the layout used for the allocation
    /// because we need the size to [fit] the allocated memory block.
    /// (The size must be between the original requested size and the size the allocator actually returned.)
    ///
    /// The `calc_size_from_hint` always returns a size that is a multiple of the chunk header alignment
    /// and `MIN_CHUNK_ALIGN`, so this downwards aligning cannot result in a size smaller than the requested one.
    ///
    /// [fit]: https://doc.rust-lang.org/std/alloc/trait.Allocator.html#memory-fitting
    #[inline(always)]
    pub(crate) const fn align_size(self, size: usize) -> usize {
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
    pub(crate) const fn calc_size_from_hint(self, size_hint: usize) -> Option<NonZeroUsize> {
        let Self { chunk_header_layout, .. } = self;

        let min = chunk_header_layout.pad_to_align().size();
        let size_step = max(ASSUMED_PAGE_SIZE, chunk_header_layout.align());
        let size_hint = max(size_hint, min);

        let size = attempt!(if size_hint < size_step {
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

        NonZeroUsize::new(size)
    }

    #[inline(always)]
    pub(crate) const fn calc_hint_from_capacity(self, layout: Layout) -> Option<usize> {
        let maximum_required_padding = layout.align().saturating_sub(MIN_CHUNK_ALIGN);
        let required_size = attempt!(layout.size().checked_add(maximum_required_padding));
        self.calc_hint_from_capacity_bytes(required_size)
    }

    #[inline(always)]
    pub(crate) const fn calc_hint_from_capacity_bytes(self, bytes: usize) -> Option<usize> {
        let Self {
            up, chunk_header_layout, ..
        } = self;

        let mut size = 0;

        if up {
            size = attempt!(offset_add_layout(size, chunk_header_layout));
            size = attempt!(size.checked_add(bytes));
        } else {
            size = attempt!(size.checked_add(bytes));
            size = attempt!(offset_add_layout(size, chunk_header_layout));
        }

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
