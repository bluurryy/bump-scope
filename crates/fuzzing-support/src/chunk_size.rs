use core::fmt;
use std::{alloc::Layout, num::NonZeroUsize};

use arbitrary::Arbitrary;

use crate::from_bump_scope::chunk_size_config::{ChunkSizeConfig, MIN_CHUNK_ALIGN};

macro_rules! assert_le {
    ($lhs:expr, $rhs:expr $(, $($msg:tt)*)?) => {
        let lhs = $lhs;
        let rhs = $rhs;

        assert!(
            lhs <= rhs,
            concat!("expected `{}` ({}) to be less or equal to `{}` ({})" $(, $($msg)*)?),
            stringify!($lhs),
            lhs,
            stringify!($rhs),
            rhs,
        )
    };
}

macro_rules! assert_ge {
    ($lhs:expr, $rhs:expr $(, $($msg:tt)*)?) => {
        let lhs = $lhs;
        let rhs = $rhs;

        assert!(
            lhs >= rhs,
            concat!("expected `{}` ({}) to be greater or equal to `{}` ({})" $(, $($msg)*)?),
            stringify!($lhs),
            lhs,
            stringify!($rhs),
            rhs,
        )
    };
}

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    up: bool,
    pointer_width: PointerWidth,
    base_allocator_layout: ArbitraryLayout,
    base_allocator: PseudoAllocator,
    size_hint_or_capacity: SizeHintOrCapacity,
}

impl Fuzz {
    pub fn run(self) {
        let Self {
            up,
            pointer_width,
            base_allocator_layout,
            base_allocator,
            size_hint_or_capacity,
        } = self;

        let config = ChunkSizeConfig {
            up,
            assumed_malloc_overhead_layout: assumed_malloc_overhead_layout(pointer_width),
            chunk_header_layout: match chunk_header_layout(pointer_width, base_allocator_layout.get()) {
                Some(some) => some,
                None => {
                    // This calculation can fail if the if the base allocator layout is great enough that adding
                    // the size of the rest of the chunk header fields results in an invalid layout size.
                    // In practice, this would be a compile time error saying `values of the type `ChunkHeader<A>` are too big`.
                    return;
                }
            },
        };

        let size_hint = match size_hint_or_capacity {
            SizeHintOrCapacity::SizeHint(size_hint) => Some(size_hint),
            SizeHintOrCapacity::Capacity(layout) => config.calc_hint_from_capacity(layout.get()),
        };

        let Some(size_hint) = size_hint else { return };

        let Some(size) = config.calc_size_from_hint(size_hint) else {
            return;
        };

        let size = size.get();

        assert!(size.is_multiple_of(MIN_CHUNK_ALIGN));

        if !up {
            assert!(size.is_multiple_of(config.chunk_header_layout.align()));
        }

        assert_ge!(size, config.chunk_header_layout.size());

        let Ok(chunk_layout) = Layout::from_size_align(size, config.chunk_header_layout.align()) else {
            return;
        };

        let Some(PseudoAllocation {
            address,
            size: unaligned_size,
        }) = base_allocator.allocate(chunk_layout)
        else {
            return;
        };

        let address = address.get();
        let size = config.align_size(unaligned_size);

        assert_ge!(size, config.chunk_header_layout.size());
        assert!(address.is_multiple_of(chunk_layout.align()));
        assert_le!(size, unaligned_size);
        assert_ge!(size, chunk_layout.size());
        assert!(size.is_multiple_of(MIN_CHUNK_ALIGN));

        if !up {
            // when downwards allocating the chunk size must also be aligned to the
            // chunk header alignment, so the header can be written to
            // `ptr.byte_add(size).cast::<ChunkHeader<A>>().sub(1)`
            assert!(size.is_multiple_of(config.chunk_header_layout.align()));
        }

        if let SizeHintOrCapacity::Capacity(required_capacity_layout) = size_hint_or_capacity {
            let content_start: usize;
            let content_end: usize;

            if up {
                content_start = address + config.chunk_header_layout.align();
                content_end = address + size;
            } else {
                content_start = address;
                content_end = (address + size) - config.chunk_header_layout.size();
            }

            let aligned_start = up_align(content_start, required_capacity_layout.align).unwrap();
            let true_capacity = content_end - aligned_start;

            if true_capacity < required_capacity_layout.size {
                dbg!(
                    chunk_layout,
                    address,
                    unaligned_size,
                    size,
                    content_start,
                    content_end,
                    aligned_start,
                    content_end - content_start,
                    true_capacity,
                    required_capacity_layout.get()
                );

                assert_ge!(true_capacity, required_capacity_layout.size);
            }
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

#[derive(Arbitrary, Clone, Copy)]
enum SizeHintOrCapacity {
    SizeHint(usize),
    Capacity(ArbitraryLayout),
}

impl fmt::Debug for SizeHintOrCapacity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SizeHint(arg0) => f.debug_tuple("SizeHintOrCapacity::SizeHint").field(arg0).finish(),
            Self::Capacity(arg0) => f.debug_tuple("SizeHintOrCapacity::Capacity").field(arg0).finish(),
        }
    }
}

// All `target_pointer_width` that rust currently supports.
#[derive(Arbitrary, Clone, Copy)]
enum PointerWidth {
    Bits16 = 16,
    Bits32 = 32,
    Bits64 = 64,
}

impl PointerWidth {
    #[cfg(test)]
    fn from_bits(bits: usize) -> Self {
        match bits {
            16 => PointerWidth::Bits16,
            32 => PointerWidth::Bits32,
            64 => PointerWidth::Bits64,
            _ => panic!("pointer width not supported: {bits}"),
        }
    }

    #[cfg(test)]
    #[expect(dead_code)]
    fn from_bytes(bytes: usize) -> Self {
        PointerWidth::from_bits(bytes * 8)
    }

    fn in_bytes(self) -> usize {
        self.in_bits() / 8
    }

    fn in_bits(self) -> usize {
        self as usize
    }
}

impl fmt::Debug for PointerWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PointerWidth::Bits16 => "PointerWidth::Bits16",
            PointerWidth::Bits32 => "PointerWidth::Bits32",
            PointerWidth::Bits64 => "PointerWidth::Bits64",
        })
    }
}

impl fmt::Display for PointerWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Display::fmt(&self.in_bits(), f)
    }
}

#[derive(Debug, Clone, Copy)]
struct ArbitraryLayout {
    size: usize,
    align: usize,
}

impl ArbitraryLayout {
    fn get(self) -> Layout {
        let Self { size, align } = self;

        match Layout::from_size_align(size, align) {
            Ok(ok) => ok,
            Err(_) => panic!("Layout {{ size: {size}, align: {align} }} does not represent a valid layout"),
        }
    }
}

impl<'a> Arbitrary<'a> for ArbitraryLayout {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        // let align_max = checked_prev_power_of_two(isize::MAX as usize).unwrap();
        let align_max = isize::MAX as usize + 1;
        let align: usize = u.int_in_range(1..=align_max)?;
        let align = checked_prev_power_of_two(align).unwrap();

        let size_max = (isize::MAX as usize) + 1 - align;
        let size: usize = u.int_in_range(0..=size_max)?;

        assert!(align.is_power_of_two());

        if Layout::from_size_align(size, align).is_err() {
            unreachable!("Layout {{ size: {size}, align: {align} }} does not represent a valid layout")
        };

        Ok(Self { size, align })
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

fn chunk_header_layout(pointer_width: PointerWidth, base_allocator_layout: Layout) -> Option<Layout> {
    let pointer_width_bytes = pointer_width.in_bytes();
    let pointer_layout = Layout::from_size_align(pointer_width_bytes, pointer_width_bytes).unwrap();

    Some(
        pointer_layout
            .repeat(4)
            .unwrap()
            .0
            .extend(base_allocator_layout)
            .ok()?
            .0
            .align_to(MIN_CHUNK_ALIGN)
            .ok()?
            .pad_to_align(),
    )
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

pub const fn bit_width(value: usize) -> u32 {
    if value == 0 { 0 } else { usize::BITS - value.leading_zeros() }
}

/// Returns the largest power-of-2 less than or equal to the input, or `None` if `self == 0`.
pub const fn checked_prev_power_of_two(value: usize) -> Option<usize> {
    if value == 0 { None } else { Some(1 << (bit_width(value) - 1)) }
}

#[test]
fn test_chunk_header_layout() {
    #[repr(align(1024))]
    #[expect(dead_code)]
    struct Big([u8; 1024]);

    for bits in [16, 32, 64] {
        // ZST allocator
        assert_eq!(
            chunk_header_layout(PointerWidth::from_bits(bits), Layout::new::<()>()).unwrap(),
            Layout::from_size_align(((bits / 8) * 4).max(16), 16).unwrap()
        );

        // Byte-sized allocator
        assert_eq!(
            chunk_header_layout(
                PointerWidth::from_bits(bits),
                Layout::from_size_align(bits / 8, bits / 8).unwrap()
            )
            .unwrap(),
            Layout::from_size_align(up_align((bits / 8) * 4 + 1, 16).unwrap().max(16), 16).unwrap()
        );

        // Pointer-sized allocator
        assert_eq!(
            chunk_header_layout(
                PointerWidth::from_bits(bits),
                Layout::from_size_align(bits / 8, bits / 8).unwrap()
            )
            .unwrap(),
            Layout::from_size_align(up_align((bits / 8) * 5, 16).unwrap().max(16), 16).unwrap()
        );

        // Unreasonably large allocator
        assert_eq!(
            chunk_header_layout(PointerWidth::from_bits(bits), Layout::new::<Big>()).unwrap(),
            Layout::from_size_align(2048, 1024).unwrap()
        );
    }
}
