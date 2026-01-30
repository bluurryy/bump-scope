#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::alloc::Layout;

use bump_scope::{Bump, traits::BumpAllocatorTyped as _};

#[test]
fn test_reserve() {
    // Create a bump allocator with two chunks.
    //
    // The first chunk is the current chunk with 3 allocated bytes.
    //
    // The second chunk has its bump pointer at its end, making it return `0` from `remaining`.
    // This `remaining` property is meaningless for non-current chunks and the second chunk's
    // whole capacity must be subtracted from the amount of bytes to reserve.

    let bump: Bump = Bump::new();
    assert_eq!(bump.stats().count(), 0);

    // allocate the first chunk and fill it
    bump.reserve(1);
    assert_eq!(bump.stats().count(), 1);

    // allocate 3 bytes
    bump.allocate_layout(Layout::new::<[u8; 3]>());
    let checkpoint = bump.checkpoint();

    // fill the first chunk
    bump.allocate_layout(Layout::array::<u8>(bump.stats().current_chunk().unwrap().remaining()).unwrap());
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(bump.stats().remaining(), 0);

    // allocate the second chunk
    bump.reserve(1);
    assert_eq!(bump.stats().count(), 2);

    // fill the second chunk
    bump.allocate_layout(Layout::array::<u8>(bump.stats().big_to_small().next().unwrap().remaining()).unwrap());
    assert_eq!(bump.stats().count(), 2);
    assert_eq!(bump.stats().remaining(), 0);

    // reset to the checkpoint when 3 bytes are allocated on the first chunk
    unsafe { bump.reset_to(checkpoint) };

    // reserving `remaining` bytes should not allocate another chunk
    let remaining = bump.stats().remaining();
    bump.reserve(remaining);
    assert_eq!(bump.stats().count(), 2);

    // reserving `remaining + 1` bytes should allocate another chunk
    bump.reserve(remaining + 1);
    assert_eq!(bump.stats().count(), 3);
}
