#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
#![cfg_attr(feature = "nightly", feature(allocator_ext))]

mod common;

use std::{
    alloc::{AllocError, Allocator, Global},
    boxed::Box,
    vec::Vec,
};

use bump_scope::{Bump, extra};

use common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn grow() {
        let mut bump: Bump<Global, C> = Bump::new();

        let mut vec = Vec::<i32, _>::new_in(&bump);
        assert_eq!(bump.stats().allocated(), 0);

        vec.reserve_exact(1);
        assert_eq!(bump.stats().allocated(), 4);

        vec.reserve_exact(2);
        assert_eq!(bump.stats().allocated(), 8);

        bump.alloc_uninit::<u8>().into_boxu();
        assert_eq!(bump.stats().allocated(), 9);

        vec.reserve_exact(3);
        assert_eq!(bump.stats().allocated(), 12 + 3 * 4);

        drop(vec);
        assert_eq!(bump.stats().allocated(), 12);

        bump.reset();
        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn shrink() {
        let bump: Bump<Global, C> = Bump::new();

        let mut vec = Vec::<i32, _>::new_in(&bump);
        assert_eq!(bump.stats().allocated(), 0);

        vec.reserve_exact(2);
        assert_eq!(bump.stats().allocated(), 8);

        vec.shrink_to(1);
        assert_eq!(bump.stats().allocated(), 4);

        let boxed: Box<_, _> = bump.alloc_uninit::<u8>().into_box();
        assert_eq!(bump.stats().allocated(), 5);

        drop(boxed);
        assert_eq!(bump.stats().allocated(), 4);

        vec.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn shrinknt() {
        let bump: Bump<Global, C> = Bump::new();

        let mut vec = Vec::<i32, _>::new_in(&bump);
        assert_eq!(bump.stats().allocated(), 0);

        vec.reserve_exact(2);
        assert_eq!(bump.stats().allocated(), 8);

        bump.alloc_uninit::<u8>().into_boxu();
        assert_eq!(bump.stats().allocated(), 9);

        vec.shrink_to(1);
        assert_eq!(bump.stats().allocated(), 9);

        drop(vec);
        assert_eq!(bump.stats().allocated(), 9);
    }

    #[test]
    fn vec_macro() {
        test_vec_macro(Global).unwrap();
        test_vec_macro(Bump::<Global, C>::new()).unwrap();
    }
}

fn test_vec_macro(alloc: impl Allocator) -> Result<(), AllocError> {
    let empty: Vec<String, _> = extra::vec![];
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.capacity(), 0);

    let one: Vec<String, _> = extra::vec![1.to_string()];
    assert_eq!(one.len(), 1);
    assert_eq!(one.capacity(), 1);

    let two: Vec<String, _> = extra::vec![1.to_string(), 2.to_string()];
    assert_eq!(two.len(), 2);
    assert_eq!(two.capacity(), 2);

    let three: Vec<String, _> = extra::vec![1.to_string(); 3];
    assert_eq!(three.len(), 3);
    assert_eq!(three.capacity(), 3);

    let empty: Vec<String, _> = extra::vec![in &alloc];
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.capacity(), 0);

    let one: Vec<String, _> = extra::vec![in &alloc; 1.to_string()];
    assert_eq!(one.len(), 1);
    assert_eq!(one.capacity(), 1);

    let two: Vec<String, _> = extra::vec![in &alloc; 1.to_string(), 2.to_string()];
    assert_eq!(two.len(), 2);
    assert_eq!(two.capacity(), 2);

    let three: Vec<String, _> = extra::vec![in &alloc; 1.to_string(); 3];
    assert_eq!(three.len(), 3);
    assert_eq!(three.capacity(), 3);

    let empty: Vec<String, _> = extra::vec![try in &alloc]?;
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.capacity(), 0);

    let one: Vec<String, _> = extra::vec![try in &alloc; 1.to_string()]?;
    assert_eq!(one.len(), 1);
    assert_eq!(one.capacity(), 1);

    let two: Vec<String, _> = extra::vec![try in &alloc; 1.to_string(), 2.to_string()]?;
    assert_eq!(two.len(), 2);
    assert_eq!(two.capacity(), 2);

    let three: Vec<String, _> = extra::vec![try in &alloc; 1.to_string(); 3]?;
    assert_eq!(three.len(), 3);
    assert_eq!(three.capacity(), 3);

    Ok(())
}
