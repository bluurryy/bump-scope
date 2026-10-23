#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{alloc::Global, fmt::Display};

use bump_scope::Bump;

use crate::common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn nothing() {
        struct Nothing;

        impl Display for Nothing {
            fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                Ok(())
            }
        }

        let mut bump: Bump<Global, C> = Bump::new();

        bump.alloc_fmt(format_args!("{Nothing}"));
        bump.alloc_fmt_mut(format_args!("{Nothing}"));

        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn nothing_extra() {
        let bump: Bump<Global, C> = Bump::new();
        let string = bump.alloc_fmt(format_args!("ext{Nothing}ra"));
        assert_eq!(&*string, "extra");
        assert_eq!(bump.stats().allocated(), 5);
    }

    #[test]
    fn nothing_extra_mut() {
        let mut bump: Bump<Global, C> = Bump::new();
        let string = bump.alloc_fmt_mut(format_args!("ext{Nothing}ra"));
        assert_eq!(&*string, "extra");
        drop(string);
        assert_eq!(bump.stats().allocated(), 5);
    }

    #[test]
    fn three() {
        let bump: Bump<Global, C> = Bump::new();
        bump.alloc_fmt(format_args!("{}", 3.1));
        assert_eq!(bump.stats().allocated(), 3);
    }

    #[test]
    fn three_mut() {
        let mut bump: Bump<Global, C> = Bump::new();
        bump.alloc_fmt_mut(format_args!("{}", 3.1));
        assert_eq!(bump.stats().allocated(), 3);
    }

    #[test]
    #[should_panic = "formatting trait implementation returned an error"]
    fn trait_panic() {
        let bump: Bump<Global, C> = Bump::new();
        bump.alloc_fmt(format_args!("{ErrorsOnFmt}"));
    }

    #[test]
    #[should_panic = "formatting trait implementation returned an error"]
    fn trait_panic_mut() {
        let mut bump: Bump<Global, C> = Bump::new();
        bump.alloc_fmt_mut(format_args!("{ErrorsOnFmt}"));
    }

}

struct Nothing;

impl Display for Nothing {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

struct ErrorsOnFmt;

impl Display for ErrorsOnFmt {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Err(core::fmt::Error)
    }
}
