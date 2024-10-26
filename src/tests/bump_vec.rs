use super::either_way;
use crate::{bump_vec, Bump, BumpVec};
use allocator_api2::alloc::Global;

either_way! {
    shrinks
    deallocates
    into_slice
    into_slice_without_shrink
}

fn shrinks<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut vec = bump_vec![in bump; 1, 2, 3, 4];
    assert_eq!(bump.stats().allocated(), 4 * 4);
    vec.pop();
    vec.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 3 * 4);
    vec.clear();
    vec.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 0);
}

fn deallocates<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let vec = bump_vec![in bump; 1, 2, 3];
    assert_eq!(bump.stats().allocated(), 3 * 4);
    drop(vec);
    assert_eq!(bump.stats().allocated(), 0);
}

fn into_slice<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut vec = bump_vec![in bump; 1, 2, 3, 4, 5];
    assert_eq!(bump.stats().allocated(), 5 * 4);
    vec.truncate(3);
    let slice = vec.into_slice();
    assert_eq!(bump.stats().allocated(), 3 * 4);
    _ = slice;
}

fn into_slice_without_shrink<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut vec = bump_vec![in bump; 1, 2, 3, 4, 5];
    assert_eq!(bump.stats().allocated(), 5 * 4);
    vec.truncate(3);
    let slice = vec.into_fixed_vec().into_slice();
    assert_eq!(bump.stats().allocated(), 5 * 4);
    _ = slice;
}

#[test]
fn buf_reserve() {
    let bump: Bump = Bump::new();

    let mut vec: BumpVec<i32> = BumpVec::with_capacity_in(1, &bump);
    unsafe { vec.buf_reserve(1, 2) };
    assert_eq!(vec.capacity(), 3);

    let mut vec: BumpVec<i32> = bump_vec![in bump; 1, 2];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 7);

    let mut vec = bump_vec![in bump; 1, 2, 3];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 7);

    let mut vec = bump_vec![in bump; 1, 2, 3, 4];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 8);
}
