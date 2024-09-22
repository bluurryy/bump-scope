use super::either_way;
use crate::{polyfill::nonnull, Bump};
use allocator_api2::alloc::{Allocator, Global};
use core::{alloc::Layout, ptr::NonNull};

either_way! {
  grow

  grow_last_in_place

  grow_last_out_of_place
}

fn layout(size: usize) -> Layout {
    Layout::from_size_align(size, 4).unwrap()
}

fn assert_aligned_to(ptr: NonNull<[u8]>) {
    assert!(nonnull::addr(ptr.cast::<u8>()).get() % 4 == 0);
}

fn grow<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();

    let ptr = bump.allocate(layout(1)).unwrap();
    assert_aligned_to(ptr);

    assert_ne!(ptr, bump.allocate(layout(1)).unwrap());

    let new = bump.allocate(layout(2048)).unwrap();
    assert_aligned_to(new);

    assert_ne!(ptr.cast::<u8>(), new.cast::<u8>());
}

fn grow_last_in_place<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();

    unsafe {
        let ptr = bump.allocate(layout(1)).unwrap();
        assert_aligned_to(ptr);

        let new = bump.grow(ptr.cast(), layout(1), layout(2)).unwrap();
        assert_aligned_to(new);

        if UP {
            assert_eq!(ptr.cast::<u8>(), new.cast::<u8>());
        }
    }
}

fn grow_last_out_of_place<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();

    unsafe {
        let ptr = bump.allocate(layout(1)).unwrap();
        assert_aligned_to(ptr);

        let new = bump.grow(ptr.cast(), layout(1), layout(2048)).unwrap();
        assert_aligned_to(new);

        assert_ne!(ptr.cast::<u8>(), new.cast::<u8>());
    }
}
