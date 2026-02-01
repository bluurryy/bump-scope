#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use bump_scope::{Bump, alloc::Global, settings::BumpSettings};
use common::either_way;

either_way! {
   test_minimum_chunk_size_4096
}

fn test_minimum_chunk_size_4096<const UP: bool>() {
    let malloc_overhead = size_of::<[usize; 2]>();
    let expected_size = 4096 - malloc_overhead;

    type MyBump<const UP: bool> = Bump<Global, BumpSettings<1, UP, false, true, true, true, 4096>>;

    assert_eq!(0, MyBump::<UP>::new().stats().size());
    assert_eq!(expected_size, MyBump::<UP>::with_size(0).stats().size());
    assert_eq!(expected_size, MyBump::<UP>::with_size(1).stats().size());
    assert_eq!(expected_size, MyBump::<UP>::with_size(4096).stats().size());
    assert_eq!(
        expected_size,
        MyBump::<UP>::new().scope_guard().scope().by_value().stats().size()
    );
}
