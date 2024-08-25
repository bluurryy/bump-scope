use core::convert::Infallible;

use allocator_api2::alloc::{Allocator, Global};

use crate::Bump;

#[test]
fn aligned_allocator_issue_32() {
    #[allow(dead_code)]
    #[repr(align(32))]
    #[derive(Clone)]
    struct BigAllocator([u8; 32]);
    const BIG_ALLOCATOR: BigAllocator = BigAllocator([0u8; 32]);

    unsafe impl Allocator for BigAllocator {
        fn allocate(
            &self,
            layout: core::alloc::Layout,
        ) -> Result<core::ptr::NonNull<[u8]>, allocator_api2::alloc::AllocError> {
            Global.allocate(layout)
        }

        unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
            Global.deallocate(ptr, layout)
        }
    }

    let _: Bump<_, 1, false> = Bump::with_size_in(0x2000, BIG_ALLOCATOR);
}
