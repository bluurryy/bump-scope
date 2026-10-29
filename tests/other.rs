#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{
    alloc::{AllocError, Allocator, Global, Layout, System},
    cell::Cell,
    ptr::NonNull,
    sync::{Arc, Mutex, PoisonError},
};

use bump_scope::{
    Bump, BumpScope, config,
    config::BumpConfig,
    stats::{AnyChunk, Chunk},
    traits::BumpAllocator,
};

use common::{MALLOC_OVERHEAD, up_and_down};

macro_rules! assert_chunk_sizes {
    ($bump:expr, $prev:expr, $curr:expr, $next:expr) => {
        let bump = &$bump;
        let curr = bump.stats().current_chunk().unwrap();
        debug_sizes(bump);
        assert_eq!(curr.size(), $curr - MALLOC_OVERHEAD, "wrong curr");
        assert!(
            curr.iter_prev()
                .map(Chunk::size)
                .eq($prev.into_iter().map(|s: usize| s - MALLOC_OVERHEAD)),
            "wrong prev"
        );
        assert!(
            curr.iter_next()
                .map(Chunk::size)
                .eq($next.into_iter().map(|s: usize| s - MALLOC_OVERHEAD)),
            "wrong next"
        );
    };
}

macro_rules! assert_eq_ident {
    ($ident:ident) => {
        assert_eq!(&*$ident, stringify!($ident));
    };
}

up_and_down! {
    const C: BumpConfig = DEFAULT_C;
    const MIN_ALIGN_8: BumpConfig = BumpConfigBuilder(C).min_align(8).build();
    const NO_MIN_SIZE: BumpConfig = BumpConfigBuilder(C).const_constructible(true).min_chunk_size(1).build();

    #[test]
    fn assert_send() {
        fn must_be_send<T: Send>(_: &T) {}
        let bump = Bump::<Global, C>::default();
        must_be_send(&bump);
    }

    #[test]
    fn alloc_iter() {
        let bump = Bump::<Global, C>::with_size(64);

        let slice_0 = bump.alloc_iter([1, 2, 3]).into_ref();
        let slice_1 = bump.alloc_iter([4, 5, 6]).into_ref();

        assert_eq!(slice_0, [1, 2, 3]);
        assert_eq!(slice_1, [4, 5, 6]);
    }

    #[test]
    fn reset_single_chunk() {
        let mut bump = Bump::<Global, NO_MIN_SIZE>::with_size(64);
        assert_chunk_sizes!(bump, [], 64, []);
        bump.reset();
        assert_chunk_sizes!(bump, [], 64, []);
    }

    #[test]
    fn reset_solo_chunk() {
        let mut bump = Bump::<Global, NO_MIN_SIZE>::with_size(64);

        assert_chunk_sizes!(bump, [], 64, []);
        bump.reset();
        assert_chunk_sizes!(bump, [], 64, []);
    }

    #[test]
    fn reset_first_chunk() {
        let mut bump = Bump::<Global, NO_MIN_SIZE>::with_size(64);

        bump.scoped(|scope| {
            force_alloc_new_chunk(scope);
            force_alloc_new_chunk(scope);
        });

        assert_chunk_sizes!(bump, [], 64, [128, 256]);
        bump.reset();
        assert_chunk_sizes!(bump, [], 256, []);
    }

    #[test]
    fn reset_middle_chunk() {
        let mut bump = Bump::<Global, NO_MIN_SIZE>::with_size(64);
        force_alloc_new_chunk(bump.as_scope());

        bump.scoped(|scope| {
            force_alloc_new_chunk(scope);
        });

        assert_chunk_sizes!(bump, [64], 128, [256]);
        bump.reset();
        assert_chunk_sizes!(bump, [], 256, []);
    }

    #[test]
    fn reset_last_chunk() {
        let mut bump = Bump::<Global, NO_MIN_SIZE>::with_size(64);
        force_alloc_new_chunk(bump.as_scope());
        force_alloc_new_chunk(bump.as_scope());
        assert_chunk_sizes!(bump, [128, 64], 256, []);
        bump.reset();
        assert_chunk_sizes!(bump, [], 256, []);
    }

    #[test]
    fn scope_by_guards() {
        let mut bump = Bump::<Global, C>::new();

        {
            let mut child_guard = bump.scope_guard();
            let child_scope = child_guard.scope();

            let child_0 = child_scope.alloc_str("child_0").into_boxu();
            let child_1 = child_scope.alloc_str("child_1").into_boxu();

            {
                let mut grand_child_guard = child_scope.scope_guard();
                let grand_child_scope = grand_child_guard.scope();

                let grand_child_0 = grand_child_scope.alloc_str("grand_child_0").into_boxu();
                let grand_child_1 = grand_child_scope.alloc_str("grand_child_1").into_boxu();

                dbg!(&grand_child_scope);

                assert_eq_ident!(grand_child_0);
                assert_eq_ident!(grand_child_1);
            }

            let child_2 = child_scope.alloc_str("child_2").into_boxu();
            let child_3 = child_scope.alloc_str("child_3").into_boxu();

            dbg!(&child_scope);

            assert_eq_ident!(child_0);
            assert_eq_ident!(child_1);
            assert_eq_ident!(child_2);
            assert_eq_ident!(child_3);
        }
    }

    #[test]
    fn scope_by_closures() {
        let mut bump = Bump::<Global, C>::new();

        bump.scoped(|child| {
            let child_0 = child.alloc_str("child_0").into_boxu();
            let child_1 = child.alloc_str("child_1").into_boxu();

            child.scoped(|grand_child| {
                let grand_child_0 = grand_child.alloc_str("grand_child_0").into_boxu();
                let grand_child_1 = grand_child.alloc_str("grand_child_1").into_boxu();

                dbg!(&grand_child);

                assert_eq_ident!(grand_child_0);
                assert_eq_ident!(grand_child_1);
            });

            let child_2 = child.alloc_str("child_2").into_boxu();
            let child_3 = child.alloc_str("child_3").into_boxu();

            dbg!(&child);

            assert_eq_ident!(child_0);
            assert_eq_ident!(child_1);
            assert_eq_ident!(child_2);
            assert_eq_ident!(child_3);
        });
    }

    #[test]
    fn reserve() {
        let bump = Bump::<Global, C>::default();
        dbg!(&bump);
        bump.reserve(256);
        assert!(bump.stats().remaining() > 256);
    }

    #[test]
    fn aligned() {
        let mut bump: Bump<Global, MIN_ALIGN_8> = Bump::new();

        bump.scoped(|bump| {
            bump.alloc(0xDEAD_BEEF_u64).into_boxu();
            assert_eq!(bump.stats().allocated(), 8);
            bump.with_config::<C, ()>(|bump| {
                bump.alloc(1u8).into_boxu();
                assert_eq!(bump.stats().allocated(), 9);
                bump.alloc(2u8).into_boxu();
                assert_eq!(bump.stats().allocated(), 10);
            });
            assert_eq!(bump.stats().allocated(), 16);
        });

        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn as_aligned() {
        let mut bump: Bump<Global, C> = Bump::new();

        {
            let bump = bump.mut_change_config::<MIN_ALIGN_8>();
            assert_eq!(bump.stats().allocated(), 0);
        }

        {
            bump.alloc(1u8).into_boxu();
            let bump = bump.mut_change_config::<MIN_ALIGN_8>();
            assert_eq!(bump.stats().allocated(), 8);
            bump.alloc(2u16).into_boxu();
            assert_eq!(bump.stats().allocated(), 16);
            bump.alloc(4u32).into_boxu();
            assert_eq!(bump.stats().allocated(), 24);
            bump.alloc(8u64).into_boxu();
            assert_eq!(bump.stats().allocated(), 32);
        }
    }

    /// Same as [`as_aligned`] but with a [`BumpScope`].
    #[test]
    fn as_aligned_scope() {
        let mut bump: Bump<Global, C> = Bump::new();

        bump.scoped(|bump| {
            let bump = bump.mut_change_config::<MIN_ALIGN_8>();
            assert_eq!(bump.stats().allocated(), 0);
        });

        bump.scoped(|bump| {
            bump.alloc(1u8).into_boxu();
            let bump = bump.mut_change_config::<MIN_ALIGN_8>();
            assert_eq!(bump.stats().allocated(), 8);
            bump.alloc(2u16).into_boxu();
            assert_eq!(bump.stats().allocated(), 16);
            bump.alloc(4u32).into_boxu();
            assert_eq!(bump.stats().allocated(), 24);
            bump.alloc(8u64).into_boxu();
            assert_eq!(bump.stats().allocated(), 32);
        });
    }

    #[test]
    fn zero_capacity() {
        let bump = Bump::<Global, C>::with_capacity(Layout::new::<[u8; 0]>());
        dbg!(bump);
    }

    #[test]
    fn alloc_iter2() {
        let bump = Bump::<Global, C>::new();
        let one = 1;
        let two = 2;
        let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two)).into_boxu();

        assert_eq!(&*string, "1 + 2 = 3");
    }

    #[test]
    fn vec_of_strings() {
        let bump = Bump::<Global, C>::new();
        let mut vec = Vec::new_in(&bump);

        for i in 0..10 {
            let string = bump.alloc_fmt(format_args!("hello {i}")).into_ref();
            vec.push(string);
        }

        let slice: &[&str] = extra::vec_into_slice(vec);

        dbg!(&slice);
    }

    #[test]
    fn realign() {
        type AlignT = u64;

        // change_config
        {
            let bump = Bump::<Global, C>::with_size(64);
            bump.alloc(0u8).into_boxu();
            assert!(!bump.stats().bump_position().unwrap().cast::<AlignT>().is_aligned());
            let bump = bump.change_config::<MIN_ALIGN_8>();
            assert!(bump.stats().bump_position().unwrap().cast::<AlignT>().is_aligned());
        }

        // mut_change_config
        {
            let mut bump = Bump::<Global, C>::with_size(64);
            bump.alloc(0u8).into_boxu();
            assert!(!bump.stats().bump_position().unwrap().cast::<AlignT>().is_aligned());
            let bump = bump.mut_change_config::<MIN_ALIGN_8>();
            assert!(bump.stats().bump_position().unwrap().cast::<AlignT>().is_aligned());
        }

        // with_config
        {
            let mut bump = Bump::<Global, C>::with_size(64);
            bump.alloc(0u8).into_boxu();
            assert!(!bump.stats().bump_position().unwrap().cast::<AlignT>().is_aligned());
            bump.with_config::<MIN_ALIGN_8, ()>(|bump| {
                assert!(bump.stats().bump_position().unwrap().cast::<AlignT>().is_aligned());
            });
        }

        // scoped_with_config
        {
            let mut bump = Bump::<Global, C>::with_size(64);
            bump.alloc(0u8).into_boxu();
            assert!(!bump.stats().bump_position().unwrap().cast::<AlignT>().is_aligned());
            bump.scoped_with_config::<MIN_ALIGN_8, ()>(|bump| {
                assert!(bump.stats().bump_position().unwrap().cast::<AlignT>().is_aligned());
            });
        }
    }

    // https://github.com/bluurryy/bump-scope/issues/6
    #[test]
    #[expect(clippy::too_many_lines)]
    fn alloc_zst() {
        thread_local! {
            static DROPS: Cell<usize> = const { Cell::new(0) };
            static CLONES: Cell<usize> = const { Cell::new(0) };
            static DEFAULTS: Cell<usize> = const { Cell::new(0) };
        }

        struct DropEmit;

        impl Default for DropEmit {
            fn default() -> Self {
                DEFAULTS.set(DEFAULTS.get() + 1);
                Self
            }
        }

        impl Clone for DropEmit {
            fn clone(&self) -> Self {
                CLONES.set(CLONES.get() + 1);
                Self
            }
        }

        impl Drop for DropEmit {
            fn drop(&mut self) {
                DROPS.set(DROPS.get() + 1);
            }
        }

        let mut bump = Bump::<Global, C>::new();

        fn reset() {
            DROPS.set(0);
            CLONES.set(0);
            DEFAULTS.set(0);
        }

        {
            reset();
            let drop_emit = bump.alloc(DropEmit);
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 1);
        }

        {
            reset();
            let drop_emit = bump.alloc_with(|| DropEmit);
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 1);
        }

        {
            reset();
            let drop_emit = bump.alloc_default::<DropEmit>();
            assert_eq!(DEFAULTS.get(), 1);
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 1);
        }

        {
            reset();
            let drop_emit = bump.alloc_try_with::<_, ()>(|| Ok(DropEmit));
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 1);
        }

        {
            reset();
            let drop_emit = [DropEmit, DropEmit, DropEmit];
            let drop_emit_clone = bump.alloc_slice_clone(&drop_emit);
            assert_eq!(CLONES.get(), 3);
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 3);
            drop(drop_emit_clone);
            assert_eq!(DROPS.get(), 6);
        }

        {
            reset();
            let drop_emit = bump.alloc_slice_fill(3, DropEmit);
            assert_eq!(CLONES.get(), 2);
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 3);
        }

        {
            reset();
            let drop_emit = bump.alloc_slice_fill(0, DropEmit);
            assert_eq!(DROPS.get(), 1);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 1);
        }

        {
            reset();
            let drop_emit = bump.alloc_slice_fill_with(3, || DropEmit);
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 3);
        }

        {
            reset();

            let result = std::panic::catch_unwind(|| {
                let mut i = 0;
                bump.alloc_slice_fill_with(5, || {
                    if i == 3 {
                        panic!("AAAAAAA");
                    }

                    i += 1;
                    DropEmit
                }).into_boxu();
            });

            assert_eq!(*result.unwrap_err().downcast::<&'static str>().unwrap(), "AAAAAAA");
            assert_eq!(DROPS.get(), 3);
        }

        {
            reset();
            let drop_emit = bump.alloc_slice_fill_with(0, || DropEmit);
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 0);
        }

        {
            reset();
            let drop_emit = bump.alloc_iter([DropEmit, DropEmit, DropEmit]);
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 3);
        }

        {
            reset();
            let drop_emit = bump.alloc_iter_mut([DropEmit, DropEmit, DropEmit]);
            assert_eq!(DROPS.get(), 0);
            drop(drop_emit);
            assert_eq!(DROPS.get(), 3);
        }
    }

    #[test]
    fn call_zst_creation_closures() {
        let bump = Bump::<Global, C>::new();

        {
            let mut calls = 0;
            bump.alloc_with(|| {
                calls += 1;
            }).into_boxu();
            assert_eq!(calls, 1);
        }

        {
            let mut calls = 0;
            bump.alloc_slice_fill_with(3, || {
                calls += 1;
            }).into_boxu();
            assert_eq!(calls, 3);
        }
    }

    #[test]
    fn dealloc_ltr() {
        let bump = Bump::<Global, C>::new();
        let slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]).into_boxu();
        let (lhs, rhs) = extra::split_at(slice, 3);

        assert_eq!(&*lhs, [1, 2, 3]);
        assert_eq!(&*rhs, [4, 5, 6]);
        assert_eq!(bump.stats().allocated(), 6 * 4);

        drop(extra::replace_allocator(lhs, &bump));

        if config::read::upwards(C) {
            assert_eq!(bump.stats().allocated(), 6 * 4);
        } else {
            assert_eq!(bump.stats().allocated(), 3 * 4);
        }

        drop(extra::replace_allocator(rhs, &bump));

        if config::read::upwards(C) {
            assert_eq!(bump.stats().allocated(), 3 * 4);
        } else {
            assert_eq!(bump.stats().allocated(), 0);
        }
    }

    #[test]
    fn dealloc_rtl() {
        let bump = Bump::<Global, C>::new();
        let slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]).into_boxu();
        let (lhs, rhs) = extra::split_at(slice, 3);

        assert_eq!(&*lhs, [1, 2, 3]);
        assert_eq!(&*rhs, [4, 5, 6]);
        assert_eq!(bump.stats().allocated(), 6 * 4);

        drop(extra::replace_allocator(rhs, &bump));

        if config::read::upwards(C) {
            assert_eq!(bump.stats().allocated(), 3 * 4);
        } else {
            assert_eq!(bump.stats().allocated(), 6 * 4);
        }

        drop(extra::replace_allocator(lhs, &bump));

        if config::read::upwards(C) {
            assert_eq!(bump.stats().allocated(), 0);
        } else {
            assert_eq!(bump.stats().allocated(), 3 * 4);
        }
    }

    #[test]
    fn chunk_size_0() {
        let chunk_header = size_of::<[usize; 4]>();
        let malloc_overhead = size_of::<[usize; 2]>();

        assert_eq!(
            Bump::<Global, NO_MIN_SIZE>::with_size(0).stats().size(),
            (chunk_header + malloc_overhead).next_power_of_two() - malloc_overhead
        );
    }

    #[test]
    fn chunk_size_512() {
        assert_eq!(
            Bump::<Global, NO_MIN_SIZE>::with_size(512).stats().size(),
            512 - size_of::<[usize; 2]>()
        );
    }

    #[test]
    fn test_drop_allocator() {
        #[derive(Clone, Default)]
        struct DropCounterMutex {
            count: Arc<Mutex<u32>>,
        }

        impl DropCounterMutex {
            fn get(&self) -> u32 {
                *self.count.lock().unwrap_or_else(PoisonError::into_inner)
            }
        }

        impl Drop for DropCounterMutex {
            fn drop(&mut self) {
                *self.count.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            }
        }

        #[expect(dead_code)]
        #[derive(Clone)]
        struct ReferenceCountedAllocator(DropCounterMutex);

        unsafe impl Allocator for ReferenceCountedAllocator {
            fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
                System.allocate(layout)
            }

            unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
                // Safety: Invariants passed to caller.
                unsafe { System.deallocate(ptr, layout) }
            }
        }

        let drop_count = DropCounterMutex::default();

        let allocator = ReferenceCountedAllocator(drop_count.clone());
        let bump: Bump<_> = Bump::with_size_in(512, allocator.clone());
        drop(bump);
        assert_eq!(drop_count.get(), 1);

        let bump: Bump<_> = Bump::with_size_in(512, allocator);
        bump.reserve(1024); // allocate another chunk
        drop(bump);
        assert_eq!(drop_count.get(), 2);
    }

    #[test]
    fn generic_bump_scope() {
        fn foo<A: Allocator, const C: BumpConfig>(bump: &mut BumpScope<A, C>) {
            assert_eq!(bump.stats().allocated(), 0);
            bump.alloc_str("good").into_boxu();
            assert_eq!(bump.stats().allocated(), 4);
            bump.scoped(|bump| {
                bump.alloc_str("day").into_boxu();
                assert_eq!(bump.stats().allocated(), 7);
                bump.scoped(|bump| {
                    bump.alloc_str("world").into_boxu();
                    assert_eq!(bump.stats().allocated(), 12);
                });
                assert_eq!(bump.stats().allocated(), 7);
            });
            assert_eq!(bump.stats().allocated(), 4);
        }

        let mut bump: Bump = Bump::new();
        foo(bump.as_mut_scope());

        let mut bump: Bump = Bump::new();
        bump.scoped(|bump| {
            foo(bump);
        });
    }

    #[test]
    fn default() {
        let bump_ref = Bump::<Global, C>::new();
        let bump = Bump::<Global, C>::default();
        assert_eq!(bump.stats().count(), 1);
        assert_eq!(bump.stats().count(), bump_ref.stats().count());
        assert_eq!(bump.stats().size(), bump_ref.stats().size());
        drop(bump);
    }
}

fn debug_sizes(bump: &impl BumpAllocator) {
    let iter = bump.any_stats().small_to_big();
    let vec = iter.map(AnyChunk::size).collect::<Vec<_>>();
    eprintln!("sizes: {vec:?}");
}

// test claim in crate docs that layout equals `Cell<NonNull<()>>`
// make sure to also check that niches are the same
mod doc_layout_claim {
    trait SizedTypeProperties: Sized {
        const SIZE: usize = mem::size_of::<Self>();
        const ALIGN: usize = mem::align_of::<Self>();
    }

    impl<T> SizedTypeProperties for T {}

    use core::{cell::Cell, ptr::NonNull};
    use std::{alloc::Global, mem};
    type Bump = bump_scope::Bump<Global>;
    type BumpScope = bump_scope::BumpScope<'static, Global>;
    type Comparand = Cell<NonNull<()>>;
    const _: () = assert!(Bump::SIZE == Comparand::SIZE && Bump::ALIGN == Comparand::ALIGN);
    const _: () = assert!(BumpScope::SIZE == Comparand::SIZE && BumpScope::ALIGN == Comparand::ALIGN);
    const _: () =
        assert!(Option::<Bump>::SIZE == Option::<Comparand>::SIZE && Option::<Bump>::ALIGN == Option::<Comparand>::ALIGN);
    const _: () = assert!(
        Option::<BumpScope>::SIZE == Option::<Comparand>::SIZE && Option::<BumpScope>::ALIGN == Option::<Comparand>::ALIGN
    );
}

fn force_alloc_new_chunk<A: Allocator, const C: BumpConfig>(bump: &BumpScope<A, C>) {
    let size = bump.stats().current_chunk().unwrap().remaining() + 1;
    let layout = Layout::from_size_align(size, 1).unwrap();
    bump.allocate(layout).unwrap();
}
