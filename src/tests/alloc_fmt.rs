use core::fmt::Display;

use crate::Bump;
use allocator_api2::alloc::Global;

use super::either_way;

fn nothing<const UP: bool>() {
    struct Nothing;

    impl Display for Nothing {
        fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            Ok(())
        }
    }

    let bump: Bump<Global, 1, UP> = Bump::new();

    bump.alloc_fmt(format_args!("{}", Nothing));

    assert_eq!(bump.stats().allocated(), 0);
}

fn nothing_extra<const UP: bool>() {
    struct Nothing;

    impl Display for Nothing {
        fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            Ok(())
        }
    }

    let bump: Bump<Global, 1, UP> = Bump::new();

    bump.alloc_fmt(format_args!("ext{}ra", Nothing));

    assert_eq!(bump.stats().allocated(), 5);
}

fn three<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();

    bump.alloc_fmt(format_args!("{}", 3.1));

    assert_eq!(bump.stats().allocated(), 3);
}

either_way! {
    nothing
    nothing_extra
    three
}
