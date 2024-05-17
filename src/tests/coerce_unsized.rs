use core::{fmt::Debug, future::Future};

use crate::{Box, Bump};

#[test]
fn slice() {
    let bump: Bump = Bump::new();
    let slice: Box<[i32]> = bump.alloc([1, 2, 3]);
    dbg!(slice);
}

#[test]
fn object() {
    let bump: Bump = Bump::new();
    let slice: Box<dyn Debug> = bump.alloc([1, 2, 3]);
    dbg!(slice);
}
