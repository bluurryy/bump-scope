use core::{alloc::Layout, num::NonZeroUsize};

const ASSUMED_PAGE_SIZE: usize = 0x1000;
const MIN_CHUNK_ALIGN: usize = 16;

#[derive(Clone, Copy)]
pub struct ChunkLayoutConfig {
    pub up: bool,
    pub assumed_malloc_overhead_layout: Layout,
    pub chunk_header_layout: Layout,
}

macro_rules! attempt {
    ($expr:expr) => {
        match $expr {
            Some(some) => some,
            None => return None,
        }
    };
}

impl ChunkLayoutConfig {
    #[inline(always)]
    pub const fn calculate_for_size_hint(self, size_hint: usize) -> Option<NonZeroUsize> {
        let Self {
            assumed_malloc_overhead_layout,
            chunk_header_layout,
            up,
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

        let size_without_overhead = size - assumed_malloc_overhead_layout.size();

        let downwards_align = if up {
            MIN_CHUNK_ALIGN
        } else {
            max(MIN_CHUNK_ALIGN, chunk_header_layout.align())
        };

        let size_for_layout = down_align(size_without_overhead, downwards_align);
        NonZeroUsize::new(size_for_layout)
    }

    #[inline(always)]
    pub const fn calculate_for_capacity(self, layout: Layout) -> Option<NonZeroUsize> {
        let Self { chunk_header_layout, .. } = self;

        let maximum_required_padding = layout.align().saturating_sub(chunk_header_layout.align());
        let required_size = attempt!(layout.size().checked_add(maximum_required_padding));
        self.calculate_for_capacity_bytes(required_size)
    }

    #[inline(always)]
    pub const fn calculate_for_capacity_bytes(self, bytes: usize) -> Option<NonZeroUsize> {
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

        self.calculate_for_size_hint(size)
    }
}

const fn max(lhs: usize, rhs: usize) -> usize {
    if lhs > rhs {
        lhs
    } else {
        rhs
    }
}

const fn offset_add_layout(mut offset: usize, layout: Layout) -> Option<usize> {
    offset = match up_align(offset, layout.align()) {
        Some(some) => some,
        None => return None,
    };

    offset = match offset.checked_add(layout.size()) {
        Some(some) => some,
        None => return None,
    };

    Some(offset)
}

#[inline(always)]
const fn up_align(addr: usize, align: usize) -> Option<usize> {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;

    let addr_plus_mask = match addr.checked_add(mask) {
        Some(some) => some,
        None => return None,
    };

    let aligned = addr_plus_mask & !mask;
    Some(aligned)
}

#[inline(always)]
const fn down_align(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use crate::tests::either_way;

    either_way! {
        debug
    }

    fn debug<const UP: bool>() {}
}
