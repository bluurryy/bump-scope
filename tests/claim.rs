#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{alloc::Global, mem::forget};

use bump_scope::Bump;

use crate::common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn scoped() {
        let bump = Bump::<Global, C>::new();

        let vec1: Vec<u8, _> = vec![in &bump; 1, 2, 3];
        let vec2: Vec<u8, _> = vec![in &bump; 4, 5, 6];

        assert_eq!(bump.stats().allocated(), 6);

        bump.claim().scoped(|bump| {
            let mut both = Vec::new_in(&bump);
            both.extend(vec1.iter().copied());
            both.extend(vec2.iter().copied());
            assert_eq!(both, [1, 2, 3, 4, 5, 6]);
            assert_eq!(bump.stats().allocated(), 6 + both.capacity());
        });

        assert_eq!(bump.stats().allocated(), 6);
    }

    #[test]
    fn scope_guard() {
        let bump = Bump::<Global, C>::new();

        let vec1: Vec<u8, _> = vec![in &bump; 1, 2, 3];
        let vec2: Vec<u8, _> = vec![in &bump; 4, 5, 6];

        assert_eq!(bump.stats().allocated(), 6);

        {
            let mut guard = bump.claim();
            let mut guard = guard.scope_guard();
            let bump = guard.scope();

            let mut both = Vec::new_in(&bump);
            both.extend(vec1.iter().copied());
            both.extend(vec2.iter().copied());
            assert_eq!(both, [1, 2, 3, 4, 5, 6]);
            assert_eq!(bump.stats().allocated(), 6 + both.capacity());
        }

        assert_eq!(bump.stats().allocated(), 6);
    }

    #[test]
    fn alloc_mut() {
        let bump = Bump::<Global, C>::new();

        let uno_dos_tres = bump.claim().alloc_iter_mut([1, 2, 3]).into_ref();
        assert_eq!(uno_dos_tres, [1, 2, 3]);
    }

    #[test]
    fn enter_scope_on_claimed() {
        fn inner() {
            let mut bump = Bump::<Global, C>::new();
            let mut guard = bump.scope_guard();
            let mut scope = guard.scope().by_value();
            forget(scope.claim());

            {
                let _guard = scope.scope_guard();
                // this will call `reset_to` with a checkpoint from the claimed dummy chunk
                // this mustn't try to actually reset within the claimed chunk as that would trigger a data race here
            }
        }

        std::thread::spawn(inner);
        std::thread::spawn(inner);
    }

    #[test]
    fn exit_scope_on_claimed() {
        fn inner() {
            let mut bump = Bump::<Global, C>::new();

            {
                let mut guard = bump.scope_guard();
                let scope = guard.scope();
                forget(scope.claim());
                // this will call `reset_to` on a claimed bump allocator
                // that should be fine, the bump allocator is thereby reclaimed
            }

            assert!(!bump.is_claimed());
        }

        std::thread::spawn(inner);
        std::thread::spawn(inner);
    }
}
