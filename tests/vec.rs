#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{alloc::Global, boxed::Box, vec::Vec};

use bump_scope::Bump;

use common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn grow() {
        let mut bump: Bump<Global, C> = Bump::new();

        let mut vec = Vec::<i32, _>::new_in(&bump);
        assert_eq!(bump.stats().allocated(), 0);

        vec.reserve_exact(1);
        assert_eq!(bump.stats().allocated(), 4);

        vec.reserve_exact(2);
        assert_eq!(bump.stats().allocated(), 8);

        bump.alloc_uninit::<u8>();
        assert_eq!(bump.stats().allocated(), 9);

        vec.reserve_exact(3);
        assert_eq!(bump.stats().allocated(), 12 + 3 * 4);

        drop(vec);
        assert_eq!(bump.stats().allocated(), 12);

        bump.reset();
        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn shrink() {
        let bump: Bump<Global, C> = Bump::new();

        let mut vec = Vec::<i32, _>::new_in(&bump);
        assert_eq!(bump.stats().allocated(), 0);

        vec.reserve_exact(2);
        assert_eq!(bump.stats().allocated(), 8);

        vec.shrink_to(1);
        assert_eq!(bump.stats().allocated(), 4);

        let boxed: Box<_, _> = bump.alloc_uninit::<u8>().into_box();
        assert_eq!(bump.stats().allocated(), 5);

        drop(boxed);
        assert_eq!(bump.stats().allocated(), 4);

        vec.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn shrinknt() {
        let bump: Bump<Global, C> = Bump::new();

        let mut vec = Vec::<i32, _>::new_in(&bump);
        assert_eq!(bump.stats().allocated(), 0);

        vec.reserve_exact(2);
        assert_eq!(bump.stats().allocated(), 8);

        bump.alloc_uninit::<u8>();
        assert_eq!(bump.stats().allocated(), 9);

        vec.shrink_to(1);
        assert_eq!(bump.stats().allocated(), 9);

        drop(vec);
        assert_eq!(bump.stats().allocated(), 9);
    }
}
