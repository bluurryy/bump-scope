use core::alloc::Layout;

use crate::{BumpAllocator, BumpAllocatorExt, alloc::Global};

type Bump = crate::Bump<Global, 1, true, false>;

#[test]
fn allocated() {
    let bump: Bump = Bump::new();
    drop(bump);
}

#[test]
fn unallocated() {
    let bump: Bump = Bump::unallocated();
    assert_eq!(bump.stats().count(), 0);
    drop(bump);
}

#[test]
fn unallocated_alloc() {
    let bump: Bump = Bump::unallocated();
    assert_eq!(bump.stats().count(), 0);
    bump.alloc_str("Hello, World!");
    assert_eq!(bump.stats().count(), 1);
    drop(bump);
}

#[test]
fn guaranteed_allocated() {
    let bump: Bump = Bump::unallocated();
    assert_eq!(bump.stats().count(), 0);
    let bump = bump.guaranteed_allocated();
    assert_eq!(bump.stats().count(), 1);
    drop(bump);
}

#[test]
fn allocated_reserve_bytes() {
    let bump: Bump = Bump::new();
    assert_eq!(bump.stats().count(), 1);
    bump.reserve_bytes(1024);
    assert_eq!(bump.stats().count(), 2);
    assert!(bump.stats().capacity() >= 1024);
    drop(bump);
}

#[test]
fn unallocated_reserve_bytes() {
    let bump: Bump = Bump::unallocated();
    assert_eq!(bump.stats().count(), 0);
    bump.reserve_bytes(1024);
    assert_eq!(bump.stats().count(), 1);
    assert!(bump.stats().capacity() >= 1024);
    drop(bump);
}

#[test]
fn checkpoint() {
    let bump: Bump = Bump::unallocated();

    let checkpoint_unallocated = bump.checkpoint();
    assert_eq!(bump.stats().count(), 0);
    assert_eq!(bump.stats().allocated(), 0);

    bump.alloc_str("hello");
    let checkpoint_hello = bump.checkpoint();
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(bump.stats().allocated(), 5);

    bump.alloc_str("world");
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(bump.stats().allocated(), 10);

    unsafe { bump.reset_to(checkpoint_hello) };
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(bump.stats().allocated(), 5);

    unsafe { bump.reset_to(checkpoint_unallocated) };
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(bump.stats().allocated(), 0);
}

#[test]
fn checkpoint_multiple_chunks() {
    let bump: Bump = Bump::unallocated();

    assert_eq!(bump.stats().count(), 0);
    let c0 = bump.checkpoint();

    allocate_another_chunk(&bump, 1);
    allocate_another_chunk(&bump, 2);
    let c2 = bump.checkpoint();

    allocate_another_chunk(&bump, 3);
    allocate_another_chunk(&bump, 4);

    assert_eq!(bump.stats().current_chunk().unwrap().iter_prev().count(), 3);
    unsafe { bump.reset_to(c2) };
    assert_eq!(bump.stats().current_chunk().unwrap().iter_prev().count(), 1);
    unsafe { bump.reset_to(c0) };
    assert_eq!(bump.stats().current_chunk().unwrap().iter_prev().count(), 0);
    assert_eq!(bump.stats().count(), 4);
    std::dbg!(bump.stats());
}

fn allocate_another_chunk(bump: &Bump, dummy_size: usize) {
    let start_chunks = bump.any_stats().count();
    let remaining = bump.any_stats().remaining();
    bump.alloc_layout(Layout::from_size_align(remaining, 1).unwrap());
    assert_eq!(bump.any_stats().count(), start_chunks);
    bump.alloc_layout(Layout::from_size_align(dummy_size, 1).unwrap());
    assert_eq!(bump.any_stats().count(), start_chunks + 1);
    assert_eq!(bump.any_stats().current_chunk().unwrap().allocated(), dummy_size);
}
