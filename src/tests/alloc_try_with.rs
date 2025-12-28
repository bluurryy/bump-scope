#![allow(clippy::result_large_err)]

use std::{
    dbg,
    mem::{self, offset_of},
};

use crate::{alloc::Global, tests::Bump};

use super::either_way;

macro_rules! assert_allocated {
    ($bump:ident, $expected:expr) => {
        let expected = $expected;
        let count = $bump.stats().count();
        let allocated = $bump.stats().allocated();
        assert_eq!(
            count, 1,
            "there are multiple chunks allocated ({count}), \
            this makes `assert_allocated_range` misleading"
        );
        assert_eq!(
            allocated, expected,
            "expected allocated bytes of {expected} but got {allocated}"
        );
    };
}

const SIZE: usize = 1024 * 10;

fn zeroes<const N: usize>() -> [u32; N] {
    [0; N]
}

type TestOk = [u32; 32];
type TestErr = [u32; 128];
type TestResult = Result<TestOk, TestErr>;

#[cfg(feature = "nightly-tests")]
fn basic_ok<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::with_size(SIZE);
    _ = bump.alloc_try_with(|| -> TestResult { Ok(zeroes()) });

    if UP {
        let expected = 32 * 4 + offset_of!(TestResult, Ok.0);
        assert_allocated!(bump, expected);
    } else {
        let expected = size_of::<TestResult>() - offset_of!(TestResult, Ok.0);
        assert_allocated!(bump, expected);
    }
}

#[cfg(feature = "nightly-tests")]
fn basic_ok_mut<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::with_size(SIZE);
    _ = bump.alloc_try_with_mut(|| -> TestResult { Ok(zeroes()) });

    if UP {
        let expected = 32 * 4 + offset_of!(TestResult, Ok.0);
        assert_allocated!(bump, expected);
    } else {
        let expected = size_of::<TestResult>() - offset_of!(TestResult, Ok.0);
        assert_allocated!(bump, expected);
    }
}

fn basic_err<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::with_size(SIZE);
    _ = bump.alloc_try_with(|| -> Result<[u32; 32], [u32; 128]> { Err(zeroes()) });
    assert_eq!(bump.stats().allocated(), 0);
}

fn basic_err_mut<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::with_size(SIZE);
    _ = bump.alloc_try_with_mut(|| -> Result<[u32; 32], [u32; 128]> { Err(zeroes()) });
    assert_eq!(bump.stats().allocated(), 0);
}

fn alloc_in_closure_ok<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::with_size(SIZE);

    _ = bump.alloc_try_with(|| -> Result<[u32; 32], [u32; 128]> {
        bump.alloc(0u8);
        Ok(zeroes())
    });

    let expected = size_of::<TestResult>() + 1;
    assert_allocated!(bump, expected);
}

fn alloc_in_closure_err<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::with_size(SIZE);

    _ = bump.alloc_try_with(|| -> Result<[u32; 32], [u32; 128]> {
        bump.alloc(0u8);
        Err(zeroes())
    });

    let expected = size_of::<TestResult>() + 1;
    assert_allocated!(bump, expected);
}

#[test]
fn wat() {
    let bump: Bump<Global, 1, false> = Bump::new();
    bump.alloc(0u32);
    dbg!(bump.stats().allocated());
}

either_way! {
    #[cfg(feature = "nightly-tests")]
    basic_ok
    #[cfg(feature = "nightly-tests")]
    basic_ok_mut
    basic_err
    basic_err_mut
    alloc_in_closure_ok
    alloc_in_closure_err
}
