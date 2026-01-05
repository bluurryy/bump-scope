use core::alloc::Layout;

use crate::{
    BumpAllocator, BumpAllocatorExt,
    alloc::{Allocator, Global},
    settings::{BumpAllocatorSettings, BumpSettings},
    tests::either_way,
};

either_way! {
    allocated
    unallocated
    unallocated_alloc
    guaranteed_allocated
    allocated_reserve_bytes
    unallocated_reserve_bytes
    checkpoint
    checkpoint_multiple_chunks
    allocate_zero_layout
    reset
    non_default_base_allocator
}

type Bump<const UP: bool, A = Global> = crate::Bump<A, BumpSettings<1, UP, false, true>>;

fn allocated<const UP: bool>() {
    let bump = <Bump<UP>>::new();
    drop(bump);
}

fn unallocated<const UP: bool>() {
    let bump = <Bump<UP>>::unallocated();
    assert_eq!(bump.stats().count(), 0);
    drop(bump);
}

fn unallocated_alloc<const UP: bool>() {
    let bump = <Bump<UP>>::unallocated();
    assert_eq!(bump.stats().count(), 0);
    bump.alloc_str("Hello, World!");
    assert_eq!(bump.stats().count(), 1);
    drop(bump);
}

fn guaranteed_allocated<const UP: bool>() {
    let bump = <Bump<UP>>::unallocated();
    assert_eq!(bump.stats().count(), 0);
    let bump = bump.into_guaranteed_allocated(crate::Bump::new);
    assert_eq!(bump.stats().count(), 1);
    drop(bump);
}

fn allocated_reserve_bytes<const UP: bool>() {
    let bump = <Bump<UP>>::new();
    assert_eq!(bump.stats().count(), 1);
    bump.reserve_bytes(1024);
    assert_eq!(bump.stats().count(), 2);
    assert!(bump.stats().capacity() >= 1024);
    drop(bump);
}

fn unallocated_reserve_bytes<const UP: bool>() {
    let bump = <Bump<UP>>::unallocated();
    assert_eq!(bump.stats().count(), 0);
    bump.reserve_bytes(1024);
    assert_eq!(bump.stats().count(), 1);
    assert!(bump.stats().capacity() >= 1024);
    drop(bump);
}

fn checkpoint<const UP: bool>() {
    let bump = <Bump<UP>>::unallocated();

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
    let bump = <Bump<UP>>::unallocated();

    assert_eq!(bump.stats().count(), 0);
    let c0 = bump.checkpoint();

    allocate_another_chunk(&bump, 1);
    allocate_another_chunk(&bump, 2);
    let c2 = bump.checkpoint();

    allocate_another_chunk(&bump, 3);
    allocate_another_chunk(&bump, 4);

    assert_eq!(bump.stats().get_current_chunk().unwrap().iter_prev().count(), 3);
    unsafe { bump.reset_to(c2) };
    assert_eq!(bump.stats().get_current_chunk().unwrap().iter_prev().count(), 1);
    unsafe { bump.reset_to(c0) };
    assert_eq!(bump.stats().get_current_chunk().unwrap().iter_prev().count(), 0);
    assert_eq!(bump.stats().count(), 4);
    std::dbg!(bump.stats());
}

fn allocate_another_chunk<const UP: bool>(bump: &Bump<UP>, dummy_size: usize) {
    let start_chunks = bump.any_stats().count();
    let remaining = bump.any_stats().remaining();
    bump.alloc_layout(Layout::from_size_align(remaining, 1).unwrap());
    assert_eq!(bump.any_stats().count(), start_chunks);
    bump.alloc_layout(Layout::from_size_align(dummy_size, 1).unwrap());
    assert_eq!(bump.any_stats().count(), start_chunks + 1);
    assert_eq!(bump.any_stats().current_chunk().unwrap().allocated(), dummy_size);
}

fn allocate_zero_layout<const UP: bool>() {
    let bump = <Bump<UP>>::unallocated();
    bump.alloc_layout(Layout::new::<()>());
}

fn reset<const UP: bool>() {
    let mut bump = <Bump<UP>>::unallocated();
    bump.reset();
}

fn non_default_base_allocator<const UP: bool>() {
    #[derive(Clone)]
    struct MyAllocator {}

    unsafe impl Allocator for MyAllocator {
        fn allocate(&self, layout: Layout) -> Result<core::ptr::NonNull<[u8]>, crate::alloc::AllocError> {
            Global.allocate(layout)
        }

        unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: Layout) {
            unsafe { Global.deallocate(ptr, layout) };
        }
    }

    let bump = <Bump<UP, MyAllocator>>::unallocated();

    #[cfg(false)] // method is not available
    bump.alloc_str("test");

    // but this works
    let test = bump
        .as_guaranteed_allocated(|| crate::Bump::new_in(MyAllocator {}))
        .alloc_str("test");

    assert_eq!(test, "test");
}
