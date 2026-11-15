use std::{
    alloc::{AllocError, Allocator, Layout},
    ptr::NonNull,
};

use bump_scope::{Bump, config::DEFAULT_BUMP_CONFIG};

#[derive(Default)]
struct MyAllocator;

unsafe impl Allocator for MyAllocator {
    fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unimplemented!()
    }

    unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {
        unimplemented!()
    }
}

fn main() {
    _ = <Bump<DEFAULT_BUMP_CONFIG, MyAllocator>>::const_new();
}
