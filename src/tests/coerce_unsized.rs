use std::{dbg, fmt::Debug, future::Future};

use crate::{BumpBox, tests::Bump};

#[test]
fn slice() {
    let bump: Bump = Bump::new();
    let slice: BumpBox<[i32]> = bump.alloc([1, 2, 3]);
    dbg!(slice);
}

#[test]
fn object() {
    let bump: Bump = Bump::new();
    let slice: BumpBox<dyn Debug> = bump.alloc([1, 2, 3]);
    dbg!(slice);
}
