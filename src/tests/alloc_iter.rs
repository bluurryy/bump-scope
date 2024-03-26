use crate::Bump;

#[test]
fn zst() {
    let bump: Bump = Bump::new();

    bump.alloc_iter([[0u8; 0]]);
    bump.alloc_iter([[0u16; 0]]);
    bump.alloc_iter([[0u32; 0]]);
    bump.alloc_iter([[0u64; 0]]);

    assert_eq!(bump.stats().allocated(), 0);
}

#[test]
fn empty() {
    let bump: Bump = Bump::new();

    bump.alloc_iter(core::iter::empty::<u8>());
    bump.alloc_iter(core::iter::empty::<u16>());
    bump.alloc_iter(core::iter::empty::<u32>());
    bump.alloc_iter(core::iter::empty::<u64>());

    assert_eq!(bump.stats().allocated(), 0);
}
