//! Tests for `dropck-eyepatch` / `#[may_dangle]`.
//! Yandros explains this quite well:
//! <https://users.rust-lang.org/t/phantomdata-and-dropck-confusion/26624/2>
//!
//! See `/tests/compile_fail/may_dangle` for tests that make sure this does not
//! allow you to write code that accesses dangling references (correct `PhantomData<T>` usage).
#![cfg(feature = "nightly-dropck-eyepatch")]

use std::{string::String, vec};

use crate::{BumpBox, BumpVec, FixedBumpVec, MutBumpVec, MutBumpVecRev, bump_vec, tests::Bump};

#[test]
fn test_std() {
    let mut v = vec![];
    let s = String::from("hello");
    v.push(&s);
}

#[test]
fn test_box() {
    let bump: Bump = Bump::new();
    let mut b = bump.alloc(None);
    let s = String::from("hello");
    *b = Some(&s);
}

#[test]
fn test_fixed_bump_vec() {
    let bump: Bump = Bump::new();
    let mut v = FixedBumpVec::with_capacity_in(1, &bump);
    let s = String::from("hello");
    v.push(&s);
}

#[test]
fn test_bump_vec() {
    let bump: Bump = Bump::new();
    let mut v = BumpVec::new_in(&bump);
    let s = String::from("hello");
    v.push(&s);
}

#[test]
fn test_mut_bump_vec() {
    let mut bump: Bump = Bump::new();
    let mut v = MutBumpVec::new_in(&mut bump);
    let s = String::from("hello");
    v.push(&s);
}

#[test]
fn test_mut_bump_vec_rev() {
    let mut bump: Bump = Bump::new();
    let mut v = MutBumpVecRev::new_in(&mut bump);
    let s = String::from("hello");
    v.push(&s);
}
