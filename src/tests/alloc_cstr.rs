use allocator_api2::alloc::Global;

use crate::Bump;

use super::either_way;

either_way! {
    simple
    from_str
    empty
    fmt
    interior_null_from_str
    interior_null_fmt
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
    let allocated = bump.alloc_cstr_fmt(format_args!("{}\0{}", "hello", "world"));
    assert_eq!(allocated, c"hello");
    assert_eq!(bump.stats().allocated(), 6);
}
