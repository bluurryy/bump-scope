use core::ops::Range;
use std::{
    dbg,
    string::{String, ToString},
};

use crate::{
    alloc::{Allocator, Global},
    tests::either_way,
    Bump, BumpScope, MutBumpAllocator, MutBumpAllocatorExt, MutBumpAllocatorScope, MutBumpAllocatorScopeExt, MutBumpVecRev,
};

either_way! {
    test_dyn_allocator
}

fn test_dyn_allocator<const UP: bool>() {
    fn numbers(range: Range<i32>) -> impl ExactSizeIterator<Item = String> {
        range.map(|i| i.to_string())
    }

    fn test<'a, const UP: bool, B: MutBumpAllocatorScopeExt<'a>>(mut bump: B) {
        const ITEM_SIZE: usize = size_of::<String>();
        assert_eq!(bump.any_stats().allocated(), 0);
        let vec = MutBumpVecRev::from_iter_in(numbers(1..4), &mut bump);
        assert_eq!(vec, ["3", "2", "1"]);
        assert_eq!(vec.len(), 3);
        assert!(dbg!(vec.capacity()) >= 3);
        assert_eq!(vec.allocator_stats().into().allocated(), 0); // mut collections are special like that
        let slice = vec.into_boxed_slice();
        assert_eq!(&*slice, ["3", "2", "1"]);
        assert_eq!(bump.any_stats().allocated(), 3 * ITEM_SIZE);
    }

    <Bump<Global, 1, UP>>::new().scoped(|bump| test::<UP, BumpScope<Global, 1, UP>>(bump));
    <Bump<Global, 1, UP>>::new().scoped(|mut bump| test::<UP, &mut BumpScope<Global, 1, UP>>(&mut bump));
    <Bump<Global, 1, UP>>::new().scoped(|mut bump| test::<UP, &mut dyn MutBumpAllocatorScope>(&mut bump));
}
