#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::{dbg, fmt::Debug};

use bump_scope::{BoxU, Bump, BumpAllocator as _};

#[test]
fn slice() {
    let bump: Bump = Bump::new();
    let slice: BoxU<[i32]> = bump.alloc([1, 2, 3]).into_boxu();
    dbg!(slice);
}

#[test]
fn object() {
    let bump: Bump = Bump::new();
    let slice: BoxU<dyn Debug> = bump.alloc([1, 2, 3]).into_boxu();
    dbg!(slice);
}
