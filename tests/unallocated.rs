#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::alloc::Layout;

use bump_scope::{
    alloc::{Allocator, Global},
    settings::BumpSettings,
    traits::{BumpAllocatorCore, BumpAllocatorTyped},
};

use common::{AssumedMallocOverhead, either_way};

either_way! {
    allocated
    unallocated
    unallocated_alloc
    allocated_reserve_bytes
    unallocated_reserve_bytes
    checkpoint
    checkpoint_multiple_chunks
    reset
    potential_data_race
    default_chunk_size
    custom_chunk_size
}

type Bump<const UP: bool, A = Global> = bump_scope::Bump<A, BumpSettings<1, UP, false, true>>;

fn allocated<const UP: bool>() {
    let bump = <Bump<UP>>::new();
    drop(bump);
}

fn unallocated<const UP: bool>() {
    let bump = <Bump<UP>>::new();
    assert_eq!(bump.stats().count(), 0);
    drop(bump);
}

fn unallocated_alloc<const UP: bool>() {
    let bump = <Bump<UP>>::new();
    assert_eq!(bump.stats().count(), 0);
    bump.alloc_str("Hello, World!");
    assert_eq!(bump.stats().count(), 1);
    drop(bump);
}

fn allocated_reserve_bytes<const UP: bool>() {
    let bump = <Bump<UP>>::with_size(512);
    assert_eq!(bump.stats().count(), 1);
    bump.reserve(1024);
    assert_eq!(bump.stats().count(), 2);
    assert!(bump.stats().capacity() >= 1024);
    drop(bump);
}

fn unallocated_reserve_bytes<const UP: bool>() {
    let bump = <Bump<UP>>::new();
    assert_eq!(bump.stats().count(), 0);
    bump.reserve(1024);
    assert_eq!(bump.stats().count(), 1);
    assert!(bump.stats().capacity() >= 1024);
    drop(bump);
}

fn checkpoint<const UP: bool>() {
    let bump = <Bump<UP>>::new();

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

fn checkpoint_multiple_chunks<const UP: bool>() {
    let bump = <Bump<UP>>::new();

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

fn allocate_another_chunk<const UP: bool>(bump: &Bump<UP>, dummy_size: usize) {
    let start_chunks = bump.any_stats().count();

    // fill the existing chunk if there is one
    let remaining = bump.any_stats().remaining();
    if remaining != 0 {
        bump.allocate_layout(Layout::from_size_align(remaining, 1).unwrap());
    }
    assert_eq!(bump.any_stats().count(), start_chunks);

    bump.allocate_layout(Layout::from_size_align(dummy_size, 1).unwrap());
    assert_eq!(bump.any_stats().count(), start_chunks + 1);
    assert_eq!(bump.any_stats().current_chunk().unwrap().allocated(), dummy_size);
}

fn reset<const UP: bool>() {
    let mut bump = <Bump<UP>>::new();
    bump.reset();
}

// if allocating on the dummy chunk would bump it's position field, even by 0
// then this cause a data race
fn potential_data_race<const UP: bool>() {
    fn create_bump_and_alloc<const UP: bool>() {
        let bump = <Bump<UP>>::new();
        bump.allocate(Layout::new::<()>()).unwrap();
    }

    std::thread::spawn(create_bump_and_alloc::<UP>);
    std::thread::spawn(create_bump_and_alloc::<UP>);
}

fn default_chunk_size<const UP: bool>() {
    let bump: bump_scope::Bump<Global, BumpSettings<1, UP>> = bump_scope::Bump::new();
    assert_eq!(bump.stats().count(), 0);
    bump.alloc("a");
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(
        bump.stats().current_chunk().unwrap().size(),
        512 - size_of::<AssumedMallocOverhead>()
    );
}

fn custom_chunk_size<const UP: bool>() {
    let bump: bump_scope::Bump<Global, BumpSettings<1, UP, false, true, true, true, 4096>> = bump_scope::Bump::new();
    assert_eq!(bump.stats().count(), 0);
    bump.alloc("a");
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(
        bump.stats().current_chunk().unwrap().size(),
        4096 - size_of::<AssumedMallocOverhead>()
    );
}
