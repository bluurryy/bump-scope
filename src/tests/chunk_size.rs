use core::{alloc::Layout, ptr::NonNull};
use std::convert::Infallible;

use crate::{
    alloc_reexport::alloc::{AllocError, Allocator, Global},
    Bump,
};

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
