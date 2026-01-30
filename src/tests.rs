#![cfg(feature = "alloc")]

use crate::{Bump, BumpVec, bump_vec};

#[test]
fn buf_reserve() {
    let bump: Bump = Bump::new();

    let mut vec: BumpVec<i32, _> = BumpVec::with_capacity_in(1, &bump);
    unsafe { vec.buf_reserve(1, 4) };
    assert_eq!(vec.capacity(), 5);

    let mut vec: BumpVec<i32, _> = bump_vec![in &bump; 1, 2];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 7);

    let mut vec = bump_vec![in &bump; 1, 2, 3];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 7);

    let mut vec = bump_vec![in &bump; 1, 2, 3, 4];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 8);
}
