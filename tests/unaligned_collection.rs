#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{alloc::Global, dbg};

use bump_scope::Bump;

use common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn bump_vec() {
        let bump = Bump::<Global, C>::new();

        bump.alloc(8u8).into_boxu();

        let mut vec = Vec::new_in(&bump);
        vec.push(32u32);

        let slice = extra::vec_into_slice(vec);
        dbg!(slice);
    }
}
