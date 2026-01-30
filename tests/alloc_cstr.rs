#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::fmt;

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

either_way! {
    test_simple
    test_from_str
    test_empty
    test_fmt
    test_interior_null_from_str
    test_interior_null_fmt
    test_interior_null_fmt_mut
}

fn test_simple<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let input = c"123456789";
    let allocated = bump.alloc_cstr(input);
    assert_eq!(allocated, input);
    assert_eq!(bump.stats().allocated(), 10);
}

fn test_from_str<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let input = "123456789";
    let expected = c"123456789";
    let allocated = bump.alloc_cstr_from_str(input);
    assert_eq!(allocated, expected);
    assert_eq!(bump.stats().allocated(), 10);
}

fn test_empty<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let input = c"";
    let allocated = bump.alloc_cstr(input);
    assert_eq!(allocated, input);
    assert_eq!(bump.stats().allocated(), 1);
}

fn test_fmt<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let allocated = bump.alloc_cstr_fmt(format_args!("1 + 2 = {}", 1 + 2));
    assert_eq!(allocated, c"1 + 2 = 3");
    assert_eq!(bump.stats().allocated(), 10);
}

fn test_interior_null_from_str<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let input = "hello\0world";
    let allocated = bump.alloc_cstr_from_str(input);
    assert_eq!(allocated, c"hello");
    assert_eq!(bump.stats().allocated(), 6);
}

fn test_interior_null_fmt<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let hello = "hello";
    let world = "world";
    let allocated = bump.alloc_cstr_fmt(assert_multiple(format_args!("{hello}\0{world}")));
    assert_eq!(allocated, c"hello");
    assert_eq!(allocated.to_bytes(), b"hello");
    assert_eq!(bump.stats().allocated(), 6);
}

fn test_interior_null_fmt_mut<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let hello = "hello";
    let world = "world";
    let allocated = bump.alloc_cstr_fmt_mut(assert_multiple(format_args!("{hello}\0{world}")));
    assert_eq!(allocated, c"hello");
    assert_eq!(allocated.to_bytes(), b"hello");
    assert_eq!(bump.stats().allocated(), 6);
}

fn assert_multiple(args: fmt::Arguments) -> fmt::Arguments {
    assert!(args.as_str().is_none(), "expected multiple format arguments");
    args
}
