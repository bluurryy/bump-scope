use core::alloc::Layout;

use crate::{
    Bump,
    alloc::{Allocator, Global},
    tests::either_way,
};

either_way! {
    test_aligned_alloc
    test_aligned_allocate
    test_scoped_alloc
    test_scoped_allocate
    test_scoped_aligned_alloc
    test_scoped_aligned_allocate
}

fn test_aligned_alloc<const UP: bool>() {
    let mut bump = <Bump<Global, 1, UP>>::new();
    let bump = bump.as_mut_scope();

    let _three = bump.alloc([0u8; 3]);
    let five = bump.alloc([0u8; 5]);

    bump.aligned::<8, ()>(|bump| {
        bump.dealloc(five);
        // deallocation succeeds and the bump pointer is unaligned
        bump.alloc(0u64);
    });
}

fn test_aligned_allocate<const UP: bool>() {
    let mut bump = <Bump<Global, 1, UP>>::new();
    let bump = bump.as_mut_scope();

    let _three = bump.allocate(Layout::new::<[u8; 3]>()).unwrap();
    let five = bump.allocate(Layout::new::<[u8; 5]>()).unwrap();

    bump.aligned::<8, ()>(|bump| {
        unsafe { bump.deallocate(five.cast(), Layout::new::<[u8; 5]>()) };
        bump.alloc(0u64);
    });
}

fn test_scoped_alloc<const UP: bool>() {
    let mut bump = <Bump<Global, 1, UP>>::new();
    let bump = bump.as_mut_scope();

    let _three = bump.alloc([0u8; 3]);
    let five = bump.alloc([0u8; 5]);

    bump.scoped::<()>(|bump| {
        bump.dealloc(five);
        // deallocation succeeds and the bump pointer is unaligned
        bump.alloc(0u64);
    });
}

fn test_scoped_allocate<const UP: bool>() {
    let mut bump = <Bump<Global, 1, UP>>::new();
    let bump = bump.as_mut_scope();

    let _three = bump.allocate(Layout::new::<[u8; 3]>()).unwrap();
    let five = bump.allocate(Layout::new::<[u8; 5]>()).unwrap();

    bump.scoped::<()>(|bump| {
        unsafe { bump.deallocate(five.cast(), Layout::new::<[u8; 5]>()) };
        bump.alloc(0u64);
    });
}

fn test_scoped_aligned_alloc<const UP: bool>() {
    let mut bump = <Bump<Global, 1, UP>>::new();
    let bump = bump.as_mut_scope();

    let _three = bump.alloc([0u8; 3]);
    let five = bump.alloc([0u8; 5]);

    bump.scoped_aligned::<8, ()>(|bump| {
        bump.dealloc(five);
        // deallocation succeeds and the bump pointer is unaligned
        bump.alloc(0u64);
    });
}

fn test_scoped_aligned_allocate<const UP: bool>() {
    let mut bump = <Bump<Global, 1, UP>>::new();
    let bump = bump.as_mut_scope();

    let _three = bump.allocate(Layout::new::<[u8; 3]>()).unwrap();
    let five = bump.allocate(Layout::new::<[u8; 5]>()).unwrap();

    bump.scoped_aligned::<8, ()>(|bump| {
        unsafe { bump.deallocate(five.cast(), Layout::new::<[u8; 5]>()) };
        bump.alloc(0u64);
    });
}
