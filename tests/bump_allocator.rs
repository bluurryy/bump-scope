#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::alloc::{Allocator as _, Layout};

use bump_scope::Bump;

/// Checks that a bigger chunk is correctly allocated when the current chunk is not largest one
#[test]
fn alloc_chunks() {
    let mut a: Bump = Bump::new();

    assert_eq!(a.stats().count(), 1);

    a.scoped(|a| {
        // alloc is large enough to require an additional chunk
        _ = a.allocate(Layout::array::<u8>(1 << 9).unwrap());
        assert_eq!(a.stats().count(), 2);
    });

    a.scoped(|a| {
        // alloc is larger than the existing chunks and requires a third chunk to be allocated
        _ = a.allocate(Layout::array::<u8>(1 << 10).unwrap());
        assert_eq!(a.stats().count(), 3);
    });
}
