#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{
    dbg,
    ops::Range,
    string::{String, ToString},
};

use bump_scope::{
    Bump, BumpScope, MutBumpVec,
    alloc::Global,
    settings::BumpSettings,
    traits::{MutBumpAllocatorCoreScope, MutBumpAllocatorTypedScope},
};

use common::either_way;

either_way! {
    test_dyn_allocator
}

fn test_dyn_allocator<const UP: bool>() {
    fn numbers(range: Range<i32>) -> impl ExactSizeIterator<Item = String> {
        range.map(|i| i.to_string())
    }

    fn test<'a, const UP: bool, B: MutBumpAllocatorTypedScope<'a>>(mut bump: B) {
        const ITEM_SIZE: usize = size_of::<String>();
        assert_eq!(bump.any_stats().allocated(), 0);
        let vec = MutBumpVec::from_iter_in(numbers(1..4), &mut bump);
        assert_eq!(vec, ["1", "2", "3"]);
        assert_eq!(vec.len(), 3);
        assert!(dbg!(vec.capacity()) >= 3);
        assert_eq!(vec.allocator_stats().into().allocated(), 0); // mut collections are special like that
        let slice = vec.into_boxed_slice();
        assert_eq!(&*slice, ["1", "2", "3"]);
        assert_eq!(bump.any_stats().allocated(), 3 * ITEM_SIZE);
    }

    <Bump<Global, BumpSettings<1, UP>>>::new().scoped(|bump| test::<UP, &mut BumpScope<Global, BumpSettings<1, UP>>>(bump));
    <Bump<Global, BumpSettings<1, UP>>>::new().scoped(|bump| test::<UP, &mut dyn MutBumpAllocatorCoreScope>(bump));
}
