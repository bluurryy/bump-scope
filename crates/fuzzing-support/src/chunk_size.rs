use std::alloc::Layout;

use arbitrary::Arbitrary;

use crate::{
    from_bump_scope::chunk_size_config::{ChunkSizeConfig, MIN_CHUNK_ALIGN},
    UpTo,
};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    up: bool,
    pointer_width: PointerWidth,
    base_allocator_layout: BaseAllocatorLayout,
    extra_allocation_size: UpTo<1024>,
    input: Input,
}

impl Fuzz {
    pub fn run(self) {
        let Self {
            up,
            pointer_width,
            base_allocator_layout,
            input,
            extra_allocation_size,
        } = self;

        let pointer_layout = Layout::from_size_align(pointer_width.0, pointer_width.0).unwrap();
        let bare_chunk_header_layout = pointer_layout.repeat(4).unwrap().0;
        let chunk_header_layout = bare_chunk_header_layout.extend(base_allocator_layout.0).unwrap().0;

        let assumed_malloc_overhead_layout = pointer_layout.repeat(2).unwrap().0;

        let config = ChunkSizeConfig {
            up,
            assumed_malloc_overhead_layout,
            chunk_header_layout,
        };

        let size_hint = match input {
            Input::SizeHint(size_hint) => Some(size_hint.0),
            Input::Capacity(layout) => config.calc_hint_from_capacity(layout.0),
        };

        let Some(size_hint) = size_hint else { return };

        let Some(size) = config.calc_size_from_hint(size_hint) else {
            return;
        };

        let size = size.get();

        assert!(size % MIN_CHUNK_ALIGN == 0);

        if !up {
            assert!(size % chunk_header_layout.align() == 0);
        }

        let layout = Layout::from_size_align(size, chunk_header_layout.align()).unwrap();

        let allocation_len = layout.size() + extra_allocation_size.0;

        let len = config.align_size(allocation_len);

        assert!(len <= allocation_len);
        assert!(len >= layout.size());
        assert!(len % MIN_CHUNK_ALIGN == 0);

        if !up {
            assert!(len % chunk_header_layout.align() == 0);
        }
    }
}

#[derive(Debug, Arbitrary)]
enum Input {
    SizeHint(UpTo<8192>),
    Capacity(FuzzLayout),
}

#[derive(Debug, Clone, Copy)]
struct PointerWidth(usize);

impl<'a> Arbitrary<'a> for PointerWidth {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(1 << u.int_in_range(0..=4)?))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <usize>::size_hint(depth)
    }
}

#[derive(Debug)]
struct BaseAllocatorLayout(Layout);

impl<'a> Arbitrary<'a> for BaseAllocatorLayout {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let align_pow2 = u.int_in_range(0..=14)?;
        let size_repeat_align = u.int_in_range(0..=16)?;

        let align = 1usize << align_pow2;
        let layout = Layout::from_size_align(align * size_repeat_align, align).unwrap();

        Ok(Self(layout))
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
