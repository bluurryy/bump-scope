use crate::Bump;
use allocator_api2::alloc::Global;

use super::either_way;

fn zst<const UP: bool>() {
    const ZST: [u64; 0] = [0u64; 0];

    let bump: Bump<Global, 1, UP> = Bump::new();

    bump.alloc_slice_copy(&[ZST]);
    bump.alloc_slice_clone(&[ZST]);
    bump.alloc_slice_fill(1, ZST);
    bump.alloc_slice_fill_with(1, || ZST);

    assert_eq!(bump.stats().allocated(), 0);
}

fn empty<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();

    bump.alloc_slice_copy::<u64>(&[]);
    bump.alloc_slice_clone::<String>(&[]);
    bump.alloc_slice_fill_with(0, || -> String { panic!("should not happen") });
    bump.alloc_slice_fill(0, 42u64);
    bump.alloc_slice_fill(0, &"hello".to_string());
    bump.alloc_slice_fill_with(0, String::default);

    assert_eq!(bump.stats().allocated(), 0);
}

fn overflow<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    bump.alloc_slice_fill_with(usize::MAX, u64::default);
}

either_way! {
    zst
    empty

    #[should_panic(expected = "capacity overflow")]
    overflow
}
