#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{alloc::Global, dbg};

use bump_scope::{Bump, BumpVec, MutBumpVec, MutBumpVecRev};

use common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn bump_vec() {
        let bump = Bump::<Global, C>::new();

        bump.alloc(8u8);

        let mut vec = BumpVec::new_in(&bump);
        vec.push(32u32);

        let slice = vec.into_slice();
        dbg!(slice);
    }

    #[test]
    fn mut_bump_vec() {
        let mut bump = Bump::<Global, C>::new();

        bump.alloc(8u8);

        let mut vec = MutBumpVec::new_in(&mut bump);
        vec.push(32u32);

        let slice = vec.into_slice();
        dbg!(slice);
    }

    #[test]
    fn mut_bump_vec_rev() {
        let mut bump = Bump::<Global, C>::new();

        bump.alloc(8u8);

        let mut vec = MutBumpVecRev::new_in(&mut bump);
        vec.push(32u32);

        let slice = vec.into_slice();
        dbg!(slice);
    }
}
