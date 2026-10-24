#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
#![expect(clippy::similar_names)]

mod common;

use bump_scope::{Bump, BumpAllocator as _, extra, vec};

use common::TestWrap;

#[test]
fn boxed_slice_split_off_zst() {
    let bump: Bump = Bump::new();

    fn defaults() -> impl Iterator<Item = TestWrap<()>> {
        core::iter::repeat_with(Default::default)
    }

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = bump.alloc_iter(defaults().take(5)).into_boxu();
        let rem = vec.split_off(3..);
        assert_eq!(vec.len(), 3);
        assert_eq!(rem.len(), 2);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = bump.alloc_iter(defaults().take(5)).into_boxu();
        let rem = vec.split_off(..3);
        assert_eq!(vec.len(), 2);
        assert_eq!(rem.len(), 3);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = bump.alloc_iter(defaults().take(5)).into_boxu();
        let rem = vec.split_off(1..4);
        assert_eq!(vec.len(), 2);
        assert_eq!(rem.len(), 3);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = bump.alloc_iter(defaults().take(5)).into_boxu();
        let rem = vec.split_off(..);
        assert_eq!(vec.len(), 0);
        assert_eq!(rem.len(), 5);
    });
}

#[test]
fn vec_split_off_zst() {
    let bump: Bump = Bump::new();

    fn defaults() -> impl Iterator<Item = TestWrap<()>> {
        core::iter::repeat_with(Default::default)
    }

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = Vec::with_capacity_in(10, &bump);
        vec.extend(defaults().take(5));
        assert_eq!(vec.capacity(), usize::MAX);
        let rem = vec.split_off(3..);
        assert_eq!(vec.len(), 3);
        assert_eq!(rem.len(), 2);
        assert_eq!(vec.capacity(), usize::MAX);
        assert_eq!(rem.capacity(), usize::MAX);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = Vec::with_capacity_in(10, &bump);
        vec.extend(defaults().take(5));
        assert_eq!(vec.capacity(), usize::MAX);
        let rem = vec.split_off(..3);
        assert_eq!(vec.len(), 2);
        assert_eq!(rem.len(), 3);
        assert_eq!(vec.capacity(), usize::MAX);
        assert_eq!(rem.capacity(), usize::MAX);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = Vec::with_capacity_in(10, &bump);
        vec.extend(defaults().take(5));
        assert_eq!(vec.capacity(), usize::MAX);
        let rem = vec.split_off(1..4);
        assert_eq!(vec.len(), 2);
        assert_eq!(rem.len(), 3);
        assert_eq!(vec.capacity(), usize::MAX);
        assert_eq!(rem.capacity(), usize::MAX);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = Vec::with_capacity_in(10, &bump);
        vec.extend(defaults().take(5));
        assert_eq!(vec.capacity(), usize::MAX);
        let rem = vec.split_off(..);
        assert_eq!(vec.len(), 0);
        assert_eq!(rem.len(), 5);
        assert_eq!(vec.capacity(), usize::MAX);
        assert_eq!(rem.capacity(), usize::MAX);
    });
}

#[test]
fn boxed_slice_split_off() {
    let bump: Bump = Bump::new();

    {
        let mut vec = bump.alloc_slice_copy(&['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(3..);
        assert_eq!(vec, ['a', 'b', 'c']);
        assert_eq!(rem, ['d', 'e']);
    }

    {
        let mut vec = bump.alloc_slice_copy(&['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(..3);
        assert_eq!(vec, ['d', 'e']);
        assert_eq!(rem, ['a', 'b', 'c']);
    }

    {
        let mut vec = bump.alloc_slice_copy(&['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(1..4);
        assert_eq!(vec, ['a', 'e']);
        assert_eq!(rem, ['b', 'c', 'd']);
    }

    {
        let mut vec = bump.alloc_slice_copy(&['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(..);
        assert_eq!(vec, []);
        assert_eq!(rem, ['a', 'b', 'c', 'd', 'e']);
    }
}

#[test]
fn vec_split_off() {
    let bump: Bump = Bump::new();

    {
        let mut vec = Vec::with_capacity_in(10, &bump);
        extra::vec_append(&mut vec, ['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(3..);
        assert_eq!(vec, ['a', 'b', 'c']);
        assert_eq!(rem, ['d', 'e']);
        assert_eq!(vec.capacity(), 3);
        assert_eq!(rem.capacity(), 7);
    }

    {
        let mut vec = Vec::with_capacity_in(10, &bump);
        extra::vec_append(&mut vec, ['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(..3);
        assert_eq!(vec, ['d', 'e']);
        assert_eq!(rem, ['a', 'b', 'c']);
        assert_eq!(vec.capacity(), 7);
        assert_eq!(rem.capacity(), 3);
    }

    {
        let mut vec = Vec::with_capacity_in(10, &bump);
        extra::vec_append(&mut vec, ['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(1..4);
        assert_eq!(vec, ['a', 'e']);
        assert_eq!(rem, ['b', 'c', 'd']);
        assert_eq!(vec.capacity(), 2);
        assert_eq!(rem.capacity(), 8);
    }

    {
        let mut vec = Vec::with_capacity_in(10, &bump);
        extra::vec_append(&mut vec, ['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(..);
        assert_eq!(vec, []);
        assert_eq!(rem, ['a', 'b', 'c', 'd', 'e']);
        assert_eq!(vec.capacity(), 0);
        assert_eq!(rem.capacity(), 10);
    }
}

#[test]
fn vec_compare_to_std() {
    let bump: Bump = Bump::new();

    let mut std = vec!['a', 'b', 'c', 'd', 'e'];
    let mut bmp = vec![in &bump; 'a', 'b', 'c', 'd', 'e'];

    let std_off = std.split_off(2);
    let bmp_off = bmp.split_off(2..);

    assert_eq!(&*std, &*bmp);
    assert_eq!(&*std_off, &*bmp_off);
    assert_eq!(&*std, ['a', 'b']);
    assert_eq!(&*std_off, ['c', 'd', 'e']);
}

#[test]
fn vec_alternative_using_drain() {
    let bump: Bump = Bump::new();
    let start = 1;
    let end = 4;

    {
        let mut vec = extra::vec_from_owned_slice_in(['a', 'b', 'c', 'd', 'e'], &bump);
        let allocator = *vec.allocator();
        let other = extra::vec_from_iter_in(vec.drain(start..end), allocator);
        assert_eq!(vec, ['a', 'e']);
        assert_eq!(other, ['b', 'c', 'd']);
    }

    {
        let mut vec = extra::vec_from_owned_slice_in(['a', 'b', 'c', 'd', 'e'], &bump);
        let mut other = Vec::new_in(*vec.allocator());
        extra::vec_append(&mut other, vec.drain(start..end));
        assert_eq!(vec, ['a', 'e']);
        assert_eq!(other, ['b', 'c', 'd']);
    }
}
