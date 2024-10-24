#![allow(unused_imports, clippy::incompatible_msrv)]
#![cfg(feature = "std")]

use core::{
    alloc::Layout,
    cell::Cell,
    fmt::Debug,
    mem,
    ops::Index,
    sync::atomic::{AtomicUsize, Ordering},
};
use std::io::IoSlice;

mod alloc_fmt;
mod alloc_iter;
mod alloc_slice;
mod alloc_try_with;
mod allocator_api;
mod bump_vec;
mod bump_vec_doc;
mod chunk_size;
#[cfg(feature = "nightly-coerce-unsized")]
mod coerce_unsized;
mod from_std;
mod mut_bump_vec_doc;
mod mut_bump_vec_rev_doc;
mod mut_collections_do_not_waste_space;
mod panic_safety;
mod pool;
#[cfg(feature = "serde")]
mod serde;
mod unaligned_collection;
mod unallocated;
mod vec;
mod bump_string;

extern crate std;

type Result<T = (), E = AllocError> = core::result::Result<T, E>;

const MALLOC_OVERHEAD: usize = size_of::<AssumedMallocOverhead>();
const OVERHEAD: usize = MALLOC_OVERHEAD + size_of::<ChunkHeader<Global>>();

use crate::{
    chunk_size::AssumedMallocOverhead, infallible, mut_bump_format, mut_bump_vec, mut_bump_vec_rev, owned_slice, Bump,
    BumpBox, BumpScope, BumpVec, Chunk, ChunkHeader, ChunkSize, FmtFn, MinimumAlignment, MutBumpString, MutBumpVec,
    MutBumpVecRev, SupportedMinimumAlignment,
};

#[allow(dead_code)]
fn assert_covariant() {
    fn bump_box<'a, 'other>(x: BumpBox<'static, &'static str>) -> BumpBox<'a, &'other str> {
        x
    }

    fn bump_slice_iter<'a, 'other>(
        x: owned_slice::IntoIter<'static, &'static str>,
    ) -> owned_slice::IntoIter<'a, &'other str> {
        x
    }

    // fn mut_bump_vec<'b, 'a, 'other>(x: MutBumpVec<'b, 'static, &'static str>) -> MutBumpVec<'b, 'a, &'other str> {
    //     x
    // }

    // fn mut_bump_string<'b, 'a>(x: MutBumpString<'b, 'static>) -> MutBumpString<'b, 'a> {
    //     x
    // }
}

macro_rules! either_way {
    ($($(#[$attr:meta])* $ident:ident)*) => {
        mod up {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    eprintln!("`UP` is `true`");
                    super::$ident::<true>();
                }
            )*
        }

        mod down {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    eprintln!("`UP` is `false`");
                    super::$ident::<false>();
                }
            )*
        }
    };
}

use allocator_api2::alloc::{AllocError, Allocator, Global};
pub(crate) use either_way;

either_way! {
    assert_send

    reset_single_chunk

    mut_bump_vec

    mut_bump_vec_push_pop

    mut_bump_vec_insert

    mut_bump_vec_remove

    mut_bump_vec_swap_remove

    mut_bump_vec_extend

    mut_bump_vec_drop

    mut_bump_vec_write

    bump_vec_shrink_can

    bump_vec_shrink_cant

    alloc_iter

    macro_syntax

    reset_first_chunk

    reset_middle_chunk

    reset_last_chunk

    scope_by_guards

    scope_by_closures

    reserve

    aligned

    as_aligned

    realign

    alloc_zst

    call_zst_creation_closures

    deallocate_in_ltr

    deallocate_in_rtl
}

macro_rules! assert_chunk_sizes {
    ($bump:expr, $prev:expr, $curr:expr, $next:expr) => {
        let bump = &$bump;
        let curr = bump.guaranteed_allocated_stats().current;
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

#[allow(dead_code)]
fn assert_send<const UP: bool>() {
    fn must_be_send<T: Send>(_: &T) {}
    let bump = Bump::<Global, 1, UP>::default();
    must_be_send(&bump);
}

fn mut_bump_vec<const UP: bool>() {
    const TIMES: usize = 5;

    for (size, count) in [(0, 2), (512, 1)] {
        let mut bump = Bump::<Global, 1, UP>::with_size(size);
        let mut vec = MutBumpVec::new_in(&mut bump);
        assert_eq!(vec.guaranteed_allocated_stats().count(), 1);
        vec.extend(core::iter::repeat(3).take(TIMES));
        assert_eq!(vec.guaranteed_allocated_stats().count(), count);
        let _ = vec.into_boxed_slice();
        dbg!(bump.guaranteed_allocated_stats());
        assert_eq!(
            bump.guaranteed_allocated_stats().current.allocated(),
            TIMES * core::mem::size_of::<i32>()
        );
        bump.reset();
        assert_eq!(bump.guaranteed_allocated_stats().allocated(), 0);
    }
}

fn mut_bump_vec_push_pop<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = mut_bump_vec![in bump; 1, 2];

    vec.push(3);

    assert_eq!(vec, [1, 2, 3]);
    assert_eq!(vec.pop(), Some(3));
    assert_eq!(vec.pop(), Some(2));
    assert_eq!(vec.pop(), Some(1));
    assert_eq!(vec.pop(), None);

    vec.push(4);
    assert_eq!(vec, [4]);
}

fn mut_bump_vec_insert<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = mut_bump_vec![in bump; 1, 2, 3];

    vec.insert(1, 4);
    assert_eq!(vec, [1, 4, 2, 3]);

    vec.insert(4, 5);
    assert_eq!(vec, [1, 4, 2, 3, 5]);

    vec.insert(0, 6);
    assert_eq!(vec, [6, 1, 4, 2, 3, 5]);
}

fn mut_bump_vec_remove<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = mut_bump_vec![in bump; 1, 2, 3, 4, 5];

    assert_eq!(vec.remove(1), 2);
    assert_eq!(vec, [1, 3, 4, 5]);

    assert_eq!(vec.remove(3), 5);
    assert_eq!(vec, [1, 3, 4]);

    assert_eq!(vec.remove(0), 1);
    assert_eq!(vec, [3, 4]);
}

fn mut_bump_vec_swap_remove<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = mut_bump_vec![in bump; 1, 2, 3, 4, 5];

    assert_eq!(vec.swap_remove(1), 2);
    assert_eq!(vec, [1, 5, 3, 4]);

    assert_eq!(vec.swap_remove(3), 4);
    assert_eq!(vec, [1, 5, 3]);

    assert_eq!(vec.swap_remove(0), 1);
    assert_eq!(vec, [3, 5]);
}

fn mut_bump_vec_extend<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = mut_bump_vec![in bump];

    vec.extend([1, 2, 3]);
    assert_eq!(vec, [1, 2, 3]);

    vec.clear();
    assert!(vec.is_empty());

    vec.extend_from_array([1, 2, 3]);
    assert_eq!(vec, [1, 2, 3]);

    vec.extend_from_slice_copy(&[4, 5, 6]);
    assert_eq!(vec, [1, 2, 3, 4, 5, 6]);

    vec.extend_from_slice_clone(&[7, 8, 9]);
    assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

fn mut_bump_vec_drop<const UP: bool>() {
    const SIZE: usize = 32;
    assert_eq!(mem::size_of::<ChunkHeader<Global>>(), SIZE);

    let mut bump = Bump::<Global, 1, UP>::with_size(64);
    assert_eq!(bump.guaranteed_allocated_stats().current.size(), 64 - MALLOC_OVERHEAD);
    assert_eq!(bump.guaranteed_allocated_stats().capacity(), 64 - OVERHEAD);
    assert_eq!(bump.guaranteed_allocated_stats().remaining(), 64 - OVERHEAD);

    let mut vec: MutBumpVec<u8, Global, 1, UP> = mut_bump_vec![in bump];
    vec.reserve(33);

    assert_eq!(vec.guaranteed_allocated_stats().current.size(), 128 - MALLOC_OVERHEAD);
    assert_eq!(
        vec.guaranteed_allocated_stats().current.prev().unwrap().size(),
        64 - MALLOC_OVERHEAD
    );
    assert_eq!(vec.guaranteed_allocated_stats().size(), 64 + 128 - MALLOC_OVERHEAD * 2);
    assert_eq!(vec.guaranteed_allocated_stats().count(), 2);

    drop(vec);

    assert_eq!(bump.guaranteed_allocated_stats().current.size(), 128 - MALLOC_OVERHEAD);
    assert_eq!(bump.guaranteed_allocated_stats().size(), 64 + 128 - MALLOC_OVERHEAD * 2);
    assert_eq!(bump.guaranteed_allocated_stats().count(), 2);
}

fn mut_bump_vec_write<const UP: bool>() {
    use std::io::Write;

    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec: MutBumpVec<u8, Global, 1, UP> = mut_bump_vec![in bump];

    let _ = vec.write(&[0]).unwrap();

    let _ = vec
        .write_vectored(&[IoSlice::new(&[1, 2, 3]), IoSlice::new(&[4, 5, 6, 7, 8])])
        .unwrap();

    let _ = vec.write(&[9, 10]).unwrap();

    assert_eq!(vec, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

fn alloc_iter<const UP: bool>() {
    let bump = Bump::<Global, 1, UP>::with_size(64);

    let slice_0 = bump.alloc_iter([1, 2, 3]);
    let slice_1 = bump.alloc_iter([4, 5, 6]);

    assert_eq!(slice_0.as_ref(), [1, 2, 3]);
    assert_eq!(slice_1.as_ref(), [4, 5, 6]);
}

fn reset_single_chunk<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::with_size(64);
    assert_chunk_sizes!(bump, [], 64, []);
    bump.reset();
    assert_chunk_sizes!(bump, [], 64, []);
}

fn macro_syntax<const UP: bool>() {
    #[allow(clippy::needless_pass_by_value)]
    fn check<T: Debug + PartialEq, const UP: bool>(v: MutBumpVec<T, Global, 1, UP>, expected: &[T]) {
        dbg!(&v);
        assert_eq!(v, expected);
    }

    let mut bump = Bump::<Global, 1, UP>::new();

    check::<i32, UP>(mut_bump_vec![in bump], &[]);
    check(mut_bump_vec![in bump; 1, 2, 3], &[1, 2, 3]);
    check(mut_bump_vec![in bump; 5; 3], &[5, 5, 5]);

    check::<i32, UP>(mut_bump_vec![in &mut bump], &[]);
    check(mut_bump_vec![in &mut bump; 1, 2, 3], &[1, 2, 3]);
    check(mut_bump_vec![in &mut bump; 5; 3], &[5, 5, 5]);
}

fn debug_sizes<const UP: bool>(bump: &Bump<Global, 1, UP>) {
    let iter = bump.guaranteed_allocated_stats().small_to_big();
    let vec = iter.map(Chunk::size).collect::<Vec<_>>();
    let sizes = FmtFn(|f| write!(f, "{vec:?}"));
    dbg!(sizes);
}

fn force_alloc_new_chunk<const UP: bool>(bump: &BumpScope<Global, 1, UP>) {
    let size = bump.guaranteed_allocated_stats().current.remaining() + 1;
    let layout = Layout::from_size_align(size, 1).unwrap();
    bump.alloc_layout(layout);
}

fn reset_first_chunk<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::with_size(64);

    bump.scoped(|scope| {
        force_alloc_new_chunk(&scope);
        force_alloc_new_chunk(&scope);
    });

    assert_chunk_sizes!(bump, [], 64, [128, 256]);
    bump.reset();
    assert_chunk_sizes!(bump, [], 256, []);
}

fn reset_middle_chunk<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::with_size(64);
    force_alloc_new_chunk(bump.as_scope());

    bump.scoped(|scope| {
        force_alloc_new_chunk(&scope);
    });

    assert_chunk_sizes!(bump, [64], 128, [256]);
    bump.reset();
    assert_chunk_sizes!(bump, [], 256, []);
}

fn reset_last_chunk<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::with_size(64);
    force_alloc_new_chunk(bump.as_scope());
    force_alloc_new_chunk(bump.as_scope());
    assert_chunk_sizes!(bump, [128, 64], 256, []);
    bump.reset();
    assert_chunk_sizes!(bump, [], 256, []);
}

macro_rules! assert_eq_ident {
    ($ident:ident) => {
        assert_eq!($ident.as_ref(), stringify!($ident));
    };
}

fn scope_by_guards<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();

    {
        let mut child_guard = bump.scope_guard();
        let mut child_scope = child_guard.scope();

        let child_0 = child_scope.alloc_str("child_0");
        let child_1 = child_scope.alloc_str("child_1");

        {
            let mut grand_child_guard = child_scope.scope_guard();
            let grand_child_scope = grand_child_guard.scope();

            let grand_child_0 = grand_child_scope.alloc_str("grand_child_0");
            let grand_child_1 = grand_child_scope.alloc_str("grand_child_1");

            dbg!(&grand_child_scope);

            assert_eq_ident!(grand_child_0);
            assert_eq_ident!(grand_child_1);
        }

        let child_2 = child_scope.alloc_str("child_2");
        let child_3 = child_scope.alloc_str("child_3");

        dbg!(&child_scope);

        assert_eq_ident!(child_0);
        assert_eq_ident!(child_1);
        assert_eq_ident!(child_2);
        assert_eq_ident!(child_3);
    }
}

fn scope_by_closures<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();

    bump.scoped(|mut child| {
        let child_0 = child.alloc_str("child_0");
        let child_1 = child.alloc_str("child_1");

        child.scoped(|grand_child| {
            let grand_child_0 = grand_child.alloc_str("grand_child_0");
            let grand_child_1 = grand_child.alloc_str("grand_child_1");

            dbg!(&grand_child);

            assert_eq_ident!(grand_child_0);
            assert_eq_ident!(grand_child_1);
        });

        let child_2 = child.alloc_str("child_2");
        let child_3 = child.alloc_str("child_3");

        dbg!(&child);

        assert_eq_ident!(child_0);
        assert_eq_ident!(child_1);
        assert_eq_ident!(child_2);
        assert_eq_ident!(child_3);
    });
}

fn reserve<const UP: bool>() {
    let bump = Bump::<Global, 1, UP>::default();
    dbg!(&bump);
    bump.reserve_bytes(256);
    assert!(bump.stats().remaining() > 256);
}

fn aligned<const UP: bool>() {
    let mut bump: Bump<Global, 8> = Bump::new();

    bump.scoped(|mut bump| {
        bump.alloc(0xDEAD_BEEF_u64);
        assert_eq!(bump.stats().allocated(), 8);
        bump.aligned::<1, ()>(|bump| {
            bump.alloc(1u8);
            assert_eq!(bump.stats().allocated(), 9);
            bump.alloc(2u8);
            assert_eq!(bump.stats().allocated(), 10);
        });
        assert_eq!(bump.stats().allocated(), 16);
    });

    assert_eq!(bump.stats().allocated(), 0);
}

fn as_aligned<const UP: bool>() {
    let mut bump: Bump = Bump::new();

    {
        let bump = bump.as_aligned_mut::<8>();
        assert_eq!(bump.stats().allocated(), 0);
    }

    {
        bump.alloc(1u8);
        let bump = bump.as_aligned_mut::<8>();
        assert_eq!(bump.stats().allocated(), 8);
        bump.alloc(2u16);
        assert_eq!(bump.stats().allocated(), 16);
        bump.alloc(4u32);
        assert_eq!(bump.stats().allocated(), 24);
        bump.alloc(8u64);
        assert_eq!(bump.stats().allocated(), 32);
    }
}

#[allow(dead_code)]
fn api_that_accepts_bump_or_bump_scope() {
    fn vec_from_mut_bump(bump: &mut Bump) -> MutBumpVec<i32> {
        MutBumpVec::new_in(bump)
    }

    fn vec_from_mut_bump_scope<'b, 'a>(bump: &'b mut BumpScope<'a>) -> MutBumpVec<'b, 'a, i32> {
        MutBumpVec::new_in(bump)
    }

    fn string_from_mut_bump(bump: &mut Bump) -> MutBumpString {
        MutBumpString::new_in(bump)
    }

    fn string_from_mut_bump_scope<'b, 'a>(bump: &'b mut BumpScope<'a>) -> MutBumpString<'b, 'a> {
        MutBumpString::new_in(bump)
    }
}

#[allow(dead_code)]
fn bump_vec_macro() {
    fn new_in(bump: &mut Bump) -> MutBumpVec<u32> {
        mut_bump_vec![in bump]
    }

    fn from_array_in(bump: &mut Bump) -> MutBumpVec<u32> {
        mut_bump_vec![in bump; 1, 2, 3]
    }

    fn from_elem_in(bump: &mut Bump) -> MutBumpVec<u32> {
        mut_bump_vec![in bump; 3; 5]
    }

    fn try_new_in(bump: &mut Bump) -> Result<MutBumpVec<u32>> {
        mut_bump_vec![try in bump]
    }

    fn try_from_array_in(bump: &mut Bump) -> Result<MutBumpVec<u32>> {
        mut_bump_vec![try in bump; 1, 2, 3]
    }

    fn try_from_elem_in(bump: &mut Bump) -> Result<MutBumpVec<u32>> {
        mut_bump_vec![try in bump; 3; 5]
    }
}

#[allow(dead_code)]
fn bump_vec_rev_macro() {
    fn new_in(bump: &mut Bump) -> MutBumpVecRev<u32> {
        mut_bump_vec_rev![in bump]
    }

    fn from_array_in(bump: &mut Bump) -> MutBumpVecRev<u32> {
        mut_bump_vec_rev![in bump; 1, 2, 3]
    }

    fn from_elem_in(bump: &mut Bump) -> MutBumpVecRev<u32> {
        mut_bump_vec_rev![in bump; 3; 5]
    }

    fn try_new_in(bump: &mut Bump) -> Result<MutBumpVecRev<u32>> {
        mut_bump_vec_rev![try in bump]
    }

    fn try_from_array_in(bump: &mut Bump) -> Result<MutBumpVecRev<u32>> {
        mut_bump_vec_rev![try in bump; 1, 2, 3]
    }

    fn try_from_elem_in(bump: &mut Bump) -> Result<MutBumpVecRev<u32>> {
        mut_bump_vec_rev![try in bump; 3; 5]
    }
}

#[allow(dead_code)]
fn bump_format_macro() {
    fn infallible_new(bump: &mut Bump) -> MutBumpString {
        mut_bump_format!(in bump)
    }

    fn fallible_new(bump: &mut Bump) -> Result<MutBumpString> {
        mut_bump_format!(try in bump)
    }

    fn infallible_raw(bump: &mut Bump) -> MutBumpString {
        mut_bump_format!(in bump, r"hey")
    }

    fn fallible_raw(bump: &mut Bump) -> Result<MutBumpString> {
        mut_bump_format!(try in bump, r"hey")
    }

    fn infallible_fmt(bump: &mut Bump) -> MutBumpString {
        let one = 1;
        let two = 2;
        mut_bump_format!(in bump, "{one} + {two} = {}", one + two)
    }

    fn fallible_fmt(bump: &mut Bump) -> Result<MutBumpString> {
        let one = 1;
        let two = 2;
        mut_bump_format!(try in bump, "{one} + {two} = {}", one + two)
    }
}

#[test]
fn zero_capacity() {
    let bump: Bump<Global, 1, false> = Bump::with_capacity(Layout::new::<[u8; 0]>());
    dbg!(bump);
}

#[test]
fn alloc_iter2() {
    let bump: Bump = Bump::new();
    let one = 1;
    let two = 2;
    let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two));

    assert_eq!(string, "1 + 2 = 3");
}

#[test]
fn vec_of_strings() {
    let bump: Bump = Bump::new();
    let mut vec = BumpVec::new_in(&bump);

    for i in 0..10 {
        let string = bump.alloc_fmt(format_args!("hello {i}")).into_ref();
        vec.push(string);
    }

    let slice: &[&str] = vec.into_slice();

    dbg!(&slice);
}

fn bump_vec_shrink_can<const UP: bool>() {
    let bump = Bump::<Global, 1, UP>::with_size(64);

    let mut vec = BumpVec::<i32, Global, 1, UP>::from_array_in([1, 2, 3], &bump);
    let addr = vec.as_ptr().addr();
    assert_eq!(vec.capacity(), 3);
    vec.shrink_to_fit();
    assert_eq!(addr, vec.as_ptr().addr());
    assert_eq!(vec.capacity(), 3);
    vec.clear();
    assert_eq!(addr, vec.as_ptr().addr());
    vec.shrink_to_fit();

    if UP {
        assert_eq!(addr, vec.as_ptr().addr());
    } else {
        assert_ne!(addr, vec.as_ptr().addr());
    }

    assert_eq!(vec.capacity(), 0);
}

fn bump_vec_shrink_cant<const UP: bool>() {
    let bump = Bump::<Global, 1, UP>::with_size(64);

    let mut vec = BumpVec::<i32, Global, 1, UP>::from_array_in([1, 2, 3], &bump);
    let addr = vec.as_ptr().addr();
    assert_eq!(vec.capacity(), 3);
    bump.alloc_str("now you can't shrink haha");

    vec.clear();
    vec.shrink_to_fit();

    assert_eq!(addr, vec.as_ptr().addr());
    assert_eq!(vec.capacity(), 3);
}

fn realign<const UP: bool>() {
    type AlignT = u64;
    const ALIGN: usize = 8;

    // into_aligned
    {
        let bump = Bump::<Global, 1, UP>::with_size(64);
        bump.alloc(0u8);
        assert!(!bump
            .guaranteed_allocated_stats()
            .current
            .bump_position()
            .cast::<AlignT>()
            .is_aligned());
        let bump = bump.into_aligned::<ALIGN>();
        assert!(bump
            .guaranteed_allocated_stats()
            .current
            .bump_position()
            .cast::<AlignT>()
            .is_aligned());
    }

    // as_aligned_mut
    {
        let mut bump = Bump::<Global, 1, UP>::with_size(64);
        bump.alloc(0u8);
        assert!(!bump
            .guaranteed_allocated_stats()
            .current
            .bump_position()
            .cast::<AlignT>()
            .is_aligned());
        let bump = bump.as_aligned_mut::<ALIGN>();
        assert!(bump
            .guaranteed_allocated_stats()
            .current
            .bump_position()
            .cast::<AlignT>()
            .is_aligned());
    }

    // aligned
    {
        let mut bump = Bump::<Global, 1, UP>::with_size(64);
        bump.alloc(0u8);
        assert!(!bump
            .guaranteed_allocated_stats()
            .current
            .bump_position()
            .cast::<AlignT>()
            .is_aligned());
        bump.aligned::<ALIGN, ()>(|bump| {
            assert!(bump
                .guaranteed_allocated_stats()
                .current
                .bump_position()
                .cast::<AlignT>()
                .is_aligned());
        });
    }

    // scoped_aligned
    {
        let mut bump = Bump::<Global, 1, UP>::with_size(64);
        bump.alloc(0u8);
        assert!(!bump
            .guaranteed_allocated_stats()
            .current
            .bump_position()
            .cast::<AlignT>()
            .is_aligned());
        bump.scoped_aligned::<ALIGN, ()>(|bump| {
            assert!(bump
                .guaranteed_allocated_stats()
                .current
                .bump_position()
                .cast::<AlignT>()
                .is_aligned());
        });
    }
}

// https://github.com/bluurryy/bump-scope/issues/6
#[allow(clippy::too_many_lines)]
fn alloc_zst<const UP: bool>() {
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

    let mut bump = Bump::<Global, 1, UP>::new();

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
        let drop_emit = bump.alloc_iter_exact([DropEmit, DropEmit, DropEmit]);
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

    {
        reset();
        let drop_emit = bump.alloc_iter_mut_rev([DropEmit, DropEmit, DropEmit]);
        assert_eq!(DROPS.get(), 0);
        drop(drop_emit);
        assert_eq!(DROPS.get(), 3);
    }
}

fn call_zst_creation_closures<const UP: bool>() {
    let bump = Bump::<Global, 1, UP>::new();

    {
        let mut calls = 0;
        bump.alloc_with(|| {
            calls += 1;
        });
        assert_eq!(calls, 1);
    }

    {
        let mut calls = 0;
        bump.alloc_slice_fill_with(3, || {
            calls += 1;
        });
        assert_eq!(calls, 3);
    }
}

fn deallocate_in_ltr<const UP: bool>() {
    let bump = Bump::<Global, 1, UP>::new();
    let slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    let (lhs, rhs) = slice.split_at(3);

    assert_eq!(lhs, [1, 2, 3]);
    assert_eq!(rhs, [4, 5, 6]);
    assert_eq!(bump.stats().allocated(), 6 * 4);

    lhs.deallocate_in(&bump);

    if UP {
        assert_eq!(bump.stats().allocated(), 6 * 4);
    } else {
        assert_eq!(bump.stats().allocated(), 3 * 4);
    }

    rhs.deallocate_in(&bump);

    if UP {
        assert_eq!(bump.stats().allocated(), 3 * 4);
    } else {
        assert_eq!(bump.stats().allocated(), 0);
    }
}

fn deallocate_in_rtl<const UP: bool>() {
    let bump = Bump::<Global, 1, UP>::new();
    let slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    let (lhs, rhs) = slice.split_at(3);

    assert_eq!(lhs, [1, 2, 3]);
    assert_eq!(rhs, [4, 5, 6]);
    assert_eq!(bump.stats().allocated(), 6 * 4);

    rhs.deallocate_in(&bump);

    if UP {
        assert_eq!(bump.stats().allocated(), 3 * 4);
    } else {
        assert_eq!(bump.stats().allocated(), 6 * 4);
    }

    lhs.deallocate_in(&bump);

    if UP {
        assert_eq!(bump.stats().allocated(), 0);
    } else {
        assert_eq!(bump.stats().allocated(), 3 * 4);
    }
}
