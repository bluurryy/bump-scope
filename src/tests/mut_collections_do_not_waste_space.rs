use std::iter;

use crate::{Bump, MutBumpVec, MutBumpVecRev};
use allocator_api2::alloc::Global;

use super::either_way;

fn vec<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::with_size(512);
    assert_eq!(bump.stats().size(), 512 - size_of::<[usize; 2]>());

    for size in [0, 100, 200, 300, 400] {
        bump.reset();

        let mut vec: MutBumpVec<u8, Global, 1, UP> = MutBumpVec::new_in(&mut bump);
        vec.extend(iter::repeat(0).take(size));
        assert_eq!(vec.stats().allocated(), 0); // `Mut*` allocations don't bump the pointer
        _ = vec.into_slice();
        assert_eq!(bump.stats().allocated(), size);
    }
}

fn vec_rev<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::with_size(512);
    assert_eq!(bump.stats().size(), 512 - size_of::<[usize; 2]>());

    for size in [0, 100, 200, 300, 400] {
        bump.reset();

        let mut vec: MutBumpVecRev<u8, Global, 1, UP> = MutBumpVecRev::new_in(&mut bump);
        vec.extend(iter::repeat(0).take(size));
        assert_eq!(vec.stats().allocated(), 0); // `Mut*` allocations don't bump the pointer
        _ = vec.into_slice();
        assert_eq!(bump.stats().allocated(), size);
    }
}

either_way! {
    vec
    vec_rev
}
