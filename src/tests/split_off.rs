#![allow(clippy::similar_names)]

use std::{string::String, vec};

use crate::{Bump, BumpString, BumpVec, bump_vec};

use super::TestWrap;

#[test]
fn boxed_slice_split_off_zst() {
    let bump: Bump = Bump::new();

    fn defaults() -> impl Iterator<Item = TestWrap<()>> {
        core::iter::repeat_with(Default::default)
    }

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = bump.alloc_iter(defaults().take(5));
        let rem = vec.split_off(3..);
        assert_eq!(vec.len(), 3);
        assert_eq!(rem.len(), 2);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = bump.alloc_iter(defaults().take(5));
        let rem = vec.split_off(..3);
        assert_eq!(vec.len(), 2);
        assert_eq!(rem.len(), 3);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = bump.alloc_iter(defaults().take(5));
        let rem = vec.split_off(1..4);
        assert_eq!(vec.len(), 2);
        assert_eq!(rem.len(), 3);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = bump.alloc_iter(defaults().take(5));
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
        let mut vec = BumpVec::with_capacity_in(10, &bump);
        vec.extend(defaults().take(5));
        assert_eq!(vec.capacity(), usize::MAX);
        let rem = vec.split_off(3..);
        assert_eq!(vec.len(), 3);
        assert_eq!(rem.len(), 2);
        assert_eq!(vec.capacity(), usize::MAX);
        assert_eq!(rem.capacity(), usize::MAX);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = BumpVec::with_capacity_in(10, &bump);
        vec.extend(defaults().take(5));
        assert_eq!(vec.capacity(), usize::MAX);
        let rem = vec.split_off(..3);
        assert_eq!(vec.len(), 2);
        assert_eq!(rem.len(), 3);
        assert_eq!(vec.capacity(), usize::MAX);
        assert_eq!(rem.capacity(), usize::MAX);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = BumpVec::with_capacity_in(10, &bump);
        vec.extend(defaults().take(5));
        assert_eq!(vec.capacity(), usize::MAX);
        let rem = vec.split_off(1..4);
        assert_eq!(vec.len(), 2);
        assert_eq!(rem.len(), 3);
        assert_eq!(vec.capacity(), usize::MAX);
        assert_eq!(rem.capacity(), usize::MAX);
    });

    TestWrap::expect().defaults(5).drops(5).run(|| {
        let mut vec = BumpVec::with_capacity_in(10, &bump);
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

#[test]
#[should_panic = "index is not on a char boundary"]
fn boxed_str_split_off_panic_front() {
    let bump: Bump = Bump::new();
    let mut string = bump.alloc_str("❤️❤️❤️");
    string.split_off(1..);
}

#[test]
#[should_panic = "index is not on a char boundary"]
fn boxed_str_split_off_panic_back() {
    let bump: Bump = Bump::new();
    let mut string = bump.alloc_str("❤️❤️❤️");
    string.split_off(..string.len() - 1);
}

#[test]
#[should_panic = "index is not on a char boundary"]
fn boxed_str_split_off_panic_middle_start() {
    let bump: Bump = Bump::new();
    let mut string = bump.alloc_str("❤️❤️❤️");
    string.split_off((string.len() / 3) + 1..(string.len() / 3) * 2);
}

#[test]
#[should_panic = "index is not on a char boundary"]
fn boxed_str_split_off_panic_middle_end() {
    let bump: Bump = Bump::new();
    let mut string = bump.alloc_str("❤️❤️❤️");
    string.split_off((string.len() / 3)..(string.len() / 3) * 2 - 1);
}

#[test]
#[should_panic = "index is not on a char boundary"]
fn string_split_off_panic_front() {
    let bump: Bump = Bump::new();
    let mut string = BumpString::from_str_in("❤️❤️❤️", &bump);
    string.split_off(1..);
}

#[test]
#[should_panic = "index is not on a char boundary"]
fn string_split_off_panic_back() {
    let bump: Bump = Bump::new();
    let mut string = BumpString::from_str_in("❤️❤️❤️", &bump);
    string.split_off(..string.len() - 1);
}

#[test]
#[should_panic = "index is not on a char boundary"]
fn string_split_off_panic_middle_start() {
    let bump: Bump = Bump::new();
    let mut string = BumpString::from_str_in("❤️❤️❤️", &bump);
    string.split_off((string.len() / 3) + 1..(string.len() / 3) * 2);
}

#[test]
#[should_panic = "index is not on a char boundary"]
fn string_split_off_panic_middle_end() {
    let bump: Bump = Bump::new();
    let mut string = BumpString::from_str_in("❤️❤️❤️", &bump);
    string.split_off((string.len() / 3)..(string.len() / 3) * 2 - 1);
}

#[test]
fn vec_compare_to_std() {
    let bump: Bump = Bump::new();

    let mut std = vec!['a', 'b', 'c', 'd', 'e'];
    let mut bmp = bump_vec![in &bump; 'a', 'b', 'c', 'd', 'e'];

    let std_off = std.split_off(2);
    let bmp_off = bmp.split_off(2..);

    assert_eq!(&*std, &*bmp);
    assert_eq!(&*std_off, &*bmp_off);
    assert_eq!(&*std, ['a', 'b']);
    assert_eq!(&*std_off, ['c', 'd', 'e']);
}

#[test]
fn string_compare_to_std() {
    let bump: Bump = Bump::new();

    let mut std = String::from("abcde");
    let mut bmp = BumpString::from_str_in("abcde", &bump);

    let std_off = std.split_off(2);
    let bmp_off = bmp.split_off(2..);

    assert_eq!(&*std, &*bmp);
    assert_eq!(&*std_off, &*bmp_off);
    assert_eq!(&*std, "ab");
    assert_eq!(&*std_off, "cde");
}

#[test]
fn vec_alternative_using_drain() {
    let bump: Bump = Bump::new();
    let start = 1;
    let end = 4;

    {
        let mut vec = BumpVec::from_owned_slice_in(['a', 'b', 'c', 'd', 'e'], &bump);
        let allocator = *vec.allocator();
        let other = BumpVec::from_iter_in(vec.drain(start..end), allocator);
        assert_eq!(vec, ['a', 'e']);
        assert_eq!(other, ['b', 'c', 'd']);
    }

    {
        let mut vec = BumpVec::from_owned_slice_in(['a', 'b', 'c', 'd', 'e'], &bump);
        let mut other = BumpVec::new_in(*vec.allocator());
        other.append(vec.drain(start..end));
        assert_eq!(vec, ['a', 'e']);
        assert_eq!(other, ['b', 'c', 'd']);
    }
}

#[test]
fn string_alternative_using_drain() {
    let bump: Bump = Bump::new();
    let start = 1;
    let end = 4;

    {
        let mut string = BumpString::from_str_in("abcde", &bump);
        let mut other = BumpString::new_in(*string.allocator());
        other.extend(string.drain(start..end));
        assert_eq!(string, "ae");
        assert_eq!(other, "bcd");
    }

    {
        let mut string = BumpString::from_str_in("abcde", &bump);
        let mut other = BumpString::new_in(*string.allocator());
        other.push_str(&string[start..end]);
        string.drain(start..end);
        assert_eq!(string, "ae");
        assert_eq!(other, "bcd");
    }
}
