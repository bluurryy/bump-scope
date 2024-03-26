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

#[cfg(feature = "nightly-coerce-unsized")]
mod coerce_unsized;

mod alloc_iter;
mod alloc_slice;

mod allocator_api;
mod bump_vec_doc;
mod from_std;
mod vec;

mod panic_safety;

extern crate std;

type Result<T = (), E = AllocError> = core::result::Result<T, E>;

const OVERHEAD: usize = ChunkSize::<true, Global>::OVERHEAD.get();
const MALLOC_OVERHEAD: usize = ASSUMED_MALLOC_OVERHEAD_SIZE.get();

use crate::{
    bump_format, bump_vec, bump_vec_rev, chunk_size::ASSUMED_MALLOC_OVERHEAD_SIZE, infallible, Bump, BumpBox, BumpScope,
    BumpString, BumpVec, BumpVecRev, Chunk, ChunkHeader, ChunkSize, FmtFn, IntoIter, MinimumAlignment,
    SupportedMinimumAlignment,
};

#[allow(dead_code)]
fn assert_covariant() {
    fn bump_box<'a, 'other>(x: BumpBox<'static, &'static str>) -> BumpBox<'a, &'other str> {
        x
    }

    fn bump_slice_iter<'a, 'other>(x: IntoIter<'static, &'static str>) -> IntoIter<'a, &'other str> {
        x
    }

    // fn bump_vec<'b, 'a, 'other>(x: BumpVec<'b, 'static, &'static str>) -> BumpVec<'b, 'a, &'other str> {
    //     x
    // }

    // fn bump_string<'b, 'a>(x: BumpString<'b, 'static>) -> BumpString<'b, 'a> {
    //     x
    // }
}

macro_rules! either_way {
    ($($ident:ident)*) => {
        mod up {
            $(
                #[test]
                fn $ident() {
                    eprintln!("`UP` is `true`");
                    super::$ident::<true>();
                }
            )*
        }

        mod down {
            $(
                #[test]
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

    bump_vec

    bump_vec_push_pop

    bump_vec_insert

    bump_vec_remove

    bump_vec_swap_remove

    bump_vec_extend

    bump_vec_drop

    bump_vec_write

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
}

macro_rules! assert_chunk_sizes {
    ($bump:expr, $prev:expr, $curr:expr, $next:expr) => {
        let bump = &$bump;
        let curr = bump.stats().current;
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

fn bump_vec<const UP: bool>() {
    const TIMES: usize = 5;

    for (size, count) in [(0, 2), (512, 1)] {
        let mut bump = Bump::<Global, 1, UP>::with_size(size);
        let mut vec = BumpVec::new_in(&mut bump);
        assert_eq!(vec.stats().count(), 1);
        vec.extend(core::iter::repeat(3).take(TIMES));
        assert_eq!(vec.stats().count(), count);
        let _ = vec.into_boxed_slice();
        dbg!(bump.stats());
        assert_eq!(bump.stats().current.allocated(), TIMES * core::mem::size_of::<i32>());
        bump.reset();
        assert_eq!(bump.stats().allocated(), 0);
    }
}

fn bump_vec_push_pop<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = bump_vec![in bump; 1, 2];

    vec.push(3);

    assert_eq!(vec, [1, 2, 3]);
    assert_eq!(vec.pop(), Some(3));
    assert_eq!(vec.pop(), Some(2));
    assert_eq!(vec.pop(), Some(1));
    assert_eq!(vec.pop(), None);

    vec.push(4);
    assert_eq!(vec, [4]);
}

fn bump_vec_insert<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = bump_vec![in bump; 1, 2, 3];

    vec.insert(1, 4);
    assert_eq!(vec, [1, 4, 2, 3]);

    vec.insert(4, 5);
    assert_eq!(vec, [1, 4, 2, 3, 5]);

    vec.insert(0, 6);
    assert_eq!(vec, [6, 1, 4, 2, 3, 5]);
}

fn bump_vec_remove<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = bump_vec![in bump; 1, 2, 3, 4, 5];

    assert_eq!(vec.remove(1), 2);
    assert_eq!(vec, [1, 3, 4, 5]);

    assert_eq!(vec.remove(3), 5);
    assert_eq!(vec, [1, 3, 4]);

    assert_eq!(vec.remove(0), 1);
    assert_eq!(vec, [3, 4]);
}

fn bump_vec_swap_remove<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = bump_vec![in bump; 1, 2, 3, 4, 5];

    assert_eq!(vec.swap_remove(1), 2);
    assert_eq!(vec, [1, 5, 3, 4]);

    assert_eq!(vec.swap_remove(3), 4);
    assert_eq!(vec, [1, 5, 3]);

    assert_eq!(vec.swap_remove(0), 1);
    assert_eq!(vec, [3, 5]);
}

fn bump_vec_extend<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec = bump_vec![in bump];

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

fn bump_vec_drop<const UP: bool>() {
    const SIZE: usize = 32;
    assert_eq!(mem::size_of::<ChunkHeader<Global>>(), SIZE);

    let mut bump = Bump::<Global, 1, UP>::with_size(64);
    assert_eq!(bump.stats().current.size(), 64 - MALLOC_OVERHEAD);
    assert_eq!(bump.stats().capacity(), 64 - OVERHEAD);
    assert_eq!(bump.stats().remaining(), 64 - OVERHEAD);

    let mut vec: BumpVec<u8, Global, 1, UP> = bump_vec![in bump];
    vec.reserve(33);

    assert_eq!(vec.stats().current.size(), 128 - MALLOC_OVERHEAD);
    assert_eq!(vec.stats().current.prev().unwrap().size(), 64 - MALLOC_OVERHEAD);
    assert_eq!(vec.stats().size(), 64 + 128 - MALLOC_OVERHEAD * 2);
    assert_eq!(vec.stats().count(), 2);

    drop(vec);

    assert_eq!(bump.stats().current.size(), 128 - MALLOC_OVERHEAD);
    assert_eq!(bump.stats().size(), 64 + 128 - MALLOC_OVERHEAD * 2);
    assert_eq!(bump.stats().count(), 2);
}

fn bump_vec_write<const UP: bool>() {
    use std::io::Write;

    let mut bump = Bump::<Global, 1, UP>::new();
    let mut vec: BumpVec<u8, Global, 1, UP> = bump_vec![in bump];

    let _ = vec.write(&[0]).unwrap();

    let _ = vec
        .write_vectored(&[IoSlice::new(&[1, 2, 3]), IoSlice::new(&[4, 5, 6, 7, 8])])
        .unwrap();

    let _ = vec.write(&[9, 10]).unwrap();

    assert_eq!(vec, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

fn alloc_iter<const UP: bool>() {
    dbg!(UP);

    let bump = Bump::<Global, 1, UP>::with_size(64);
    // dbg!(&bump);
    let slice_0 = bump.alloc_iter([1, 2, 3]);

    // dbg!(&bump);
    // let slice_1 = bump.alloc_iter([4, 5, 6]);

    // dbg!(&bump);
    assert_eq!(slice_0.as_ref(), [1, 2, 3]);
    // assert_eq!(slice_1.as_ref(), [4, 5, 6]);
}

fn reset_single_chunk<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::with_size(64);
    assert_chunk_sizes!(bump, [], 64, []);
    bump.reset();
    assert_chunk_sizes!(bump, [], 64, []);
}

fn macro_syntax<const UP: bool>() {
    #[allow(clippy::needless_pass_by_value)]
    fn check<T: Debug + PartialEq, const UP: bool>(v: BumpVec<T, Global, 1, UP>, expected: &[T]) {
        dbg!(&v);
        assert_eq!(v, expected);
    }

    let mut bump = Bump::<Global, 1, UP>::new();

    check::<i32, UP>(bump_vec![in bump], &[]);
    check(bump_vec![in bump; 1, 2, 3], &[1, 2, 3]);
    check(bump_vec![in bump; 5; 3], &[5, 5, 5]);

    check::<i32, UP>(bump_vec![in &mut bump], &[]);
    check(bump_vec![in &mut bump; 1, 2, 3], &[1, 2, 3]);
    check(bump_vec![in &mut bump; 5; 3], &[5, 5, 5]);
}

fn debug_sizes<const UP: bool>(bump: &Bump<Global, 1, UP>) {
    let iter = bump.stats().small_to_big();
    let vec = iter.map(Chunk::size).collect::<Vec<_>>();
    let sizes = FmtFn(|f| write!(f, "{vec:?}"));
    dbg!(sizes);
}

fn force_alloc_new_chunk<const UP: bool>(bump: &BumpScope<Global, 1, UP>) {
    let size = bump.stats().current.remaining() + 1;
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
        bump.aligned::<1>(|bump| {
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

#[test]
fn with_drop() {
    thread_local! {
        static DROPS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Default, Debug)]
    struct Foo(#[allow(dead_code)] i32);

    impl Drop for Foo {
        fn drop(&mut self) {
            dbg!(self);
            DROPS.set(DROPS.get() + 1);
        }
    }

    let bump: Bump = Bump::new();
    let bump = bump.with_drop();

    let _one: &mut Foo = infallible(bump.generic_alloc(Foo(1)));
    let _two: &mut Foo = infallible(bump.generic_alloc(Foo(2)));

    assert_eq!(DROPS.get(), 0);

    drop(bump);

    assert_eq!(DROPS.get(), 2);
}

#[allow(dead_code)]
fn api_that_accepts_bump_or_bump_scope() {
    fn vec_from_mut_bump(bump: &mut Bump) -> BumpVec<i32> {
        BumpVec::new_in(bump)
    }

    fn vec_from_mut_bump_scope<'b, 'a>(bump: &'b mut BumpScope<'a>) -> BumpVec<'b, 'a, i32> {
        BumpVec::new_in(bump)
    }

    fn string_from_mut_bump(bump: &mut Bump) -> BumpString {
        BumpString::new_in(bump)
    }

    fn string_from_mut_bump_scope<'b, 'a>(bump: &'b mut BumpScope<'a>) -> BumpString<'b, 'a> {
        BumpString::new_in(bump)
    }
}

#[allow(dead_code)]
fn bump_vec_macro() {
    fn new_in(bump: &mut Bump) -> BumpVec<u32> {
        bump_vec![in bump]
    }

    fn from_array_in(bump: &mut Bump) -> BumpVec<u32> {
        bump_vec![in bump; 1, 2, 3]
    }

    fn from_elem_in(bump: &mut Bump) -> BumpVec<u32> {
        bump_vec![in bump; 3; 5]
    }

    fn try_new_in(bump: &mut Bump) -> Result<BumpVec<u32>> {
        bump_vec![try in bump]
    }

    fn try_from_array_in(bump: &mut Bump) -> Result<BumpVec<u32>> {
        bump_vec![try in bump; 1, 2, 3]
    }

    fn try_from_elem_in(bump: &mut Bump) -> Result<BumpVec<u32>> {
        bump_vec![try in bump; 3; 5]
    }
}

#[allow(dead_code)]
fn bump_vec_rev_macro() {
    fn new_in(bump: &mut Bump) -> BumpVecRev<u32> {
        bump_vec_rev![in bump]
    }

    fn from_array_in(bump: &mut Bump) -> BumpVecRev<u32> {
        bump_vec_rev![in bump; 1, 2, 3]
    }

    fn from_elem_in(bump: &mut Bump) -> BumpVecRev<u32> {
        bump_vec_rev![in bump; 3; 5]
    }

    fn try_new_in(bump: &mut Bump) -> Result<BumpVecRev<u32>> {
        bump_vec_rev![try in bump]
    }

    fn try_from_array_in(bump: &mut Bump) -> Result<BumpVecRev<u32>> {
        bump_vec_rev![try in bump; 1, 2, 3]
    }

    fn try_from_elem_in(bump: &mut Bump) -> Result<BumpVecRev<u32>> {
        bump_vec_rev![try in bump; 3; 5]
    }
}

#[allow(dead_code)]
fn bump_format_macro() {
    fn infallible_new(bump: &mut Bump) -> BumpString {
        bump_format!(in bump)
    }

    fn fallible_new(bump: &mut Bump) -> Result<BumpString> {
        bump_format!(try in bump)
    }

    fn infallible_raw(bump: &mut Bump) -> BumpString {
        bump_format!(in bump, r"hey")
    }

    fn fallible_raw(bump: &mut Bump) -> Result<BumpString> {
        bump_format!(try in bump, r"hey")
    }

    fn infallible_fmt(bump: &mut Bump) -> BumpString {
        let one = 1;
        let two = 2;
        bump_format!(in bump, "{one} + {two} = {}", one + two)
    }

    fn fallible_fmt(bump: &mut Bump) -> Result<BumpString> {
        let one = 1;
        let two = 2;
        bump_format!(try in bump, "{one} + {two} = {}", one + two)
    }
}

#[test]
fn zero_capacity() {
    let bump: Bump<_, 1, false> = Bump::with_capacity(Layout::new::<[u8; 0]>());
    dbg!(bump);
}
