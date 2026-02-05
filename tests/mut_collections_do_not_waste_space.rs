#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::iter;

use bump_scope::{Bump, MutBumpVec, MutBumpVecRev, alloc::Global, settings::BumpSettings};

macro_rules! either_way {
    ($($(#[$attr:meta])* $ident:ident)*) => {
        mod up {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    std::eprintln!("`UP` is `true`");
                    super::$ident::<true>();
                }
            )*
        }

        mod down {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    std::eprintln!("`UP` is `false`");
                    super::$ident::<false>();
                }
            )*
        }
    };
}

fn vec<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    assert_eq!(bump.stats().size(), 512 - size_of::<[usize; 2]>());

    for size in [0, 100, 200, 300, 400] {
        bump.reset();

        let mut vec: MutBumpVec<u8, _> = MutBumpVec::new_in(&mut bump);
        vec.extend(iter::repeat_n(0, size));
        assert_eq!(vec.allocator_stats().allocated(), 0); // `Mut*` allocations don't bump the pointer
        _ = vec.into_slice();
        assert_eq!(bump.stats().allocated(), size);
    }
}

fn vec_rev<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    assert_eq!(bump.stats().size(), 512 - size_of::<[usize; 2]>());

    for size in [0, 100, 200, 300, 400] {
        bump.reset();

        let mut vec: MutBumpVecRev<u8, _> = MutBumpVecRev::new_in(&mut bump);
        vec.extend(iter::repeat_n(0, size));
        assert_eq!(vec.allocator_stats().allocated(), 0); // `Mut*` allocations don't bump the pointer
        _ = vec.into_slice();
        assert_eq!(bump.stats().allocated(), size);
    }
}

either_way! {
    vec
    vec_rev
}
