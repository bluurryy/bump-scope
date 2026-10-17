#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
//! doc tests but for up and down

mod common;

use std::alloc::Global;

use bump_scope::{Bump, MutBumpVecRev, mut_bump_vec_rev};

use common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn new() {
        let mut bump: Bump<Global, C> = Bump::new();
        let vec: MutBumpVecRev<i32, _> = mut_bump_vec_rev![in &mut bump];
        assert!(vec.is_empty());
    }

    #[test]
    fn from_array() {
        let mut bump: Bump<Global, C> = Bump::new();
        let vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 3);
    }

    #[test]
    fn from_elem() {
        let mut bump: Bump<Global, C> = Bump::new();
        let vec = mut_bump_vec_rev![in &mut bump; 1; 3];
        assert_eq!(vec, [1, 1, 1]);
    }

    #[test]
    fn extend_from_within_copy() {
        let mut bump: Bump<Global, C> = Bump::new();

        let mut vec = mut_bump_vec_rev![in &mut bump; 0, 1, 2, 3, 4];

        vec.extend_from_within_copy(2..);
        assert_eq!(vec, [2, 3, 4, 0, 1, 2, 3, 4]);

        vec.extend_from_within_copy(..2);
        assert_eq!(vec, [2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);

        vec.extend_from_within_copy(4..8);
        assert_eq!(vec, [4, 0, 1, 2, 2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);
    }

    #[test]
    fn resize() {
        let mut bump: Bump<Global, C> = Bump::new();

        let mut vec = mut_bump_vec_rev![in &mut bump; "hello"];
        vec.resize(3, "world");
        assert_eq!(vec, ["world", "world", "hello"]);
        drop(vec);

        let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4];
        vec.resize(2, 0);
        assert_eq!(vec, [3, 4]);
    }

    #[test]
    fn resize_with() {
        let mut bump: Bump<Global, C> = Bump::new();

        let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
        vec.resize_with(5, Default::default);
        assert_eq!(vec, [0, 0, 1, 2, 3]);
        drop(vec);

        let mut vec = mut_bump_vec_rev![in &mut bump];
        let mut p = 1;
        vec.resize_with(4, || {
            p *= 2;
            p
        });
        assert_eq!(vec, [16, 8, 4, 2]);
    }

    #[test]
    fn capacity() {
        let mut bump: Bump<Global, C> = Bump::new();

        let vec: MutBumpVecRev<i32, _> = MutBumpVecRev::with_capacity_in(2048, &mut bump);
        assert!(vec.capacity() >= 2048);
    }

    #[test]
    fn insert() {
        let mut bump: Bump<Global, C> = Bump::new();

        let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
        vec.insert(1, 4);
        assert_eq!(vec, [1, 4, 2, 3]);
        vec.insert(4, 5);
        assert_eq!(vec, [1, 4, 2, 3, 5]);
    }

    #[test]
    fn remove() {
        let mut bump: Bump<Global, C> = Bump::new();

        let mut v = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
        assert_eq!(v.remove(1), 2);
        assert_eq!(v, [1, 3]);
    }

    #[test]
    fn swap_remove() {
        let mut bump: Bump<Global, C> = Bump::new();

        let mut v = mut_bump_vec_rev![in &mut bump; "foo", "bar", "baz", "qux"];

        assert_eq!(v.swap_remove(1), "bar");
        assert_eq!(v, ["foo", "baz", "qux"]);

        assert_eq!(v.swap_remove(0), "foo");
        assert_eq!(v, ["baz", "qux"]);
    }

    #[test]
    fn truncate() {
        let mut bump: Bump<Global, C> = Bump::new();

        {
            let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
            vec.truncate(2);
            assert_eq!(vec, [4, 5]);
        }

        {
            let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
            vec.truncate(8);
            assert_eq!(vec, [1, 2, 3]);
        }

        {
            let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
            vec.truncate(0);
            assert!(vec.is_empty());
        }
    }
}
