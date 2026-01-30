#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::string::{String, ToString};

use bump_scope::{Bump, alloc::Global, settings::BumpSettings};

macro_rules! either_way {
    ($($(#[$attr:meta])* $ident:ident)*) => {
        $(
            mod $ident {
                #[test]
                $(#[$attr])*
                fn up() {
                    std::eprintln!("`UP` is `true`");
                    super::$ident::<true>();
                }

                #[test]
                $(#[$attr])*
                fn down() {
                    std::eprintln!("`UP` is `false`");
                    super::$ident::<false>();
                }
            }
        )*
    };
}

fn zst<const UP: bool>() {
    const ZST: [u64; 0] = [0u64; 0];

    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    bump.alloc_slice_copy(&[ZST; 10]);
    bump.alloc_slice_clone(&[ZST; 10]);
    bump.alloc_slice_fill(10, ZST);
    bump.alloc_slice_fill_with(10, || ZST);

    assert_eq!(bump.stats().allocated(), 0);
}

fn empty<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    bump.alloc_slice_copy::<u64>(&[]);
    bump.alloc_slice_clone::<String>(&[]);
    bump.alloc_slice_fill_with(0, || -> String { panic!("should not happen") });
    bump.alloc_slice_fill(0, 42u64);
    bump.alloc_slice_fill(0, &"hello".to_string());
    bump.alloc_slice_fill_with(0, String::default);

    assert_eq!(bump.stats().allocated(), 0);
}

fn overflow<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    bump.alloc_slice_fill_with(usize::MAX, u64::default);
}

either_way! {
    zst
    empty

    #[should_panic(expected = "capacity overflow")]
    overflow
}
