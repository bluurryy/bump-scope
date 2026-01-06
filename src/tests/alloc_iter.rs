use crate::{alloc::Global, settings::BumpSettings, tests::Bump};

use super::either_way;

fn zst<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    bump.alloc_iter([[0u8; 0]; 10]);
    bump.alloc_iter([[0u16; 0]; 10]);
    bump.alloc_iter([[0u32; 0]; 10]);
    bump.alloc_iter([[0u64; 0]; 10]);

    bump.alloc_iter_mut([[0u8; 0]; 10]);
    bump.alloc_iter_mut([[0u16; 0]; 10]);
    bump.alloc_iter_mut([[0u32; 0]; 10]);
    bump.alloc_iter_mut([[0u64; 0]; 10]);

    bump.alloc_iter_mut_rev([[0u8; 0]; 10]);
    bump.alloc_iter_mut_rev([[0u16; 0]; 10]);
    bump.alloc_iter_mut_rev([[0u32; 0]; 10]);
    bump.alloc_iter_mut_rev([[0u64; 0]; 10]);

    assert_eq!(bump.stats().allocated(), 0);
}

fn empty<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    bump.alloc_iter(core::iter::empty::<u8>());
    bump.alloc_iter(core::iter::empty::<u16>());
    bump.alloc_iter(core::iter::empty::<u32>());
    bump.alloc_iter(core::iter::empty::<u64>());

    bump.alloc_iter_mut(core::iter::empty::<u8>());
    bump.alloc_iter_mut(core::iter::empty::<u16>());
    bump.alloc_iter_mut(core::iter::empty::<u32>());
    bump.alloc_iter_mut(core::iter::empty::<u64>());

    bump.alloc_iter_mut_rev(core::iter::empty::<u8>());
    bump.alloc_iter_mut_rev(core::iter::empty::<u16>());
    bump.alloc_iter_mut_rev(core::iter::empty::<u32>());
    bump.alloc_iter_mut_rev(core::iter::empty::<u64>());

    assert_eq!(bump.stats().allocated(), 0);
}

fn three<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    bump.alloc_iter(SuppressHints([1, 2, 3].into_iter()));
    assert_eq!(bump.stats().allocated(), 3 * 4);
}

fn three_mut<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    bump.alloc_iter_mut(SuppressHints([1, 2, 3].into_iter()));
    assert_eq!(bump.stats().allocated(), 3 * 4);
}

fn three_mut_rev<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    bump.alloc_iter_mut_rev(SuppressHints([1, 2, 3].into_iter()));
    assert_eq!(bump.stats().allocated(), 3 * 4);
}

// so as not to give `BumpVec` the correct capacity before iteration via `size_hint`
struct SuppressHints<T>(T);

impl<T: Iterator> Iterator for SuppressHints<T> {
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        T::next(&mut self.0)
    }
}

either_way! {
    zst
    empty
    three
    three_mut
    three_mut_rev
}
