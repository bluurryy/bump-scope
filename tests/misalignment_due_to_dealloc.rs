#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::alloc::{Allocator, Global, Layout};

use bump_scope::Bump;

use crate::common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;
    const C_MIN_ALIGN_8: BumpConfig = BumpConfigBuilder(DEFAULT_C).min_align(8).build();

    #[test]
    fn test_aligned_alloc() {
        let mut bump = Bump::<Global, C>::new();
        let bump = bump.as_mut_scope();

        let _three = bump.alloc([0u8; 3]).into_boxu();
        let five = bump.alloc([0u8; 5]).into_boxu();

        bump.with_config::<C_MIN_ALIGN_8, ()>(|bump| {
            drop(extra::box_replace_allocator(five, &bump));
            // deallocation succeeds and the bump pointer is unaligned
            bump.alloc(0u64).into_boxu();
        });
    }

    #[test]
    fn test_aligned_allocate() {
        let mut bump = Bump::<Global, C>::new();
        let bump = bump.as_mut_scope();

        let _three = bump.allocate(Layout::new::<[u8; 3]>()).unwrap();
        let five = bump.allocate(Layout::new::<[u8; 5]>()).unwrap();

        bump.with_config::<C_MIN_ALIGN_8, ()>(|bump| {
            unsafe { bump.deallocate(five.cast(), Layout::new::<[u8; 5]>()) };
            bump.alloc(0u64).into_boxu();
        });
    }

    #[test]
    fn test_scoped_alloc() {
        let mut bump = Bump::<Global, C>::new();
        let bump = bump.as_mut_scope();

        let _three = bump.alloc([0u8; 3]).into_boxu();
        let five = bump.alloc([0u8; 5]).into_boxu();

        bump.scoped::<()>(|bump| {
            drop(extra::box_replace_allocator(five, &*bump));
            // deallocation succeeds and the bump pointer is unaligned
            bump.alloc(0u64).into_boxu();
        });
    }

    #[test]
    fn test_scoped_allocate() {
        let mut bump = Bump::<Global, C>::new();
        let bump = bump.as_mut_scope();

        let _three = bump.allocate(Layout::new::<[u8; 3]>()).unwrap();
        let five = bump.allocate(Layout::new::<[u8; 5]>()).unwrap();

        bump.scoped::<()>(|bump| {
            unsafe { bump.deallocate(five.cast(), Layout::new::<[u8; 5]>()) };
            bump.alloc(0u64).into_boxu();
        });
    }

    #[test]
    fn test_scoped_aligned_alloc() {
        let mut bump = Bump::<Global, C>::new();
        let bump = bump.as_mut_scope();

        let _three = bump.alloc([0u8; 3]).into_boxu();
        let five = bump.alloc([0u8; 5]).into_boxu();

        bump.scoped_with_config::<C_MIN_ALIGN_8, ()>(|bump| {
            drop(extra::box_replace_allocator(five, &*bump));
            // deallocation succeeds and the bump pointer is unaligned
            bump.alloc(0u64).into_boxu();
        });
    }

    #[test]
    fn test_scoped_aligned_allocate() {
        let mut bump = Bump::<Global, C>::new();
        let bump = bump.as_mut_scope();

        let _three = bump.allocate(Layout::new::<[u8; 3]>()).unwrap();
        let five = bump.allocate(Layout::new::<[u8; 5]>()).unwrap();

        bump.scoped_with_config::<C_MIN_ALIGN_8, ()>(|bump| {
            unsafe { bump.deallocate(five.cast(), Layout::new::<[u8; 5]>()) };
            bump.alloc(0u64).into_boxu();
        });
    }
}
