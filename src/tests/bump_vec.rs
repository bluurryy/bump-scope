#![allow(clippy::manual_assert)]

use core::ops::Range;
use std::{
    boxed::Box,
    dbg, format,
    hint::black_box,
    string::{String, ToString},
};

use crate::{
    Bump, BumpAllocator, BumpAllocatorExt, BumpAllocatorScope, BumpScope, BumpVec, MutBumpAllocator, MutBumpAllocatorScope,
    WithoutDealloc, WithoutShrink, alloc::Global, bump_vec, tests::expect_no_panic,
};

use super::either_way;

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
    map_in_place_same_layout
    map_in_place_smaller_layout
    map_in_place_to_zst
    map_in_place_from_zst_to_zst
    test_dyn_allocator
}

fn shrinks<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();

    // should shrink
    let mut vec = bump_vec![in &bump; 1, 2, 3, 4];
    assert_eq!(bump.stats().allocated(), 4 * 4);
    vec.pop();
    vec.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 3 * 4);
    vec.clear();
    vec.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 0);

    // shouldn't shrink
    let mut vec = bump_vec![in WithoutShrink(&bump); 1, 2, 3, 4];
    assert_eq!(bump.stats().allocated(), 4 * 4);
    vec.pop();
    vec.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 4 * 4);
    vec.clear();
    vec.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 4 * 4);
}

fn deallocates<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();

    // should deallocate
    let vec = bump_vec![in &bump; 1, 2, 3];
    assert_eq!(bump.stats().allocated(), 3 * 4);
    drop(vec);
    assert_eq!(bump.stats().allocated(), 0);

    // shouldn't deallocate
    let vec = bump_vec![in WithoutDealloc(&bump); 1, 2, 3];
    assert_eq!(bump.stats().allocated(), 3 * 4);
    drop(vec);
    assert_eq!(bump.stats().allocated(), 3 * 4);
}

fn into_slice<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut vec = bump_vec![in &bump; 1, 2, 3, 4, 5];
    assert_eq!(bump.stats().allocated(), 5 * 4);
    vec.truncate(3);
    let slice = vec.into_slice();
    assert_eq!(bump.stats().allocated(), 3 * 4);
    _ = slice;
}

fn into_slice_without_shrink<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut vec = bump_vec![in &bump; 1, 2, 3, 4, 5];
    assert_eq!(bump.stats().allocated(), 5 * 4);
    vec.truncate(3);
    let slice = vec.into_fixed_vec().into_slice();
    assert_eq!(bump.stats().allocated(), 5 * 4);
    _ = slice;
}

#[test]
fn buf_reserve() {
    let bump: Bump = Bump::new();

    let mut vec: BumpVec<i32, _> = BumpVec::with_capacity_in(1, &bump);
    unsafe { vec.buf_reserve(1, 4) };
    assert_eq!(vec.capacity(), 5);

    let mut vec: BumpVec<i32, _> = bump_vec![in &bump; 1, 2];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 7);

    let mut vec = bump_vec![in &bump; 1, 2, 3];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 7);

    let mut vec = bump_vec![in &bump; 1, 2, 3, 4];
    unsafe { vec.buf_reserve(2, 5) };
    assert_eq!(vec.capacity(), 8);
}

fn map_same_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<String, _>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: BumpVec<Option<String>, _> = a.map(|s| {
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

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
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
            let a = BumpVec::<String, _>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: BumpVec<Box<str>, _> = a.map(|s| {
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

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
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
            let a = BumpVec::<Box<str>, _>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string().into()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<Box<str>>() * 3);
            let b: BumpVec<String, _> = a.map(|s| {
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

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
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
            let a = BumpVec::<String, _>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: BumpVec<(), _> = a.map(|_| {
                if i == panic_on {
                    panic!("oh no");
                }

                i += 1;
            });
            assert_eq!(b.capacity(), usize::MAX);
            assert_eq!(bump.stats().allocated(), 0);
            dbg!(b);
        });

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
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
            let a = BumpVec::<(), _>::from_iter_exact_in([(), (), ()], &bump);
            assert_eq!(a.capacity(), usize::MAX);
            assert_eq!(bump.stats().allocated(), 0);
            let b: BumpVec<String, _> = a.map(|()| {
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

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
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
            let a = BumpVec::<(), _>::from_iter_exact_in([(), (), ()], &bump);
            assert_eq!(a.capacity(), usize::MAX);
            assert_eq!(bump.stats().allocated(), 0);
            let b: BumpVec<(), _> = a.map(|()| {
                if i == panic_on {
                    panic!("oh no");
                }

                i += 1;
            });
            assert_eq!(b.capacity(), usize::MAX);
            assert_eq!(bump.stats().allocated(), 0);
            dbg!(b);
        });

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
    }
}

fn map_in_place_same_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<String, _>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: BumpVec<Option<String>, _> = a.map_in_place(|s| {
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

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
    }
}

fn map_in_place_smaller_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<String, _>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: BumpVec<Box<str>, _> = a.map_in_place(|s| {
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

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
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

fn map_in_place_to_zst<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<String, _>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: BumpVec<AlignedZst, _> = a.map_in_place(|_| {
                if i == panic_on {
                    panic!("oh no");
                }

                i += 1;
                AlignedZst
            });
            assert_eq!(b.capacity(), usize::MAX);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            dbg!(b);
        });

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        if panic_on == 0 {
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3, "panic_on={panic_on}");
        } else {
            assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
        }
    }
}

fn map_in_place_from_zst_to_zst<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = BumpVec::<(), _>::from_iter_exact_in([(), (), ()], &bump);
            assert_eq!(a.capacity(), usize::MAX);
            assert_eq!(bump.stats().allocated(), 0);
            let b: BumpVec<AlignedZst, _> = a.map_in_place(|()| {
                if i == panic_on {
                    panic!("oh no");
                }

                i += 1;
                AlignedZst
            });
            assert_eq!(b.capacity(), usize::MAX);
            assert_eq!(bump.stats().allocated(), 0);
            dbg!(b);
        });

        if panic_on == 0 {
            expect_no_panic(result);
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
    }
}

#[repr(align(1024))]
#[derive(Debug)]
struct AlignedZst;

fn test_dyn_allocator<const UP: bool>() {
    fn numbers(range: Range<i32>) -> impl ExactSizeIterator<Item = String> {
        range.map(|i| i.to_string())
    }

    fn test<B: BumpAllocatorExt>(bump: B) {
        const ITEM_SIZE: usize = size_of::<String>();
        assert_eq!(bump.any_stats().allocated(), 0);
        let mut vec = BumpVec::from_iter_in(numbers(1..4), &bump);
        assert_eq!(vec, ["1", "2", "3"]);
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.capacity(), 3);
        assert_eq!(bump.any_stats().allocated(), 3 * ITEM_SIZE);
        vec.reserve_exact(4);
        assert_eq!(vec, ["1", "2", "3"]);
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.capacity(), 7);
        assert_eq!(bump.any_stats().allocated(), 7 * ITEM_SIZE);
        vec.shrink_to_fit();
        assert_eq!(vec, ["1", "2", "3"]);
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.capacity(), 3);
        assert_eq!(bump.any_stats().allocated(), 3 * ITEM_SIZE);
        vec.truncate(2);
        assert_eq!(vec, ["1", "2"]);
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.capacity(), 3);
        assert_eq!(bump.any_stats().allocated(), 3 * ITEM_SIZE);
        vec.shrink_to_fit();
        assert_eq!(vec, ["1", "2"]);
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.capacity(), 2);
        assert_eq!(bump.any_stats().allocated(), 2 * ITEM_SIZE);
        vec.clear();
        vec.shrink_to_fit();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 0);
        assert_eq!(bump.any_stats().allocated(), 0);
        drop(vec);
        let vec = BumpVec::from_iter_exact_in(numbers(1..2), &bump);
        assert_eq!(vec, ["1"]);
        assert_eq!(vec.len(), 1);
        assert_eq!(vec.capacity(), 1);
        assert_eq!(bump.any_stats().allocated(), ITEM_SIZE);
        drop(vec);
        assert_eq!(bump.any_stats().allocated(), 0);
    }

    test::<Bump>(Bump::new());
    test::<&Bump>(&Bump::new());
    test::<&mut Bump>(&mut Bump::new());

    Bump::new().scoped(|bump| test::<BumpScope>(bump));
    Bump::new().scoped(|bump| test::<&BumpScope>(&bump));
    Bump::new().scoped(|mut bump| test::<&mut BumpScope>(&mut bump));

    test::<&dyn BumpAllocator>(&<Bump>::new());
    test::<&mut dyn BumpAllocator>(&mut <Bump>::new());
    test::<&dyn MutBumpAllocator>(&<Bump>::new());
    test::<&mut dyn MutBumpAllocator>(&mut <Bump>::new());
    test::<&dyn BumpAllocatorScope>(<Bump>::new().as_scope());
    test::<&mut dyn BumpAllocatorScope>(<Bump>::new().as_mut_scope());
    test::<&dyn MutBumpAllocatorScope>(<Bump>::new().as_scope());
    test::<&mut dyn MutBumpAllocatorScope>(<Bump>::new().as_mut_scope());
}
