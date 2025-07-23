use core::ops::Range;
use std::{
    dbg,
    string::{String, ToString},
};

use crate::{
    alloc::Allocator, Bump, BumpScope, MutBumpAllocator, MutBumpAllocatorExt, MutBumpAllocatorScope,
    MutBumpAllocatorScopeExt, MutBumpVec,
};

#[test]
fn dyn_allocator() {
    fn numbers(range: Range<i32>) -> impl ExactSizeIterator<Item = String> {
        range.map(|i| i.to_string())
    }

    // TODO: this should work without a `Scope` a mutable ref to the bump should automatically be a scope
    fn test<'a, B: MutBumpAllocatorScopeExt<'a>>(mut bump: B) {
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

    <Bump>::new().scoped(|bump| test::<BumpScope>(bump));
    <Bump>::new().scoped(|mut bump| test::<&mut BumpScope>(&mut bump));
    <Bump>::new().scoped(|mut bump| test::<&mut dyn MutBumpAllocatorScope>(&mut bump));
}
