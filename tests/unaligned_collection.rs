#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::dbg;

use bump_scope::{Bump, BumpVec, MutBumpVec, MutBumpVecRev, alloc::Global, settings::BumpSettings};

use common::either_way;

either_way! {
    bump_vec
    mut_bump_vec
    mut_bump_vec_rev
}

fn bump_vec<const UP: bool>() {
    let bump = Bump::<Global, BumpSettings<1, UP>>::new();

    bump.alloc(8u8);

    let mut vec = BumpVec::new_in(&bump);
    vec.push(32u32);

    let slice = vec.into_slice();
    dbg!(slice);
}

fn mut_bump_vec<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();

    bump.alloc(8u8);

    let mut vec = MutBumpVec::new_in(&mut bump);
    vec.push(32u32);

    let slice = vec.into_slice();
    dbg!(slice);
}

fn mut_bump_vec_rev<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();

    bump.alloc(8u8);

    let mut vec = MutBumpVecRev::new_in(&mut bump);
    vec.push(32u32);

    let slice = vec.into_slice();
    dbg!(slice);
}
