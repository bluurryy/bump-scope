use super::either_way;
use crate::{bump_vec, Bump};
use allocator_api2::alloc::Global;

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

either_way! {
    shrinks
    deallocates
}
