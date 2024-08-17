use crate::Bump;
use allocator_api2::alloc::Global;

use super::either_way;

fn zst<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();

    bump.alloc_iter([[0u8; 0]]);
    bump.alloc_iter([[0u16; 0]]);
    bump.alloc_iter([[0u32; 0]]);
    bump.alloc_iter([[0u64; 0]]);

    assert_eq!(bump.stats().allocated(), 0);
}

fn empty<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();

    bump.alloc_iter(core::iter::empty::<u8>());
    bump.alloc_iter(core::iter::empty::<u16>());
    bump.alloc_iter(core::iter::empty::<u32>());
    bump.alloc_iter(core::iter::empty::<u64>());

    assert_eq!(bump.stats().allocated(), 0);
}

// test to not take up unnecessary memory when using `alloc_iter`
fn three<const UP: bool>() {
    // so as not to give `BumpVec` the correct capacity before iteration via `size_hint`
    struct SuppressHints<T>(T);

    impl<T: Iterator> Iterator for SuppressHints<T> {
        type Item = T::Item;

        fn next(&mut self) -> Option<Self::Item> {
            T::next(&mut self.0)
        }
    }

    let bump: Bump<Global, 1, UP> = Bump::new();

    bump.alloc_iter(SuppressHints([1, 2, 3].into_iter()));

    assert_eq!(bump.stats().allocated(), 3 * 4);
}

either_way! {
    zst
    empty
    three
}
