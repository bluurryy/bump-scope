#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
#![cfg(feature = "nightly-coerce-unsized")]

use std::{dbg, fmt::Debug};

use bump_scope::{Bump, BoxU};

#[test]
fn slice() {
    let bump: Bump = Bump::new();
    let slice: BoxU<[i32]> = bump.alloc([1, 2, 3]);
    dbg!(slice);
}

#[test]
fn object() {
    let bump: Bump = Bump::new();
    let slice: BoxU<dyn Debug> = bump.alloc([1, 2, 3]);
    dbg!(slice);
}
