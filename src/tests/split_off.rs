use crate::{Bump, BumpString, BumpVec};

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
        let mut vec = BumpVec::with_capacity_in(10, &bump);
        vec.append(['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(3..);
        assert_eq!(vec, ['a', 'b', 'c']);
        assert_eq!(rem, ['d', 'e']);
        assert_eq!(vec.capacity(), 3);
        assert_eq!(rem.capacity(), 7);
    }

    {
        let mut vec = BumpVec::with_capacity_in(10, &bump);
        vec.append(['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(..3);
        assert_eq!(vec, ['d', 'e']);
        assert_eq!(rem, ['a', 'b', 'c']);
        assert_eq!(vec.capacity(), 7);
        assert_eq!(rem.capacity(), 3);
    }

    {
        let mut vec = BumpVec::with_capacity_in(10, &bump);
        vec.append(['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(1..4);
        assert_eq!(vec, ['a', 'e']);
        assert_eq!(rem, ['b', 'c', 'd']);
        assert_eq!(vec.capacity(), 2);
        assert_eq!(rem.capacity(), 8);
    }

    {
        let mut vec = BumpVec::with_capacity_in(10, &bump);
        vec.append(['a', 'b', 'c', 'd', 'e']);
        let rem = vec.split_off(..);
        assert_eq!(vec, []);
        assert_eq!(rem, ['a', 'b', 'c', 'd', 'e']);
        assert_eq!(vec.capacity(), 0);
        assert_eq!(rem.capacity(), 10);
    }
}

#[test]
fn boxed_str_split_off() {
    let bump: Bump = Bump::new();

    {
        let mut vec = bump.alloc_str("abcde");
        let rem = vec.split_off(3..);
        assert_eq!(vec, "abc");
        assert_eq!(rem, "de");
    }

    {
        let mut vec = bump.alloc_str("abcde");
        let rem = vec.split_off(..3);
        assert_eq!(vec, "de");
        assert_eq!(rem, "abc");
    }

    {
        let mut vec = bump.alloc_str("abcde");
        let rem = vec.split_off(1..4);
        assert_eq!(vec, "ae");
        assert_eq!(rem, "bcd");
    }

    {
        let mut vec = bump.alloc_str("abcde");
        let rem = vec.split_off(..);
        assert_eq!(vec, "");
        assert_eq!(rem, "abcde");
    }
}

#[test]
fn string_split_off() {
    let bump: Bump = Bump::new();

    {
        let mut vec = BumpString::with_capacity_in(10, &bump);
        vec.push_str("abcde");
        let rem = vec.split_off(3..);
        assert_eq!(vec, "abc");
        assert_eq!(rem, "de");
        assert_eq!(vec.capacity(), 3);
        assert_eq!(rem.capacity(), 7);
    }

    {
        let mut vec = BumpString::with_capacity_in(10, &bump);
        vec.push_str("abcde");
        let rem = vec.split_off(..3);
        assert_eq!(vec, "de");
        assert_eq!(rem, "abc");
        assert_eq!(vec.capacity(), 7);
        assert_eq!(rem.capacity(), 3);
    }

    {
        let mut vec = BumpString::with_capacity_in(10, &bump);
        vec.push_str("abcde");
        let rem = vec.split_off(1..4);
        assert_eq!(vec, "ae");
        assert_eq!(rem, "bcd");
        assert_eq!(vec.capacity(), 2);
        assert_eq!(rem.capacity(), 8);
    }

    {
        let mut vec = BumpString::with_capacity_in(10, &bump);
        vec.push_str("abcde");
        let rem = vec.split_off(..);
        assert_eq!(vec, "");
        assert_eq!(rem, "abcde");
        assert_eq!(vec.capacity(), 0);
        assert_eq!(rem.capacity(), 10);
    }
}
