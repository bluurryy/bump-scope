use core::fmt;
use std::{alloc::Layout, num::NonZeroUsize};

use arbitrary::Arbitrary;

use crate::{
    from_bump_scope::chunk_size_config::{ChunkSizeConfig, MIN_CHUNK_ALIGN},
    UpTo,
};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    up: bool,
    pointer_width: PointerWidth,
    base_allocator: BaseAllocator,
    input: SizeHintOrCapacity,
    allocator: PseudoAllocator,
}

impl Fuzz {
    pub fn run(self) {
        let Self {
            up,
            pointer_width,
            base_allocator,
            input,
            allocator,
        } = self;

        let config = ChunkSizeConfig {
            up,
            assumed_malloc_overhead_layout: assumed_malloc_overhead_layout(pointer_width),
            chunk_header_layout: chunk_header_layout(pointer_width, base_allocator.layout),
        };

        let size_hint = match input {
            SizeHintOrCapacity::SizeHint(size_hint) => Some(size_hint.0),
            SizeHintOrCapacity::Capacity(layout) => config.calc_hint_from_capacity(layout.0),
        };

        let Some(size_hint) = size_hint else { return };

        let Some(size) = config.calc_size_from_hint(size_hint) else {
            return;
        };

        let size = size.get();

        assert!(size % MIN_CHUNK_ALIGN == 0);

        if !up {
            assert!(size % config.chunk_header_layout.align() == 0);
        }

        let layout = Layout::from_size_align(size, config.chunk_header_layout.align()).unwrap();

        let Some(allocation) = allocator.allocate(layout) else {
            return;
        };

        let size = config.align_size(allocation.size);

        assert!(allocation.address.get() % layout.align() == 0);
        assert!(size <= allocation.size);
        assert!(size >= layout.size());
        assert!(size % MIN_CHUNK_ALIGN == 0);

        if !up {
            // when downwards allocating the chunk size must also be aligned to the
            // chunk header alignment, so the header can be written to
            // `ptr.byte_add(size).cast::<ChunkHeader<A>>().sub(1)`
            assert!(size % config.chunk_header_layout.align() == 0);
        }
    }
}

#[derive(Debug, Arbitrary)]
struct PseudoAllocator {
    allocation_offset: usize,
    allocation_additional_size: usize,
}

impl PseudoAllocator {
    fn allocate(&self, layout: Layout) -> Option<PseudoAllocation> {
        let size = layout.size().checked_add(self.allocation_additional_size)?;
        let addr = down_align(self.allocation_offset, layout.align());

        // addr + size must not overflow
        addr.checked_add(size)?;

        Some(PseudoAllocation {
            address: NonZeroUsize::new(addr)?,
            size,
        })
    }
}

struct PseudoAllocation {
    address: NonZeroUsize,
    size: usize,
}

#[derive(Debug, Arbitrary)]
enum SizeHintOrCapacity {
    SizeHint(UpTo<8192>),
    Capacity(FuzzLayout),
}

#[derive(Arbitrary, Clone, Copy)]
struct PointerWidth(UpTo<4>);

impl PointerWidth {
    #[cfg(test)]
    fn from_bits(bits: usize) -> Self {
        assert!(bits % 8 == 0);
        Self::from_bytes(bits / 8)
    }

    #[cfg(test)]
    fn from_bytes(bytes: usize) -> Self {
        assert!(bytes.is_power_of_two());
        Self(UpTo(bytes.trailing_zeros() as usize))
    }

    fn in_bytes(self) -> usize {
        1 << self.0 .0
    }

    fn in_bits(self) -> usize {
        8 << self.0 .0
    }
}

impl fmt::Debug for PointerWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self.in_bits(), f)
    }
}

#[derive(Debug)]
struct BaseAllocator {
    layout: Layout,
}

impl<'a> Arbitrary<'a> for BaseAllocator {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let align_pow2 = u.int_in_range(0..=14)?;
        let size_repeat_align = u.int_in_range(0..=16)?;

        let align = 1usize << align_pow2;
        let layout = Layout::from_size_align(align * size_repeat_align, align).unwrap();

        Ok(Self { layout })
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <[usize; 2] as Arbitrary>::size_hint(depth)
    }
}

#[derive(Debug, Clone, Copy)]
struct FuzzLayout(Layout);

impl<'a> Arbitrary<'a> for FuzzLayout {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let size = u.int_in_range(0..=512)?;
        let align = 1 << u.int_in_range(0..=14)?;
        Ok(FuzzLayout(Layout::from_size_align(size, align).unwrap()))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <[usize; 2]>::size_hint(depth)
    }
}

const fn down_align(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}

fn assumed_malloc_overhead_layout(pointer_width: PointerWidth) -> Layout {
    let pointer_width_bytes = pointer_width.in_bytes();
    let pointer_layout = Layout::from_size_align(pointer_width_bytes, pointer_width_bytes).unwrap();

    pointer_layout.repeat(2).unwrap().0
}

fn chunk_header_layout(pointer_width: PointerWidth, base_allocator_layout: Layout) -> Layout {
    let pointer_width_bytes = pointer_width.in_bytes();
    let pointer_layout = Layout::from_size_align(pointer_width_bytes, pointer_width_bytes).unwrap();

    pointer_layout
        .repeat(4)
        .unwrap()
        .0
        .extend(base_allocator_layout)
        .unwrap()
        .0
        .align_to(MIN_CHUNK_ALIGN)
        .unwrap()
        .pad_to_align()
}

#[test]
fn test_chunk_header_layout() {
    for bits in [8, 16, 32, 64] {
        assert_eq!(
            chunk_header_layout(PointerWidth::from_bits(bits), Layout::new::<()>()),
            Layout::from_size_align((bits / 2).max(16), 16).unwrap()
        );
    }

    assert_eq!(
        chunk_header_layout(PointerWidth::from_bits(64), Layout::new::<u64>()),
        Layout::from_size_align(48, 16).unwrap()
    );

    assert_eq!(
        chunk_header_layout(PointerWidth::from_bits(64), Layout::new::<u8>()),
        Layout::from_size_align(48, 16).unwrap()
    );

    #[repr(align(1024))]
    #[allow(dead_code)]
    struct Big([u8; 1024]);

    assert_eq!(
        chunk_header_layout(PointerWidth::from_bits(64), Layout::new::<Big>()),
        Layout::from_size_align(2048, 1024).unwrap()
    );
}
