#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::alloc::{Allocator, Global, Layout};

use bump_scope::{Bump, config::BumpConfig, traits::BumpAllocator};

use common::{AssumedMallocOverhead, up_and_down};

up_and_down! {
    const C: BumpConfig = BumpConfigBuilder(DEFAULT_C).const_constructible(true).build();

    #[test]
    fn allocated() {
        let bump = Bump::<Global, C>::new();
        assert_eq!(bump.stats().count(), 1);
        drop(bump);
    }

    #[test]
    fn unallocated() {
        let bump = Bump::<Global, C>::const_new();
        assert_eq!(bump.stats().count(), 0);
        drop(bump);
    }

    #[test]
    fn default() {
        let bump = Bump::<Global, C>::default();
        assert_eq!(bump.stats().count(), 0);
        drop(bump);
    }

    #[test]
    fn unallocated_alloc() {
        let bump = Bump::<Global, C>::const_new();
        assert_eq!(bump.stats().count(), 0);
        bump.alloc_str("Hello, World!").into_boxu();
        assert_eq!(bump.stats().count(), 1);
        drop(bump);
    }

    #[test]
    fn allocated_reserve_bytes() {
        let bump = Bump::<Global, C>::new();
        assert_eq!(bump.stats().count(), 1);
        bump.reserve(1024);
        assert_eq!(bump.stats().count(), 2);
        assert!(bump.stats().capacity() >= 1024);
        drop(bump);
    }

    #[test]
    fn unallocated_reserve_bytes() {
        let bump = Bump::<Global, C>::const_new();
        assert_eq!(bump.stats().count(), 0);
        bump.reserve(1024);
        assert_eq!(bump.stats().count(), 1);
        assert!(bump.stats().capacity() >= 1024);
        drop(bump);
    }

    #[test]
    fn checkpoint() {
        let bump = Bump::<Global, C>::const_new();

        let checkpoint_unallocated = bump.checkpoint();
        assert_eq!(bump.stats().count(), 0);
        assert_eq!(bump.stats().allocated(), 0);

        bump.alloc_str("hello").into_boxu();
        let checkpoint_hello = bump.checkpoint();
        assert_eq!(bump.stats().count(), 1);
        assert_eq!(bump.stats().allocated(), 5);

        bump.alloc_str("world").into_boxu();
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
        let bump = Bump::<Global, C>::const_new();

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
    }

    #[test]
    fn reset() {
        let mut bump = Bump::<Global, C>::const_new();
        bump.reset();
    }

    // if allocating on the dummy chunk would bump it's position field, even by 0
    // then this cause a data race
    #[test]
    fn potential_data_race() {
        fn create_bump_and_alloc() {
            let bump = Bump::<Global, C>::new();
            bump.allocate(Layout::new::<()>()).unwrap();
        }

        std::thread::spawn(create_bump_and_alloc);
        std::thread::spawn(create_bump_and_alloc);
    }

    #[test]
    fn default_chunk_size() {
        let bump = Bump::<Global, C>::const_new();
        assert_eq!(bump.stats().count(), 0);
        bump.alloc("a").into_boxu();
        assert_eq!(bump.stats().count(), 1);
        assert_eq!(
            bump.stats().current_chunk().unwrap().size(),
            512 - size_of::<AssumedMallocOverhead>()
        );
    }

    #[test]
    fn custom_chunk_size() {
        const MIN_SIZE_4096: BumpConfig = BumpConfigBuilder(C).min_chunk_size(4096).build();
        let bump = Bump::<Global, MIN_SIZE_4096>::const_new();
        assert_eq!(bump.stats().count(), 0);
        bump.alloc("a").into_boxu();
        assert_eq!(bump.stats().count(), 1);
        assert_eq!(
            bump.stats().current_chunk().unwrap().size(),
            4096 - size_of::<AssumedMallocOverhead>()
        );
    }
}

fn allocate_another_chunk<const C: BumpConfig>(bump: &Bump<Global, C>, dummy_size: usize) {
    let start_chunks = bump.any_stats().count();

    // fill the existing chunk if there is one
    let remaining = bump.any_stats().remaining();
    if remaining != 0 {
        bump.allocate(Layout::from_size_align(remaining, 1).unwrap()).unwrap();
    }
    assert_eq!(bump.any_stats().count(), start_chunks);

    bump.allocate(Layout::from_size_align(dummy_size, 1).unwrap()).unwrap();
    assert_eq!(bump.any_stats().count(), start_chunks + 1);
    assert_eq!(bump.any_stats().current_chunk().unwrap().allocated(), dummy_size);
}
