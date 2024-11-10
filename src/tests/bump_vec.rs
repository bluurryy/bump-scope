#![allow(clippy::manual_assert)]
use core::hint::black_box;

use super::either_way;
use crate::{bump_vec, Bump, BumpVec};
use allocator_api2::alloc::Global;

either_way! {
    shrinks
    deallocates
    into_slice
    into_slice_without_shrink
    map_same_layout
    map_smaller_layout
    map_bigger_layout
    map_to_zst
    map_from_zst
    map_from_zst_to_zst
}

fn shrinks<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut vec = bump_vec![in bump; 1, 2, 3, 4];
    assert_eq!(bump.stats().allocated(), 4 * 4);
    vec.pop();
    vec.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 3 * 4);
    vec.clear();
    vec.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 0);
}

fn deallocates<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let vec = bump_vec![in bump; 1, 2, 3];
    assert_eq!(bump.stats().allocated(), 3 * 4);
    drop(vec);
    assert_eq!(bump.stats().allocated(), 0);
}

fn into_slice<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut vec = bump_vec![in bump; 1, 2, 3, 4, 5];
    assert_eq!(bump.stats().allocated(), 5 * 4);
    vec.truncate(3);
    let slice = vec.into_slice();
    assert_eq!(bump.stats().allocated(), 3 * 4);
    _ = slice;
}

fn into_slice_without_shrink<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut vec = bump_vec![in bump; 1, 2, 3, 4, 5];
    assert_eq!(bump.stats().allocated(), 5 * 4);
    vec.truncate(3);
    let slice = vec.into_fixed_vec().into_slice();
    assert_eq!(bump.stats().allocated(), 5 * 4);
    _ = slice;
}

#[test]
fn buf_reserve() {
    let bump: Bump = Bump::new();

    let mut vec: BumpVec<i32> = BumpVec::with_capacity_in(1, &bump);
    unsafe { vec.buf_reserve(1, 4) };
    assert_eq!(vec.capacity(), 5);

    let mut vec: BumpVec<i32> = bump_vec![in bump; 1, 2];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 7);

    let mut vec = bump_vec![in bump; 1, 2, 3];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 7);

    let mut vec = bump_vec![in bump; 1, 2, 3, 4];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 8);
}

fn map_same_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<String, Global, 1, UP>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: BumpVec<Option<String>, Global, 1, UP> = a.map(|s| {
                if i == panic_on {
                    panic!("oh no");
                }

                let value = format!("hello: {s}");
                i += 1;
                Some(value)
            });
            assert_eq!(b.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<Option<String>>() * 3);
            dbg!(b);
        });

        if panic_on != 0 {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
    }
}

fn map_smaller_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<String, Global, 1, UP>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: BumpVec<Box<str>, Global, 1, UP> = a.map(|s| {
                if i == panic_on {
                    panic!("oh no");
                }

                let value = format!("hello: {s}");
                i += 1;
                value.into()
            });
            assert_eq!(b.capacity(), 4);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            dbg!(b);
        });

        if panic_on != 0 {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        if UP {
            // when upwards allocating we can't deallocate `b`,
            // because it can no longer be identified as the last allocation
            assert_eq!(
                bump.stats().allocated(),
                if panic_on == 0 { size_of::<String>() * 3 } else { 0 },
                "panic_on={panic_on}"
            );
        } else {
            // when downwards allocating we can deallocate `b`,
            // but we only retain the knowledge of its new extent of `size_of::<Box<str>>() * 4`
            // which is smaller than its old extent of `size_of::<String>() * 3`, so 8 bytes
            // are left over
            assert_eq!(
                bump.stats().allocated(),
                if panic_on == 0 {
                    size_of::<String>() * 3 - size_of::<Box<str>>() * 4
                } else {
                    0
                },
                "panic_on={panic_on}"
            );
        }
    }
}

fn map_bigger_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<Box<str>, Global, 1, UP>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string().into()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<Box<str>>() * 3);
            let b: BumpVec<String, Global, 1, UP> = a.map(|s| {
                if i == panic_on {
                    panic!("oh no");
                }

                let value = format!("hello: {s}");
                i += 1;
                value
            });
            assert_eq!(b.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<Box<str>>() * 3 + size_of::<String>() * 3);
            dbg!(b);
        });

        if panic_on != 0 {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(
            bump.stats().allocated(),
            if panic_on == 0 { size_of::<Box<str>>() * 3 } else { 0 },
            "panic_on={panic_on}"
        );
    }
}

fn map_to_zst<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<String, Global, 1, UP>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: BumpVec<(), Global, 1, UP> = a.map(|_| {
                if i == panic_on {
                    panic!("oh no");
                }

                i += 1;
            });
            assert_eq!(b.capacity(), 3);
            assert_eq!(bump.stats().allocated(), 0);
            dbg!(b);
        });

        if panic_on != 0 {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
    }
}

fn map_from_zst<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<(), Global, 1, UP>::from_iter_exact_in([(), (), ()], &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), 0);
            let b: BumpVec<String, Global, 1, UP> = a.map(|()| {
                if i == panic_on {
                    panic!("oh no");
                }

                i += 1;
                "hello".into()
            });
            assert_eq!(b.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            dbg!(b);
        });

        if panic_on != 0 {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
    }
}

fn map_from_zst_to_zst<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<(), Global, 1, UP>::from_iter_exact_in([(), (), ()], &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), 0);
            let b: BumpVec<(), Global, 1, UP> = a.map(|()| {
                if i == panic_on {
                    panic!("oh no");
                }

                i += 1;
            });
            assert_eq!(b.capacity(), 3);
            assert_eq!(bump.stats().allocated(), 0);
            dbg!(b);
        });

        if panic_on != 0 {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
    }
}
