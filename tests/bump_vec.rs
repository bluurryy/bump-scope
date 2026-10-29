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

use bump_scope::{Bump, BumpScope, config};

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
        assert_eq!(vec.capacity(), 3);
        assert_eq!(bump.stats().allocated(), 4 * 4);

        // Setting the capacity to 0 constitutes a deallocate call, which we didn't disable.
        // However only when downwards bumping can we deallocate the vec because the vec's capacity has been changed to 3.
        // When upwards bumping we check if `bump_pos == ptr + cap` which now isn't true anymore.
        // When downwards bumping we just check `bump_pos == ptr` which is still true, however the allocation
        // is now only 12 bytes, so 4 bytes remain.
        dbg!(vec.len(), vec.capacity());
        vec.clear();
        dbg!(vec.len(), vec.capacity());
        vec.shrink_to_fit();
        dbg!(vec.len(), vec.capacity());
        assert_eq!(vec.capacity(), 0);
        assert_eq!(bump.stats().allocated(), if config::read::upwards(C) {
            16
        } else {
            4
        });
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
        let slice = extra::vec_into_slice(extra::detach_allocator(vec).0);
        assert_eq!(bump.stats().allocated(), 5 * 4);
        _ = slice;
    }

    #[test]
    fn map_same_layout() {
        let elements = [0, 1, 2, 3, 4].map(|i| i.to_string());

        for panic_on in 0..elements.len() + 1 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let a = extra::vec_from_iter_in::<String, _>(elements.clone(), &bump);
                assert_eq!(a.capacity(), elements.len());
                assert_eq!(bump.stats().allocated(), size_of::<String>() * elements.len());

                let mut i = 0;
                let b: Vec<Option<String>, _> = extra::vec_map(a, |s| {
                    if i == panic_on {
                        panic!("oh no");
                    }

                    let value = format!("hello: {s}");
                    i += 1;
                    Some(value)
                });
                assert_eq!(b.capacity(), elements.len());
                assert_eq!(bump.stats().allocated(), size_of::<Option<String>>() * elements.len());
            });

            if panic_on == elements.len() {
                expect_no_panic(result);
            } else {
                assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
            }

            assert_eq!(bump.stats().allocated(), 0);
        }
    }

    #[test]
    fn map_smaller_layout() {
        for panic_on in 0..7 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let mut i = 1;
                let a = extra::vec_from_iter_in::<String, _>([1, 2, 3, 4, 5, 6].map(|i| i.to_string()), &bump);
                assert_eq!(a.capacity(), 6);
                assert_eq!(bump.stats().allocated(), size_of::<String>() * 6);
                let b: Vec<Box<str>, _> = extra::vec_map(a, |s| {
                    if i == panic_on {
                        panic!("oh no");
                    }

                    let value = format!("hello: {s}");
                    i += 1;
                    value.into()
                });
                assert_eq!(b.capacity(), 9); // (6 * 3) / 2
                assert_eq!(bump.stats().allocated(), size_of::<String>() * 6);
                dbg!(b);
            });

            if panic_on == 0 {
                expect_no_panic(result);
            } else {
                assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
            }

            // The size of 6 Strings and 9 Box<str> happen to be exactly the same, which makes
            // deallocation work.
            assert_eq!(bump.stats().allocated(), 0);
        }
    }

    #[test]
    fn map_bigger_layout() {
        let elements = [0, 1, 2, 3, 4, 5].map(|i| i.to_string().into());

        for panic_on in 0..elements.len() + 1 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let a = extra::vec_from_iter_in::<Box<str>, _>(elements.clone(), &bump);
                assert_eq!(a.capacity(), elements.len());
                assert_eq!(bump.stats().allocated(), size_of::<Box<str>>() * elements.len());

                let mut i = 0;
                let b: Vec<String, _> = extra::vec_map(a, |s| {
                    if i == panic_on {
                        panic!("oh no");
                    }

                    let value = format!("hello: {s}");
                    i += 1;
                    value
                });
                assert_eq!(b.capacity(), elements.len());
                assert_eq!(bump.stats().allocated(), size_of::<Box<str>>() * elements.len() + size_of::<String>() * elements.len());
            });

            if panic_on == elements.len() {
                expect_no_panic(result);
            } else {
                assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
            }

            assert_eq!(
                bump.stats().allocated(),
                if panic_on == elements.len() { size_of::<Box<str>>() * elements.len() } else { 0 },
                "panic_on={panic_on}"
            );
        }
    }

    #[test]
    fn map_to_zst() {
        let elements = [0, 1, 2, 3, 4].map(|i| i.to_string());

        for panic_on in 0..elements.len() + 1 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let a = extra::vec_from_iter_in::<String, _>(elements.clone(), &bump);
                assert_eq!(a.capacity(), elements.len());
                assert_eq!(bump.stats().allocated(), size_of::<String>() * elements.len());

                let mut i = 0;
                let b: Vec<(), _> = extra::vec_map(a, |_| {
                    if i == panic_on {
                        panic!("oh no");
                    }

                    i += 1;
                });
                assert_eq!(b.capacity(), usize::MAX);
                assert_eq!(bump.stats().allocated(), 0);
            });

            if panic_on == elements.len() {
                expect_no_panic(result);
            } else {
                assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
            }

            assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
        }
    }

    #[test]
    fn map_from_zst() {
        let elements = [(), (), (), (), ()];

        for panic_on in 0..elements.len() + 1 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let a = extra::vec_from_iter_in::<(), _>(elements, &bump);
                assert_eq!(a.capacity(), usize::MAX);
                assert_eq!(bump.stats().allocated(), 0);

                let mut i = 0;
                let b: Vec<String, _> = extra::vec_map(a, |()| {
                    if i == panic_on {
                        panic!("oh no");
                    }

                    i += 1;
                    "hello".into()
                });
                assert_eq!(b.capacity(), elements.len());
                assert_eq!(bump.stats().allocated(), size_of::<String>() * elements.len());
            });

            if panic_on == elements.len() {
                expect_no_panic(result);
            } else {
                assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
            }

            assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
        }
    }

    #[test]
    fn map_from_zst_to_zst() {
        let elements = [(), (), (), (), ()];

        for panic_on in 0..elements.len() + 1 {
            let bump: Bump<Global, C> = Bump::new();

            let result = std::panic::catch_unwind(|| {
                let a = extra::vec_from_iter_in::<(), _>(elements, &bump);
                assert_eq!(a.capacity(), usize::MAX);
                assert_eq!(bump.stats().allocated(), 0);

                let mut i = 0;
                let b: Vec<(), _> = extra::vec_map(a, |()| {
                    if i == panic_on {
                        panic!("oh no");
                    }

                    i += 1;
                });
                assert_eq!(b.capacity(), usize::MAX);
                assert_eq!(bump.stats().allocated(), 0);
            });

            if panic_on == elements.len() {
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

            let vec = extra::vec_from_iter_exact_in(numbers(1..2), &bump);
            assert_eq!(vec, ["1"]);
            assert_eq!(vec.len(), 1);
            assert_eq!(vec.capacity(), 1);
            assert_eq!(bump.any_stats().allocated(), ITEM_SIZE );
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
