use core::{alloc::Layout, ptr::NonNull};
use std::convert::Infallible;

use crate::{
    alloc::{AllocError, Allocator, Global},
    Bump,
};

use super::either_way;

macro_rules! create_mock_allocator {
    (
        $ident:ident $size_and_align:literal
    ) => {
        #[allow(dead_code)]
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
                Global.deallocate(ptr, layout);
            }
        }
    };
}

either_way! {
    aligned_allocator_issue_32
    giant_base_allocator
}

fn aligned_allocator_issue_32<const UP: bool>() {
    create_mock_allocator! {
        BigAllocator 32
    }

    let _: Bump<_, 1, false> = Bump::with_size_in(0x2000, BigAllocator::new());
}

fn giant_base_allocator<const UP: bool>() {
    create_mock_allocator! {
        MyAllocator 4096
    }

    let bump: Bump<_, 1, UP> = Bump::with_size_in(0x2000, MyAllocator::new());
    assert_eq!(bump.stats().allocated(), 0);
    bump.alloc_str("hey");
    assert_eq!(bump.stats().allocated(), 3);
}
