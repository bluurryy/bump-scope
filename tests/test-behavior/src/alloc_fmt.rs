use std::fmt::Display;

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

        let mut bump: Bump<C> = Bump::new();

        bump.alloc_fmt(format_args!("{Nothing}")).into_boxu();
        bump.mut_alloc_fmt(format_args!("{Nothing}")).into_boxu();

        assert_eq!(bump.allocated(), 0);
    }

    #[test]
    fn nothing_extra() {
        let bump: Bump<C> = Bump::new();
        let string = bump.alloc_fmt(format_args!("ext{Nothing}ra")).into_box();
        assert_eq!(&*string, "extra");
        assert_eq!(bump.allocated(), 5);
        drop(string);
        assert_eq!(bump.allocated(), 0);
    }

    #[test]
    fn nothing_extra_mut() {
        let mut bump: Bump<C> = Bump::new();
        let bump = bump.as_mut_bump_scope();
        let string = bump.mut_alloc_fmt(format_args!("ext{Nothing}ra")).into_box();
        assert_eq!(&*string, "extra");
        assert_eq!(Box::allocator(&string).allocated(), 5);
        drop(string);
        assert_eq!(bump.allocated(), 0);
    }

    #[test]
    fn three() {
        let bump: Bump<C> = Bump::new();
        let string = bump.alloc_fmt(format_args!("{}", 3.1)).into_box();
        assert_eq!(&*string, "3.1");
        assert_eq!(bump.allocated(), 3);
        drop(string);
        assert_eq!(bump.allocated(), 0);
    }

    #[test]
    fn three_mut() {
        let mut bump: Bump<C> = Bump::new();
        let string = bump.mut_alloc_fmt(format_args!("{}", 3.1)).into_box();
        assert_eq!(&*string, "3.1");
        assert_eq!(Box::allocator(&string).allocated(), 3);
        drop(string);
        assert_eq!(bump.allocated(), 0);
    }

    #[test]
    #[should_panic = "formatting trait implementation returned an error"]
    fn trait_panic() {
        let bump: Bump<C> = Bump::new();
        bump.alloc_fmt(format_args!("{ErrorsOnFmt}")).into_boxu();
    }

    #[test]
    #[should_panic = "formatting trait implementation returned an error"]
    fn trait_panic_mut() {
        let mut bump: Bump<C> = Bump::new();
        bump.mut_alloc_fmt(format_args!("{ErrorsOnFmt}")).into_boxu();
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
