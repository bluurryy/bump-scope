#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{alloc::Global, fmt};

use bump_scope::Bump;

use crate::common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn test_simple() {
        let bump: Bump<Global, C> = Bump::new();
        let input = c"123456789";
        let allocated = bump.alloc_cstr(input).into_ref();
        assert_eq!(allocated, input);
        assert_eq!(bump.stats().allocated(), 10);
    }

    #[test]
    fn test_from_str() {
        let bump: Bump<Global, C> = Bump::new();
        let input = "123456789";
        let expected = c"123456789";
        let allocated = bump.alloc_cstr_from_str(input).into_ref();
        assert_eq!(allocated, expected);
        assert_eq!(bump.stats().allocated(), 10);
    }

    #[test]
    fn test_empty() {
        let bump: Bump<Global, C> = Bump::new();
        let input = c"";
        let allocated = bump.alloc_cstr(input).into_ref();
        assert_eq!(allocated, input);
        assert_eq!(bump.stats().allocated(), 1);
    }

    #[test]
    fn test_fmt() {
        let bump: Bump<Global, C> = Bump::new();
        let allocated = bump.alloc_cstr_fmt(format_args!("1 + 2 = {}", 1 + 2)).into_ref();
        assert_eq!(allocated, c"1 + 2 = 3");
        assert_eq!(bump.stats().allocated(), 10);
    }

    #[test]
    fn test_interior_null_from_str() {
        let bump: Bump<Global, C> = Bump::new();
        let input = "hello\0world";
        let allocated = bump.alloc_cstr_from_str(input).into_ref();
        assert_eq!(allocated, c"hello");
        assert_eq!(bump.stats().allocated(), 6);
    }

    #[test]
    fn test_interior_null_fmt() {
        let bump: Bump<Global, C> = Bump::new();
        let hello = "hello";
        let world = "world";
        let allocated = bump.alloc_cstr_fmt(assert_multiple(format_args!("{hello}\0{world}"))).into_ref();
        assert_eq!(allocated, c"hello");
        assert_eq!(allocated.to_bytes(), b"hello");
        assert_eq!(bump.stats().allocated(), 6);
    }

    #[test]
    fn test_interior_null_fmt_mut() {
        let mut bump: Bump<Global, C> = Bump::new();
        let hello = "hello";
        let world = "world";
        let allocated = bump.alloc_cstr_fmt_mut(assert_multiple(format_args!("{hello}\0{world}"))).into_ref();
        assert_eq!(allocated, c"hello");
        assert_eq!(allocated.to_bytes(), b"hello");
        assert_eq!(bump.stats().allocated(), 6);
    }
}

fn assert_multiple(args: fmt::Arguments) -> fmt::Arguments {
    assert!(args.as_str().is_none(), "expected multiple format arguments");
    args
}
