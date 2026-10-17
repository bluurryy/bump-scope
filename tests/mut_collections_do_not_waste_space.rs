#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{alloc::Global, iter};

use bump_scope::{Bump, MutBumpVec, MutBumpVecRev};

use common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn vec() {
        let mut bump: Bump<Global, C> = Bump::new();
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

    #[test]
    fn vec_rev() {
        let mut bump: Bump<Global, C> = Bump::new();
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
}
