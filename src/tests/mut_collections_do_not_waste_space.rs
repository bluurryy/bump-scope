use core::iter;

use crate::{Bump, MutBumpVec, MutBumpVecRev, alloc::Global};

use super::either_way;

fn vec<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::with_size(512);
    assert_eq!(bump.stats().size(), 512 - size_of::<[usize; 2]>());

    for size in [0, 100, 200, 300, 400] {
        bump.reset();

        let mut vec: MutBumpVec<u8, _> = MutBumpVec::new_in(&mut bump);
        vec.extend(iter::repeat_n(0, size));
        assert_eq!(vec.allocator_stats().allocated(), 0); // `Mut*` allocations don't bump the pointer
        _ = vec.into_slice();
        assert_eq!(bump.stats().allocated(), size);
    }
}

fn vec_rev<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::with_size(512);
    assert_eq!(bump.stats().size(), 512 - size_of::<[usize; 2]>());

    for size in [0, 100, 200, 300, 400] {
        bump.reset();

        let mut vec: MutBumpVecRev<u8, _> = MutBumpVecRev::new_in(&mut bump);
        vec.extend(iter::repeat_n(0, size));
        assert_eq!(vec.allocator_stats().allocated(), 0); // `Mut*` allocations don't bump the pointer
        _ = vec.into_slice();
        assert_eq!(bump.stats().allocated(), size);
    }
}

either_way! {
    vec
    vec_rev
}
