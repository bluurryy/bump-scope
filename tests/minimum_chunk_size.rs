#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::alloc::Global;

use bump_scope::Bump;
use common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn test_minimum_chunk_size_4096() {
        let malloc_overhead = size_of::<[usize; 2]>();
        let expected_size = 4096 - malloc_overhead;

        const MY_C: BumpConfig = BumpConfigBuilder(C).const_constructible(true).min_chunk_size(4096).build();
        type MyBump = Bump<Global, MY_C>;

        assert_eq!(0, MyBump::unallocated().stats().size());
        assert_eq!(expected_size, MyBump::with_size(0).stats().size());
        assert_eq!(expected_size, MyBump::with_size(1).stats().size());
        assert_eq!(expected_size, MyBump::with_size(4096).stats().size());
        assert_eq!(
            expected_size,
            MyBump::new().scope_guard().scope().by_value().stats().size()
        );
    }
}
