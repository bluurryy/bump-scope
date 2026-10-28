#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
#![cfg_attr(feature = "nightly", feature(offset_of_enum))]
#![allow(clippy::result_large_err)]

mod common;

use std::alloc::Global;

#[cfg(feature = "nightly")]
use std::mem::offset_of;

use bump_scope::Bump;
#[cfg(feature = "nightly")]
use bump_scope::config;

use crate::common::up_and_down;

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

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    #[cfg(feature = "nightly")]
    fn basic_ok() {
        let bump: Bump<Global, C> = Bump::with_size(SIZE);
        _ = bump.alloc_try_with(|| -> TestResult { Ok(zeroes()) }).unwrap().into_boxu();
        assert_allocated!(bump, size_of::<TestResult>());
    }

    #[test]
    #[cfg(feature = "nightly")]
    fn basic_ok_mut() {
        let mut bump: Bump<Global, C> = Bump::with_size(SIZE);
        _ = bump.alloc_try_with_mut(|| -> TestResult { Ok(zeroes()) }).unwrap().into_boxu();


        if config::read::upwards(C) {
            let expected = 32 * 4 + offset_of!(TestResult, Ok.0);
            assert_allocated!(bump, expected);
        } else {
            let expected = size_of::<TestResult>() - offset_of!(TestResult, Ok.0);
            assert_allocated!(bump, expected);
        }
    }

    #[test]
    fn basic_err() {
        let bump: Bump<Global, C> = Bump::with_size(SIZE);
        _ = bump.alloc_try_with(|| -> TestResult { Err(zeroes()) }).unwrap_err();
        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn basic_err_mut() {
        let mut bump: Bump<Global, C> = Bump::with_size(SIZE);
        _ = bump.alloc_try_with_mut(|| -> TestResult { Err(zeroes()) }).unwrap_err();
        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn alloc_in_closure_ok() {
        let bump: Bump<Global, C> = Bump::with_size(SIZE);

        _ = bump.alloc_try_with(|| -> TestResult {
            bump.alloc(0u8).into_boxu();
            Ok(zeroes())
        }).unwrap().into_boxu();

        let expected = size_of::<TestResult>() + 1;
        assert_allocated!(bump, expected);
    }

    #[test]
    fn alloc_in_closure_err() {
        let bump: Bump<Global, C> = Bump::with_size(SIZE);

        _ = bump.alloc_try_with(|| -> TestResult {
            bump.alloc(0u8).into_boxu();
            Err(zeroes())
        }).unwrap_err();

        let expected = size_of::<TestResult>() + 1;
        assert_allocated!(bump, expected);
    }
}
