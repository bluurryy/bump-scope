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

#[test]
fn aligned_allocator_issue_32() {
    #[allow(dead_code)]
    #[repr(align(32))]
    #[derive(Clone)]
    struct BigAllocator([u8; 32]);
    const BIG_ALLOCATOR: BigAllocator = BigAllocator([0u8; 32]);

    unsafe impl Allocator for BigAllocator {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            Global.allocate(layout)
        }

        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            Global.deallocate(ptr, layout);
        }
    }

    let _: Bump<_, 1, false> = Bump::with_size_in(0x2000, BIG_ALLOCATOR);
}

either_way! {
    giant_base_allocator
}

fn giant_base_allocator<const UP: bool>() {
    create_mock_allocator! {
        MyAllocator 16
    }

    let bump: Bump<_, 1, UP> = Bump::with_size_in(0x2000, MyAllocator::new());
    assert_eq!(bump.stats().allocated(), 0);
    bump.alloc_str("hey");
    assert_eq!(bump.stats().allocated(), 3);
}
