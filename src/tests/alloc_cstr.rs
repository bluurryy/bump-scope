use core::fmt;

use crate::{Bump, alloc::Global};

use super::either_way;

either_way! {
    simple
    from_str
    empty
    fmt
    interior_null_from_str
    interior_null_fmt
    interior_null_fmt_mut
}

fn simple<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let input = c"123456789";
    let allocated = bump.alloc_cstr(input);
    assert_eq!(allocated, input);
    assert_eq!(bump.stats().allocated(), 10);
}

fn from_str<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let input = "123456789";
    let expected = c"123456789";
    let allocated = bump.alloc_cstr_from_str(input);
    assert_eq!(allocated, expected);
    assert_eq!(bump.stats().allocated(), 10);
}

fn empty<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let input = c"";
    let allocated = bump.alloc_cstr(input);
    assert_eq!(allocated, input);
    assert_eq!(bump.stats().allocated(), 1);
}

fn fmt<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let allocated = bump.alloc_cstr_fmt(format_args!("1 + 2 = {}", 1 + 2));
    assert_eq!(allocated, c"1 + 2 = 3");
    assert_eq!(bump.stats().allocated(), 10);
}

fn interior_null_from_str<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let input = "hello\0world";
    let allocated = bump.alloc_cstr_from_str(input);
    assert_eq!(allocated, c"hello");
    assert_eq!(bump.stats().allocated(), 6);
}

fn interior_null_fmt<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let hello = "hello";
    let world = "world";
    let allocated = bump.alloc_cstr_fmt(assert_multiple(format_args!("{hello}\0{world}")));
    assert_eq!(allocated, c"hello");
    assert_eq!(allocated.to_bytes(), b"hello");
    assert_eq!(bump.stats().allocated(), 6);
}

fn interior_null_fmt_mut<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::new();
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
