#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
#![expect(clippy::manual_assert)]

mod common;

use std::{
    alloc::Global,
    boxed::Box,
    dbg, format,
    ops::Range,
    string::{String, ToString},
};

use bump_scope::{Bump, BumpScope, config, traits::BumpAllocator};

use common::{expect_no_panic, up_and_down};

up_and_down! {
    const C: BumpConfig = DEFAULT_C;
    const C_NO_SHRINK: BumpConfig = BumpConfigBuilder(C).shrinks(false).build();
    const C_NO_DEALLOC: BumpConfig = BumpConfigBuilder(C).deallocates(false).build();

    #[test]
    fn shrinks() {
        let bump: Bump<Global, C> = Bump::new();

        // should shrink
        let mut vec = vec![in &bump; 1, 2, 3, 4];
        assert_eq!(bump.stats().allocated(), 4 * 4);
        vec.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 4 * 4);
        vec.pop();
        vec.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 3 * 4);
        vec.clear();
        vec.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 0);
        drop(vec);

        // shouldn't shrink
        let mut vec = vec![in bump.ref_change_config::<C_NO_SHRINK>(); 1, 2, 3, 4];
        assert_eq!(bump.stats().allocated(), 4 * 4);
        vec.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 4 * 4);
        vec.pop();
        vec.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 4 * 4);
        vec.clear();
        vec.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 4 * 4);
    }

    #[test]
    fn deallocates() {
        let bump: Bump<Global, C> = Bump::new();

        // should deallocate
        let vec = vec![in &bump; 1, 2, 3];
        assert_eq!(bump.stats().allocated(), 3 * 4);
        drop(vec);
        assert_eq!(bump.stats().allocated(), 0);

        // shouldn't deallocate
        let vec = vec![in bump.ref_change_config::<C_NO_DEALLOC>(); 1, 2, 3];
        assert_eq!(bump.stats().allocated(), 3 * 4);
        drop(vec);
        assert_eq!(bump.stats().allocated(), 3 * 4);
    }

    #[test]
    fn into_slice() {
        let bump: Bump<Global, C> = Bump::new();
        let mut vec = vec![in &bump; 1, 2, 3, 4, 5];
        assert_eq!(bump.stats().allocated(), 5 * 4);
        vec.truncate(3);
        let slice = extra::vec_into_slice(vec);
        assert_eq!(bump.stats().allocated(), 3 * 4);
        _ = slice;
    }

    #[test]
    fn into_slice_without_shrink() {
        let bump: Bump<Global, C> = Bump::new();
        let mut vec = vec![in &bump; 1, 2, 3, 4, 5];
        assert_eq!(bump.stats().allocated(), 5 * 4);
        vec.truncate(3);
        let slice = extra::vec_into_slice(extra::vec_remove_allocator(vec).0);
        assert_eq!(bump.stats().allocated(), 5 * 4);
        _ = slice;
    }

    #[test]
    fn map_same_layout() {
        for panic_on in 0..4 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let mut i = 1;
                let a = extra::vec_from_iter_in::<String, _>([1, 2, 3].map(|i| i.to_string()), &bump);
                assert_eq!(a.capacity(), 3);
                assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
                let b: Vec<Option<String>, _> = extra::vec_map(a, |s| {
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

    #[test]
    fn map_smaller_layout() {
        for panic_on in 0..4 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let mut i = 1;
                let a = extra::vec_from_iter_in::<String, _>([1, 2, 3].map(|i| i.to_string()), &bump);
                assert_eq!(a.capacity(), 3);
                assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
                let b: Vec<Box<str>, _> = extra::vec_map(a, |s| {
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

            if config::read::upwards(C) {
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

    #[test]
    fn map_bigger_layout() {
        for panic_on in 0..4 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let mut i = 1;
                let a = extra::vec_from_iter_in::<Box<str>, _>([1, 2, 3].map(|i| i.to_string().into()), &bump);
                assert_eq!(a.capacity(), 3);
                assert_eq!(bump.stats().allocated(), size_of::<Box<str>>() * 3);
                let b: Vec<String, _> = extra::vec_map(a, |s| {
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

    #[test]
    fn map_to_zst() {
        for panic_on in 0..4 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let mut i = 1;
                let a = extra::vec_from_iter_in::<String, _>([1, 2, 3].map(|i| i.to_string()), &bump);
                assert_eq!(a.capacity(), 3);
                assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
                let b: Vec<(), _> = extra::vec_map(a, |_| {
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

    #[test]
    fn map_from_zst() {
        for panic_on in 0..4 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let mut i = 1;
                let a = extra::vec_from_iter_in::<(), _>([(), (), ()], &bump);
                assert_eq!(a.capacity(), usize::MAX);
                assert_eq!(bump.stats().allocated(), 0);
                let b: Vec<String, _> = extra::vec_map(a, |()| {
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

    #[test]
    fn map_from_zst_to_zst() {
        for panic_on in 0..4 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let mut i = 1;
                let a = extra::vec_from_iter_in::<(), _>([(), (), ()], &bump);
                assert_eq!(a.capacity(), usize::MAX);
                assert_eq!(bump.stats().allocated(), 0);
                let b: Vec<(), _> = extra::vec_map(a, |()| {
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

    #[test]
    fn test_dyn_allocator() {
        fn numbers(range: Range<i32>) -> impl ExactSizeIterator<Item = String> {
            range.map(|i| i.to_string())
        }

        fn test<B: BumpAllocator>(bump: B) {
            const ITEM_SIZE: usize = size_of::<String>();
            assert_eq!(bump.any_stats().allocated(), 0);
            let mut vec = extra::vec_from_iter_in(numbers(1..4), &bump);
            assert_eq!(vec, ["1", "2", "3"]);
            assert_eq!(vec.len(), 3);
            assert_eq!(vec.capacity(), 4); // because of minimum capacity
            assert_eq!(bump.any_stats().allocated(), 4 * ITEM_SIZE);
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
            let vec = extra::vec_from_iter_in(numbers(1..2), &bump);
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

        Bump::new().scoped(|bump| test::<&BumpScope>(bump));
        Bump::new().scoped(|bump| test::<&mut BumpScope>(bump));
    }
}
