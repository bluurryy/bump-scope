//! Some sanity checks for chunk size calculation.
//! Chunk size also has a fuzz target called `chunk_size`.

use core::{alloc::Layout, ptr::NonNull};
use std::convert::Infallible;

use crate::{
    alloc::{AllocError, Allocator, Global},
    chunk_size::AssumedMallocOverhead,
    settings::BumpSettings,
    tests::Bump,
};

use super::either_way;

macro_rules! create_mock_allocator {
    (
        $ident:ident $size_and_align:literal
    ) => {
        #[expect(dead_code)]
        #[repr(align($size_and_align))]
        #[derive(Clone)]
        struct $ident([u8; $size_and_align]);

        impl $ident {
            fn new() -> Self {
                Self([0; $size_and_align])
            }
        }

        unsafe impl Allocator for $ident {
            fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
                Global.allocate(layout)
            }

            unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
                unsafe { Global.deallocate(ptr, layout) };
            }
        }
    };
}

either_way! {
    aligned_allocator_issue_32
    giant_base_allocator
    zst
}

const OVERHEAD: usize = size_of::<AssumedMallocOverhead>();

fn zst<const UP: bool>() {
    // four pointers, + overhead, next power of two, minus overhead
    let bump = <Bump<Global, BumpSettings<1, UP>>>::with_size(0);
    assert_eq!(bump.stats().size(), size_of::<[usize; 8]>() - OVERHEAD);

    let bump = <Bump<Global, BumpSettings<1, UP>>>::with_size(512 - 1);
    assert_eq!(bump.stats().size(), 512 - OVERHEAD);

    let bump = <Bump<Global, BumpSettings<1, UP>>>::with_size(512);
    assert_eq!(bump.stats().size(), 512 - OVERHEAD);

    let bump = <Bump<Global, BumpSettings<1, UP>>>::with_size(0x1000 - 1);
    assert_eq!(bump.stats().size(), 0x1000 - OVERHEAD);

    let bump = <Bump<Global, BumpSettings<1, UP>>>::with_size(0x1000);
    assert_eq!(bump.stats().size(), 0x1000 - OVERHEAD);

    let bump = <Bump<Global, BumpSettings<1, UP>>>::with_size(0x2000 - 1);
    assert_eq!(bump.stats().size(), 0x2000 - OVERHEAD);

    let bump = <Bump<Global, BumpSettings<1, UP>>>::with_size(0x2000);
    assert_eq!(bump.stats().size(), 0x2000 - OVERHEAD);

    // same as `with_size(0)`
    let bump = <Bump<Global, BumpSettings<1, UP>>>::with_capacity(Layout::array::<u8>(0).unwrap());
    assert_eq!(bump.stats().size(), size_of::<[usize; 8]>() - OVERHEAD);
}

// TODO: `UP` not used
fn aligned_allocator_issue_32<const UP: bool>() {
    create_mock_allocator! {
        BigAllocator 32
    }

    let _: Bump<_, BumpSettings<1, false>> = Bump::with_size_in(0x2000, BigAllocator::new());
}

fn giant_base_allocator<const UP: bool>() {
    create_mock_allocator! {
        MyAllocator 4096
    }

    let bump: Bump<_, BumpSettings<1, UP>> = Bump::with_size_in(0x2000, MyAllocator::new());
    assert_eq!(bump.stats().allocated(), 0);
    bump.alloc_str("hey");
    assert_eq!(bump.stats().allocated(), 3);
}
