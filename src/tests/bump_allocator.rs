use std::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{BumpAllocatorExt, BumpVec, MutBumpAllocatorExt, MutBumpVec, tests::Bump};

fn number_strings(numbers: impl IntoIterator<Item = i32>) -> impl Iterator<Item = String> {
    numbers.into_iter().map(|i| i.to_string())
}

#[test]
fn smoke_test() {
    fn test<A: BumpAllocatorExt>(a: A) {
        let mut vec = BumpVec::from_iter_in(number_strings(1..=5), a);
        vec.extend(number_strings(6..=9));
    }

    fn mut_test<A: MutBumpAllocatorExt>(a: A) {
        let mut vec = MutBumpVec::from_iter_in(number_strings(1..=5), a);
        vec.extend(number_strings(6..=9));
    }

    let mut a: Bump = Bump::new();
    test(&mut a);
    test(&a);
    test(a);

    let mut a: Bump = Bump::new();
    a.scoped(|mut a| {
        test(&mut a);
        test(&a);
        test(a);
    });

    let mut a: Bump = Bump::new();
    mut_test(&mut a);
    mut_test(a);

    let mut a: Bump = Bump::new();
    a.scoped(|mut a| {
        mut_test(&mut a);
        mut_test(a);
    });
}

/// Checks that a bigger chunk is correctly allocated when the current chunk is not largest one
#[test]
fn alloc_chunks() {
    let mut a: Bump = Bump::new();

    a.scoped(|mut a| {
        // alloc is large enough to require an additional chunk
        let _: MutBumpVec<u8, &mut _> = MutBumpVec::with_capacity_in(1 << 9, &mut a);
    });

    a.scoped(|mut a| {
        // alloc is larger than the existing chunks and requires a third chunk to be allocated
        let _: MutBumpVec<u8, _> = MutBumpVec::with_capacity_in(1 << 10, &mut a);
    });
}
