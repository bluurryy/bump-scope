#![expect(unused_imports, clippy::manual_assert)]
#![cfg(feature = "std")]

use core::iter;
use std::{
    alloc::Layout,
    any::Any,
    boxed::Box,
    cell::Cell,
    dbg, eprintln,
    fmt::Debug,
    io::IoSlice,
    mem,
    ops::Index,
    ptr::NonNull,
    string::{String, ToString},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
    thread_local,
    vec::Vec,
};

#[cfg(feature = "nightly-clone-to-uninit")]
mod alloc_clone;
mod alloc_cstr;
mod alloc_fmt;
mod alloc_iter;
mod alloc_slice;
mod alloc_try_with;
mod allocator_api;
mod append;
mod bump_allocator;
mod bump_string;
mod bump_vec;
mod bump_vec_doc;
mod chunk_size;
#[cfg(feature = "nightly-coerce-unsized")]
mod coerce_unsized;
mod fixed_bump_vec;
#[cfg(all(feature = "nightly-fn-traits", feature = "nightly-coerce-unsized"))]
mod fn_traits;
mod grow_vec;
mod into_flattened;
mod io_write;
mod limited_allocator;
mod may_dangle;
mod misalignment_due_to_dealloc;
mod mut_bump_vec_doc;
mod mut_bump_vec_rev_doc;
mod mut_collections_do_not_waste_space;
mod panic_safety;
mod pool;
#[cfg(feature = "serde")]
mod serde;
mod split_off;
mod test_mut_bump_vec;
mod test_mut_bump_vec_rev;
mod test_wrap;
mod unaligned_collection;
mod unallocated;
mod vec;

type Result<T = (), E = AllocError> = core::result::Result<T, E>;

const MALLOC_OVERHEAD: usize = size_of::<AssumedMallocOverhead>();
const OVERHEAD: usize = MALLOC_OVERHEAD + size_of::<ChunkHeader<Global>>();

use crate::{
    Bump, BumpBox, BumpScope, BumpString, BumpVec, MutBumpString, MutBumpVec, MutBumpVecRev, SizedTypeProperties,
    alloc::{AllocError, Allocator, Global as System, Global},
    chunk::ChunkHeader,
    chunk_size::{AssumedMallocOverhead, ChunkSize},
    mut_bump_format, mut_bump_vec, mut_bump_vec_rev, owned_slice, panic_on_error,
    settings::{BumpAllocatorSettings, BumpSettings, MinimumAlignment, SupportedMinimumAlignment, True},
    stats::Chunk,
    traits::{BumpAllocator, BumpAllocatorTyped},
};

pub(crate) use test_wrap::TestWrap;

#[expect(dead_code)]
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
                    std::eprintln!("`UP` is `true`");
                    super::$ident::<true>();
                }
            )*
        }

        mod down {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    std::eprintln!("`UP` is `false`");
                    super::$ident::<false>();
                }
            )*
        }
    };
}

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

    dealloc_ltr

    dealloc_rtl

    default_chunk_size

    min_chunk_size
}

macro_rules! assert_chunk_sizes {
    ($bump:expr, $prev:expr, $curr:expr, $next:expr) => {
        let bump = &$bump;
        let curr = bump.stats().current_chunk();
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

fn assert_send<const UP: bool>() {
    fn must_be_send<T: Send>(_: &T) {}
    let bump = Bump::<Global, BumpSettings<1, UP>>::default();
    must_be_send(&bump);
}

fn mut_bump_vec<const UP: bool>() {
    const TIMES: usize = 5;

    for (size, count) in [(0, 2), (512, 1)] {
        let mut bump = Bump::<Global, BumpSettings<1, UP>>::with_size(size);
        let mut vec = MutBumpVec::new_in(&mut bump);
        assert_eq!(vec.allocator_stats().count(), 1);
        vec.extend(iter::repeat_n(3, TIMES));
        assert_eq!(vec.allocator_stats().count(), count);
        let _ = vec.into_boxed_slice();
        dbg!(bump.stats());
        assert_eq!(bump.stats().current_chunk().allocated(), TIMES * core::mem::size_of::<i32>());
        bump.reset();
        assert_eq!(bump.stats().allocated(), 0);
    }
}

fn mut_bump_vec_push_pop<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();
    let mut vec = mut_bump_vec![in &mut bump; 1, 2];

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
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();
    let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];

    vec.insert(1, 4);
    assert_eq!(vec, [1, 4, 2, 3]);

    vec.insert(4, 5);
    assert_eq!(vec, [1, 4, 2, 3, 5]);

    vec.insert(0, 6);
    assert_eq!(vec, [6, 1, 4, 2, 3, 5]);
}

fn mut_bump_vec_remove<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();
    let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3, 4, 5];

    assert_eq!(vec.remove(1), 2);
    assert_eq!(vec, [1, 3, 4, 5]);

    assert_eq!(vec.remove(3), 5);
    assert_eq!(vec, [1, 3, 4]);

    assert_eq!(vec.remove(0), 1);
    assert_eq!(vec, [3, 4]);
}

fn mut_bump_vec_swap_remove<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();
    let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3, 4, 5];

    assert_eq!(vec.swap_remove(1), 2);
    assert_eq!(vec, [1, 5, 3, 4]);

    assert_eq!(vec.swap_remove(3), 4);
    assert_eq!(vec, [1, 5, 3]);

    assert_eq!(vec.swap_remove(0), 1);
    assert_eq!(vec, [3, 5]);
}

fn mut_bump_vec_extend<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();
    let mut vec = mut_bump_vec![in &mut bump];

    vec.extend([1, 2, 3]);
    assert_eq!(vec, [1, 2, 3]);

    vec.clear();
    assert!(vec.is_empty());

    vec.append([1, 2, 3]);
    assert_eq!(vec, [1, 2, 3]);

    vec.extend_from_slice_copy(&[4, 5, 6]);
    assert_eq!(vec, [1, 2, 3, 4, 5, 6]);

    vec.extend_from_slice_clone(&[7, 8, 9]);
    assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

fn mut_bump_vec_drop<const UP: bool>() {
    const SIZE: usize = 32;
    assert_eq!(mem::size_of::<ChunkHeader<Global>>(), SIZE);

    let mut bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);
    assert_eq!(bump.stats().current_chunk().size(), 64 - MALLOC_OVERHEAD);
    assert_eq!(bump.stats().capacity(), 64 - OVERHEAD);
    assert_eq!(bump.stats().remaining(), 64 - OVERHEAD);

    let mut vec: MutBumpVec<u8, _> = mut_bump_vec![in &mut bump];
    vec.reserve(33);

    assert_eq!(vec.allocator_stats().current_chunk().size(), 128 - MALLOC_OVERHEAD);
    assert_eq!(
        vec.allocator_stats().current_chunk().prev().unwrap().size(),
        64 - MALLOC_OVERHEAD
    );
    assert_eq!(vec.allocator_stats().size(), 64 + 128 - MALLOC_OVERHEAD * 2);
    assert_eq!(vec.allocator_stats().count(), 2);

    drop(vec);

    assert_eq!(bump.stats().current_chunk().size(), 128 - MALLOC_OVERHEAD);
    assert_eq!(bump.stats().size(), 64 + 128 - MALLOC_OVERHEAD * 2);
    assert_eq!(bump.stats().count(), 2);
}

fn mut_bump_vec_write<const UP: bool>() {
    use std::io::Write;

    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();
    let mut vec: MutBumpVec<u8, _> = mut_bump_vec![in &mut bump];

    let _ = vec.write(&[0]).unwrap();

    let _ = vec
        .write_vectored(&[IoSlice::new(&[1, 2, 3]), IoSlice::new(&[4, 5, 6, 7, 8])])
        .unwrap();

    let _ = vec.write(&[9, 10]).unwrap();

    assert_eq!(vec, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

fn alloc_iter<const UP: bool>() {
    let bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);

    let slice_0 = bump.alloc_iter([1, 2, 3]);
    let slice_1 = bump.alloc_iter([4, 5, 6]);

    assert_eq!(slice_0.as_ref(), [1, 2, 3]);
    assert_eq!(slice_1.as_ref(), [4, 5, 6]);
}

fn reset_single_chunk<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);
    assert_chunk_sizes!(bump, [], 64, []);
    bump.reset();
    assert_chunk_sizes!(bump, [], 64, []);
}

fn macro_syntax<const UP: bool>() {
    #[expect(clippy::needless_pass_by_value)]
    fn check<T: Debug + PartialEq, const UP: bool>(
        v: MutBumpVec<T, &mut Bump<Global, BumpSettings<1, UP>>>,
        expected: &[T],
    ) {
        dbg!(&v);
        assert_eq!(v, expected);
    }

    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();

    check::<i32, UP>(mut_bump_vec![in &mut bump], &[]);
    check(mut_bump_vec![in &mut bump; 1, 2, 3], &[1, 2, 3]);
    check(mut_bump_vec![in &mut bump; 5; 3], &[5, 5, 5]);

    check::<i32, UP>(mut_bump_vec![in &mut bump], &[]);
    check(mut_bump_vec![in &mut bump; 1, 2, 3], &[1, 2, 3]);
    check(mut_bump_vec![in &mut bump; 5; 3], &[5, 5, 5]);
}

fn debug_sizes<const UP: bool>(bump: &Bump<Global, BumpSettings<1, UP>>) {
    let iter = bump.stats().small_to_big();
    let vec = iter.map(Chunk::size).collect::<Vec<_>>();
    eprintln!("sizes: {vec:?}");
}

fn force_alloc_new_chunk<const UP: bool>(bump: &BumpScope<Global, BumpSettings<1, UP>>) {
    let size = bump.stats().current_chunk().remaining() + 1;
    let layout = Layout::from_size_align(size, 1).unwrap();
    bump.allocate_layout(layout);
}

fn reset_first_chunk<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);

    bump.scoped(|scope| {
        force_alloc_new_chunk(&scope);
        force_alloc_new_chunk(&scope);
    });

    assert_chunk_sizes!(bump, [], 64, [128, 256]);
    bump.reset();
    assert_chunk_sizes!(bump, [], 256, []);
}

fn reset_middle_chunk<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);
    force_alloc_new_chunk(bump.as_scope());

    bump.scoped(|scope| {
        force_alloc_new_chunk(&scope);
    });

    assert_chunk_sizes!(bump, [64], 128, [256]);
    bump.reset();
    assert_chunk_sizes!(bump, [], 256, []);
}

fn reset_last_chunk<const UP: bool>() {
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);
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
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();

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
    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();

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
    let bump = Bump::<Global, BumpSettings<1, UP>>::default();
    dbg!(&bump);
    bump.reserve_bytes(256);
    assert!(bump.stats().remaining() > 256);
}

fn aligned<const UP: bool>() {
    let mut bump: Bump<Global, BumpSettings<8, UP>> = Bump::new();

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
    let mut bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    {
        let bump = bump.as_mut_aligned::<8>();
        assert_eq!(bump.typed_stats().allocated(), 0);
    }

    {
        bump.alloc(1u8);
        let bump = bump.as_mut_aligned::<8>();
        assert_eq!(bump.typed_stats().allocated(), 8);
        bump.alloc(2u16);
        assert_eq!(bump.typed_stats().allocated(), 16);
        bump.alloc(4u32);
        assert_eq!(bump.typed_stats().allocated(), 24);
        bump.alloc(8u64);
        assert_eq!(bump.typed_stats().allocated(), 32);
    }
}

#[expect(dead_code)]
fn api_that_accepts_bump_or_bump_scope() {
    fn vec_from_mut_bump(bump: &mut Bump) -> MutBumpVec<i32, &mut Bump> {
        MutBumpVec::new_in(bump)
    }

    fn vec_from_mut_bump_scope<'b, 'a>(bump: &'b mut BumpScope<'a>) -> MutBumpVec<i32, &'b mut BumpScope<'a>> {
        MutBumpVec::new_in(bump)
    }

    fn string_from_mut_bump(bump: &mut Bump) -> MutBumpString<&mut Bump> {
        MutBumpString::new_in(bump)
    }

    fn string_from_mut_bump_scope<'b, 'a>(bump: &'b mut BumpScope<'a>) -> MutBumpString<&'b mut BumpScope<'a>> {
        MutBumpString::new_in(bump)
    }
}

#[expect(dead_code)]
fn bump_vec_macro() {
    fn new_in(bump: &mut Bump) -> MutBumpVec<u32, &mut Bump> {
        mut_bump_vec![in bump]
    }

    fn from_array_in(bump: &mut Bump) -> MutBumpVec<u32, &mut Bump> {
        mut_bump_vec![in bump; 1, 2, 3]
    }

    fn from_elem_in(bump: &mut Bump) -> MutBumpVec<u32, &mut Bump> {
        mut_bump_vec![in bump; 3; 5]
    }

    fn try_new_in(bump: &mut Bump) -> Result<MutBumpVec<u32, &mut Bump>> {
        mut_bump_vec![try in bump]
    }

    fn try_from_array_in(bump: &mut Bump) -> Result<MutBumpVec<u32, &mut Bump>> {
        mut_bump_vec![try in bump; 1, 2, 3]
    }

    fn try_from_elem_in(bump: &mut Bump) -> Result<MutBumpVec<u32, &mut Bump>> {
        mut_bump_vec![try in bump; 3; 5]
    }
}

#[expect(dead_code)]
fn bump_vec_rev_macro() {
    fn new_in(bump: &mut Bump) -> MutBumpVecRev<u32, &mut Bump> {
        mut_bump_vec_rev![in bump]
    }

    fn from_array_in(bump: &mut Bump) -> MutBumpVecRev<u32, &mut Bump> {
        mut_bump_vec_rev![in bump; 1, 2, 3]
    }

    fn from_elem_in(bump: &mut Bump) -> MutBumpVecRev<u32, &mut Bump> {
        mut_bump_vec_rev![in bump; 3; 5]
    }

    fn try_new_in(bump: &mut Bump) -> Result<MutBumpVecRev<u32, &mut Bump>> {
        mut_bump_vec_rev![try in bump]
    }

    fn try_from_array_in(bump: &mut Bump) -> Result<MutBumpVecRev<u32, &mut Bump>> {
        mut_bump_vec_rev![try in bump; 1, 2, 3]
    }

    fn try_from_elem_in(bump: &mut Bump) -> Result<MutBumpVecRev<u32, &mut Bump>> {
        mut_bump_vec_rev![try in bump; 3; 5]
    }
}

#[expect(dead_code)]
fn bump_format_macro() {
    fn infallible_new(bump: &mut Bump) -> MutBumpString<&mut Bump> {
        mut_bump_format!(in bump)
    }

    fn fallible_new(bump: &mut Bump) -> Result<MutBumpString<&mut Bump>> {
        mut_bump_format!(try in bump)
    }

    fn infallible_raw(bump: &mut Bump) -> MutBumpString<&mut Bump> {
        mut_bump_format!(in bump, r"hey")
    }

    fn fallible_raw(bump: &mut Bump) -> Result<MutBumpString<&mut Bump>> {
        mut_bump_format!(try in bump, r"hey")
    }

    fn infallible_fmt(bump: &mut Bump) -> MutBumpString<&mut Bump> {
        let one = 1;
        let two = 2;
        mut_bump_format!(in bump, "{one} + {two} = {}", one + two)
    }

    fn fallible_fmt(bump: &mut Bump) -> Result<MutBumpString<&mut Bump>> {
        let one = 1;
        let two = 2;
        mut_bump_format!(try in bump, "{one} + {two} = {}", one + two)
    }
}

#[test]
fn zero_capacity() {
    let bump: Bump<Global, BumpSettings<1, false>> = Bump::with_capacity(Layout::new::<[u8; 0]>());
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
    let bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);

    let mut vec = BumpVec::<i32, _>::from_owned_slice_in([1, 2, 3], &bump);
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
    let bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);

    let mut vec = BumpVec::<i32, _>::from_owned_slice_in([1, 2, 3], &bump);
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
        let bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);
        bump.alloc(0u8);
        assert!(!bump.stats().current_chunk().bump_position().cast::<AlignT>().is_aligned());
        let bump = bump.into_aligned::<ALIGN>();
        assert!(bump.stats().current_chunk().bump_position().cast::<AlignT>().is_aligned());
    }

    // as_mut_aligned
    {
        let mut bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);
        bump.alloc(0u8);
        assert!(!bump.stats().current_chunk().bump_position().cast::<AlignT>().is_aligned());
        let bump = bump.as_mut_aligned::<ALIGN>();
        assert!(
            bump.typed_stats()
                .current_chunk()
                .bump_position()
                .cast::<AlignT>()
                .is_aligned()
        );
    }

    // aligned
    {
        let mut bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);
        bump.alloc(0u8);
        assert!(!bump.stats().current_chunk().bump_position().cast::<AlignT>().is_aligned());
        bump.aligned::<ALIGN, ()>(|bump| {
            assert!(bump.stats().current_chunk().bump_position().cast::<AlignT>().is_aligned());
        });
    }

    // scoped_aligned
    {
        let mut bump = Bump::<Global, BumpSettings<1, UP>>::with_size(64);
        bump.alloc(0u8);
        assert!(!bump.stats().current_chunk().bump_position().cast::<AlignT>().is_aligned());
        bump.scoped_aligned::<ALIGN, ()>(|bump| {
            assert!(bump.stats().current_chunk().bump_position().cast::<AlignT>().is_aligned());
        });
    }
}

// https://github.com/bluurryy/bump-scope/issues/6
#[expect(clippy::too_many_lines)]
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

    let mut bump = Bump::<Global, BumpSettings<1, UP>>::new();

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
            });
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
    let bump = Bump::<Global, BumpSettings<1, UP>>::new();

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

fn dealloc_ltr<const UP: bool>() {
    let bump = Bump::<Global, BumpSettings<1, UP>>::new();
    let slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    let (lhs, rhs) = slice.split_at(3);

    assert_eq!(lhs, [1, 2, 3]);
    assert_eq!(rhs, [4, 5, 6]);
    assert_eq!(bump.stats().allocated(), 6 * 4);

    bump.dealloc(lhs);

    if UP {
        assert_eq!(bump.stats().allocated(), 6 * 4);
    } else {
        assert_eq!(bump.stats().allocated(), 3 * 4);
    }

    bump.dealloc(rhs);

    if UP {
        assert_eq!(bump.stats().allocated(), 3 * 4);
    } else {
        assert_eq!(bump.stats().allocated(), 0);
    }
}

fn dealloc_rtl<const UP: bool>() {
    let bump = Bump::<Global, BumpSettings<1, UP>>::new();
    let slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    let (lhs, rhs) = slice.split_at(3);

    assert_eq!(lhs, [1, 2, 3]);
    assert_eq!(rhs, [4, 5, 6]);
    assert_eq!(bump.stats().allocated(), 6 * 4);

    bump.dealloc(rhs);

    if UP {
        assert_eq!(bump.stats().allocated(), 3 * 4);
    } else {
        assert_eq!(bump.stats().allocated(), 6 * 4);
    }

    bump.dealloc(lhs);

    if UP {
        assert_eq!(bump.stats().allocated(), 0);
    } else {
        assert_eq!(bump.stats().allocated(), 3 * 4);
    }
}

#[test]
fn min_non_zero_cap() {
    fn bump_with_remaining_capacity(capacity: usize) -> Bump {
        let bump = Bump::with_capacity(Layout::array::<u8>(capacity).unwrap());
        let to_be_occupied = bump.stats().capacity() - capacity;
        bump.alloc_uninit_slice::<u8>(to_be_occupied);
        bump
    }

    #[expect(dead_code)]
    struct Big([u8; 1025]);

    impl Default for Big {
        fn default() -> Self {
            Self([0u8; 1025])
        }
    }

    fn test_vecs<T: Default>(expected_capacity: usize) {
        {
            let bump: Bump = Bump::new();
            let mut vec: BumpVec<T, _> = BumpVec::new_in(&bump);
            vec.push_with(Default::default);
            assert_eq!(vec.capacity(), expected_capacity, "{}", std::any::type_name::<T>());
        }

        {
            let mut bump = bump_with_remaining_capacity(size_of::<T>() * expected_capacity);
            let mut vec: MutBumpVec<T, _> = MutBumpVec::new_in(&mut bump);
            vec.push_with(Default::default);
            assert_eq!(vec.capacity(), expected_capacity, "{}", std::any::type_name::<T>());
            drop(vec);
        }

        {
            let mut bump = bump_with_remaining_capacity(size_of::<T>() * expected_capacity);
            let mut vec: MutBumpVecRev<T, _> = MutBumpVecRev::new_in(&mut bump);
            vec.push_with(Default::default);
            assert_eq!(vec.capacity(), expected_capacity, "{}", std::any::type_name::<T>());
            drop(vec);
        }
    }

    test_vecs::<u8>(8);
    test_vecs::<[u8; 4]>(4);
    test_vecs::<Big>(1);

    {
        let bump: Bump = Bump::new();
        let mut string = BumpString::new_in(&bump);
        string.push('a');
        assert_eq!(string.capacity(), 8);
        drop(string);
    }

    {
        let mut bump = bump_with_remaining_capacity(8);
        let mut string = MutBumpString::new_in(&mut bump);
        string.push('a');
        assert_eq!(string.capacity(), 8);
        drop(string);
    }
}

// test claim in crate docs that layout equals `Cell<NonNull<()>>`
// make sure to also check that niches are the same
mod doc_layout_claim {
    use crate::{SizedTypeProperties, alloc::Global};
    use core::{cell::Cell, ptr::NonNull};
    type Bump = crate::Bump<Global>;
    type BumpScope = crate::BumpScope<'static, Global>;
    type Comparand = Cell<NonNull<()>>;
    const _: () = assert!(Bump::SIZE == Comparand::SIZE && Bump::ALIGN == Comparand::ALIGN);
    const _: () = assert!(BumpScope::SIZE == Comparand::SIZE && BumpScope::ALIGN == Comparand::ALIGN);
    const _: () =
        assert!(Option::<Bump>::SIZE == Option::<Comparand>::SIZE && Option::<Bump>::ALIGN == Option::<Comparand>::ALIGN);
    const _: () = assert!(
        Option::<BumpScope>::SIZE == Option::<Comparand>::SIZE && Option::<BumpScope>::ALIGN == Option::<Comparand>::ALIGN
    );
}

fn expect_no_panic<T>(result: Result<T, Box<dyn Any + Send>>) -> T {
    match result {
        Ok(value) => value,
        Err(payload) => unexpected_panic(payload),
    }
}

#[expect(clippy::match_wild_err_arm)]
fn unexpected_panic(payload: Box<dyn Any + Send>) -> ! {
    match panic_payload_string(payload) {
        Ok(msg) => panic!("unexpected panic: {msg}"),
        Err(_) => panic!("unexpected panic (no message)"),
    }
}

fn panic_payload_string(payload: Box<dyn Any + Send>) -> Result<String, Box<dyn Any + Send>> {
    let payload = match payload.downcast::<&'static str>() {
        Ok(string) => return Ok(string.to_string()),
        Err(payload) => payload,
    };

    let payload = match payload.downcast::<String>() {
        Ok(string) => return Ok(*string),
        Err(payload) => payload,
    };

    Err(payload)
}

fn default_chunk_size<const UP: bool>() {
    assert_eq!(
        Bump::<Global, BumpSettings<1, UP>>::new().stats().size(),
        512 - size_of::<[usize; 2]>()
    );
}

fn min_chunk_size<const UP: bool>() {
    assert_eq!(
        Bump::<Global, BumpSettings<1, UP>>::with_size(0).stats().size(),
        64 - size_of::<[usize; 2]>()
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
    let bump = Bump::<_>::new_in(allocator.clone());
    drop(bump);
    assert_eq!(drop_count.get(), 1);

    let bump = Bump::<_>::new_in(allocator);
    bump.reserve_bytes(1024);
    drop(bump);
    assert_eq!(drop_count.get(), 3);
}

#[test]
fn generic_bump_scope() {
    fn foo(mut bump: impl BumpAllocator<Settings: BumpAllocatorSettings<GuaranteedAllocated = True>>) {
        assert_eq!(bump.as_scope().stats().allocated(), 0);
        bump.as_scope().alloc_str("good");
        assert_eq!(bump.as_scope().stats().allocated(), 4);
        bump.scoped(|mut bump| {
            bump.alloc_str("day");
            assert_eq!(bump.stats().allocated(), 7);
            bump.scoped(|bump| {
                bump.alloc_str("world");
                assert_eq!(bump.stats().allocated(), 12);
            });
            assert_eq!(bump.stats().allocated(), 7);
        });
        assert_eq!(bump.as_scope().stats().allocated(), 4);
    }

    let bump: Bump = Bump::new();
    foo(bump);

    let mut bump: Bump = Bump::new();
    bump.scoped(|bump| {
        foo(bump);
    });
}
