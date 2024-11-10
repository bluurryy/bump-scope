#![allow(clippy::manual_assert)]
use core::hint::black_box;

use super::either_way;
use crate::{bump_vec, Bump, FixedBumpVec};
use allocator_api2::alloc::Global;

either_way! {
    map_same_layout
    map_smaller_layout
    map_to_zst
    map_from_zst_to_zst
}

fn map_same_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = FixedBumpVec::<String>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: FixedBumpVec<Option<String>> = a.map_in_place(|s| {
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
            result.unwrap();
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), size_of::<String>() * 3, "panic_on={panic_on}");
    }
}

fn map_smaller_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = FixedBumpVec::<String>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: FixedBumpVec<Box<str>> = a.map_in_place(|s| {
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
            result.unwrap();
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), size_of::<String>() * 3, "panic_on={panic_on}");
    }
}

fn map_to_zst<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = FixedBumpVec::<String>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: FixedBumpVec<()> = a.map_in_place(|_| {
                if i == panic_on {
                    panic!("oh no");
                }

                i += 1;
            });
            assert_eq!(b.capacity(), usize::MAX);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            dbg!(b);
        });

        if panic_on == 0 {
            result.unwrap();
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), size_of::<String>() * 3, "panic_on={panic_on}");
    }
}

fn map_from_zst_to_zst<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, 1, UP> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = FixedBumpVec::<()>::from_iter_exact_in([(), (), ()], &bump);
            // FIXME: 3 should be usize::MAX
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), 0);
            let b: FixedBumpVec<()> = a.map_in_place(|()| {
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
            result.unwrap();
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), 0, "panic_on={panic_on}");
    }
}
