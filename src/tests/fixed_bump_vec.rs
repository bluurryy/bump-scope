#![expect(clippy::manual_assert)]

use std::{
    boxed::Box,
    dbg, format,
    hint::black_box,
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    FixedBumpVec,
    alloc::Global,
    bump_vec,
    settings::BumpSettings,
    tests::{Bump, expect_no_panic},
};

use super::either_way;

either_way! {
    map_in_place_same_layout
    map_in_place_smaller_layout
    map_in_place_to_zst
    map_in_place_from_zst_to_zst
}

fn map_in_place_same_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

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
            expect_no_panic(result);
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), size_of::<String>() * 3, "panic_on={panic_on}");
    }
}

fn map_in_place_smaller_layout<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

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
            expect_no_panic(result);
        } else {
            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "oh no");
        }

        assert_eq!(bump.stats().allocated(), size_of::<String>() * 3, "panic_on={panic_on}");
    }
}

fn map_in_place_to_zst<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = FixedBumpVec::<String>::from_iter_exact_in([1, 2, 3].map(|i| i.to_string()), &bump);
            assert_eq!(a.capacity(), 3);
            assert_eq!(bump.stats().allocated(), size_of::<String>() * 3);
            let b: FixedBumpVec<AlignedZst> = a.map_in_place(|_| {
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

        assert_eq!(bump.stats().allocated(), size_of::<String>() * 3, "panic_on={panic_on}");
    }
}

fn map_in_place_from_zst_to_zst<const UP: bool>() {
    for panic_on in 0..4 {
        let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

        let result = std::panic::catch_unwind(|| {
            let mut i = 1;
            let a = FixedBumpVec::<()>::from_iter_exact_in([(), (), ()], &bump);
            assert_eq!(a.capacity(), usize::MAX);
            assert_eq!(bump.stats().allocated(), 0);
            let b: FixedBumpVec<AlignedZst> = a.map_in_place(|()| {
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
