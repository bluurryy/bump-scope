use super::either_way;
use crate::{bump_format, mut_bump_format, Bump};
use allocator_api2::alloc::{AllocError, Allocator, Global};
use core::{
    alloc::Layout,
    fmt::{Debug, Display},
};
use std::ptr::NonNull;

fn nothing<const UP: bool>() {
    struct Nothing;

    impl Display for Nothing {
        fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            Ok(())
        }
    }

    let mut bump: Bump<Global, 1, UP> = Bump::new();

    bump.alloc_fmt(format_args!("{Nothing}"));
    bump.alloc_fmt_mut(format_args!("{Nothing}"));

    assert_eq!(bump.stats().allocated(), 0);
}

fn nothing_extra<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let string = bump.alloc_fmt(format_args!("ext{Nothing}ra"));
    assert_eq!(string, "extra");
    assert_eq!(bump.stats().allocated(), 5);
}

fn nothing_extra_mut<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::new();
    let string = bump.alloc_fmt_mut(format_args!("ext{Nothing}ra"));
    assert_eq!(string, "extra");
    drop(string);
    assert_eq!(bump.stats().allocated(), 5);
}

struct Nothing;

impl Display for Nothing {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

fn three<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    bump.alloc_fmt(format_args!("{}", 3.1));
    assert_eq!(bump.stats().allocated(), 3);
}

fn three_mut<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::new();
    bump.alloc_fmt_mut(format_args!("{}", 3.1));
    assert_eq!(bump.stats().allocated(), 3);
}

struct ErrorsOnFmt;

impl Display for ErrorsOnFmt {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Err(core::fmt::Error)
    }
}

fn trait_panic<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    bump.alloc_fmt(format_args!("{ErrorsOnFmt}"));
}

fn trait_panic_mut<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::new();
    bump.alloc_fmt_mut(format_args!("{ErrorsOnFmt}"));
}

fn format_trait_panic<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    bump_format!(in &bump, "{ErrorsOnFmt}");
}

fn format_trait_panic_mut<const UP: bool>() {
    let mut bump: Bump<Global, 1, UP> = Bump::new();
    mut_bump_format!(in &mut bump, "{ErrorsOnFmt}");
}

either_way! {
    nothing
    nothing_extra
    nothing_extra_mut
    three
    three_mut

    #[should_panic = "formatting trait implementation returned an error"]
    trait_panic

    #[should_panic = "formatting trait implementation returned an error"]
    trait_panic_mut

    #[should_panic = "formatting trait implementation returned an error"]
    format_trait_panic

    #[should_panic = "formatting trait implementation returned an error"]
    format_trait_panic_mut
}
