use std::{
    alloc::{AllocError, Allocator, Global, Layout},
    ptr::NonNull,
};

use bump_scope::{Bump, config::DEFAULT_BUMP_CONFIG};

const PTR_SIZE: usize = size_of::<*const ()>();

#[derive(Clone, Copy)]
struct SmallAllocator<const SIZE: usize> {
    #[expect(dead_code)]
    byte: [u8; SIZE],
}

unsafe impl<const SIZE: usize> Allocator for SmallAllocator<SIZE> {
    fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unimplemented!()
    }

    unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {
        unimplemented!()
    }
}

#[test]
fn test_size() {
    assert_eq!(
        size_of::<Bump<DEFAULT_BUMP_CONFIG, Global>>(),
        if cfg!(feature = "nightly") { PTR_SIZE } else { PTR_SIZE * 2 }
    );

    assert_eq!(
        size_of::<Bump<DEFAULT_BUMP_CONFIG, &Global>>(),
        if cfg!(feature = "nightly") {
            PTR_SIZE * 2
        } else {
            PTR_SIZE * 3
        }
    );

    assert_eq!(size_of::<Bump<DEFAULT_BUMP_CONFIG, Box<Global>>>(), PTR_SIZE * 3);

    // Rust optimizes the enum layout for ZSTs, null becomes the owned variant.
    assert_eq!(size_of::<Bump<DEFAULT_BUMP_CONFIG, SmallAllocator<0>>>(), PTR_SIZE * 2);

    // `SmallAllocator` does not implement `AllocatorClone`, so we need to use the enum representation.
    assert_eq!(size_of::<Bump<DEFAULT_BUMP_CONFIG, SmallAllocator<1>>>(), PTR_SIZE * 3);
}
