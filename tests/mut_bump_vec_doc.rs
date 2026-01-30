#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
//! doc tests but for up and down

use bump_scope::{Bump, MutBumpVec, alloc::Global, mut_bump_vec, settings::BumpSettings};

macro_rules! either_way {
    ($($(#[$attr:meta])* $ident:ident)*) => {
        mod up {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    std::eprintln!("`UP` is `true`");
                    super::$ident::<true>();
                }
            )*
        }

        mod down {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    std::eprintln!("`UP` is `false`");
                    super::$ident::<false>();
                }
            )*
        }
    };
}

either_way! {
  new

  from_array

  from_elem

  extend_from_within_copy

  resize

  resize_with

  capacity

  insert

  remove

  swap_remove

  truncate
}

fn new<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let vec: MutBumpVec<i32, _> = mut_bump_vec![in &mut bump];
    assert!(vec.is_empty());
}

fn from_array<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    assert_eq!(vec[0], 1);
    assert_eq!(vec[1], 2);
    assert_eq!(vec[2], 3);
}

fn from_elem<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let vec = mut_bump_vec![in &mut bump; 1; 3];
    assert_eq!(vec, [1, 1, 1]);
}

fn extend_from_within_copy<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    let mut vec = mut_bump_vec![in &mut bump; 0, 1, 2, 3, 4];

    vec.extend_from_within_copy(2..);
    assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4]);

    vec.extend_from_within_copy(..2);
    assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1]);

    vec.extend_from_within_copy(4..8);
    assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1, 4, 2, 3, 4]);
}

fn resize<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    let mut vec = mut_bump_vec![in &mut bump; "hello"];
    vec.resize(3, "world");
    assert_eq!(vec, ["hello", "world", "world"]);
    drop(vec);

    let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3, 4];
    vec.resize(2, 0);
    assert_eq!(vec, [1, 2]);
}

fn resize_with<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    vec.resize_with(5, Default::default);
    assert_eq!(vec, [1, 2, 3, 0, 0]);
    drop(vec);

    let mut vec = mut_bump_vec![in &mut bump];
    let mut p = 1;
    vec.resize_with(4, || {
        p *= 2;
        p
    });
    assert_eq!(vec, [2, 4, 8, 16]);
}

fn capacity<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    let vec: MutBumpVec<i32, _> = MutBumpVec::with_capacity_in(2048, &mut bump);
    assert!(vec.capacity() >= 2048);
}

fn insert<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    vec.insert(1, 4);
    assert_eq!(vec, [1, 4, 2, 3]);
    vec.insert(4, 5);
    assert_eq!(vec, [1, 4, 2, 3, 5]);
}

fn remove<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    let mut v = mut_bump_vec![in &mut bump; 1, 2, 3];
    assert_eq!(v.remove(1), 2);
    assert_eq!(v, [1, 3]);
}

fn swap_remove<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    let mut v = mut_bump_vec![in &mut bump; "foo", "bar", "baz", "qux"];

    assert_eq!(v.swap_remove(1), "bar");
    assert_eq!(v, ["foo", "qux", "baz"]);

    assert_eq!(v.swap_remove(0), "foo");
    assert_eq!(v, ["baz", "qux"]);
}

fn truncate<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    {
        let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3, 4, 5];
        vec.truncate(2);
        assert_eq!(vec, [1, 2]);
    }

    {
        let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
        vec.truncate(8);
        assert_eq!(vec, [1, 2, 3]);
    }

    {
        let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
        vec.truncate(0);
        assert!(vec.is_empty());
    }
}
