#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{
    alloc::Global,
    string::{String, ToString},
};

use bump_scope::Bump;

use crate::common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn zst() {
        const ZST: [u64; 0] = [0u64; 0];

        let bump: Bump<Global, C> = Bump::new();

        bump.alloc_slice_copy(&[ZST; 10]).into_boxu();
        bump.alloc_slice_clone(&[ZST; 10]).into_boxu();
        bump.alloc_slice_fill(10, ZST).into_boxu();
        bump.alloc_slice_fill_with(10, || ZST).into_boxu();

        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn empty() {
        let bump: Bump<Global, C> = Bump::new();

        bump.alloc_slice_copy::<u64>(&[]).into_boxu();
        bump.alloc_slice_clone::<String>(&[]).into_boxu();
        bump.alloc_slice_fill_with(0, || -> String { panic!("should not happen") }).into_boxu();
        bump.alloc_slice_fill(0, 42u64).into_boxu();
        bump.alloc_slice_fill(0, &"hello".to_string()).into_boxu();
        bump.alloc_slice_fill_with(0, String::default).into_boxu();

        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn overflow() {
        let bump: Bump<Global, C> = Bump::new();
        bump.alloc_slice_fill_with(usize::MAX, u64::default).into_boxu();
    }
}
