//! Adapted from rust's `library/alloc/tests/vec.rs` commit 4426e9a3c260e329f51c94e2b231f72574271f0b

use std::{
    alloc::System,
    assert_eq,
    assert_matches::assert_matches,
    borrow::Cow,
    boxed::Box,
    cell::Cell,
    dbg,
    fmt::Debug,
    format, hint,
    iter::{InPlaceIterable, IntoIterator},
    mem::{self, size_of, swap},
    num::NonZero,
    ops::Bound::*,
    panic::{catch_unwind, AssertUnwindSafe},
    println,
    ptr::NonNull,
    rc::Rc,
    string::{String, ToString},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, PoisonError,
    },
    vec::{Drain, IntoIter, Vec},
};

use allocator_api2::alloc::{AllocError, Allocator, Layout};

use crate::{bump_vec, Bump, BumpVec};

struct DropCounter<'a> {
    count: &'a mut u32,
}

impl Drop for DropCounter<'_> {
    fn drop(&mut self) {
        *self.count += 1;
    }
}

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

#[test]
fn test_small_vec_struct() {
    assert_eq!(size_of::<BumpVec<u8, Bump>>(), size_of::<usize>() * 4);
}

#[test]
fn test_double_drop() {
    let bump_x: Bump = Bump::new();
    let bump_y: Bump = Bump::new();

    struct TwoVec<'a, T> {
        x: BumpVec<T, &'a Bump>,
        y: BumpVec<T, &'a Bump>,
    }

    let (mut count_x, mut count_y) = (0, 0);
    {
        let mut tv = TwoVec {
            x: BumpVec::new_in(&bump_x),
            y: BumpVec::new_in(&bump_y),
        };
        tv.x.push(DropCounter { count: &mut count_x });
        tv.y.push(DropCounter { count: &mut count_y });

        // If BumpVec had a drop flag, here is where it would be zeroed.
        // Instead, it should rely on its internal state to prevent
        // doing anything significant when dropped multiple times.
        drop(tv.x);

        // Here tv goes out of scope, tv.y should be dropped, but not tv.x.
    }

    assert_eq!(count_x, 1);
    assert_eq!(count_y, 1);
}

#[test]
fn test_reserve() {
    let bump: Bump = Bump::new();

    let mut v = BumpVec::new_in(&bump);
    assert_eq!(v.capacity(), 0);

    v.reserve(2);
    assert!(v.capacity() >= 2);

    for i in 0..16 {
        v.push(i);
    }

    assert!(v.capacity() >= 16);
    v.reserve(16);
    assert!(v.capacity() >= 32);

    v.push(16);

    v.reserve(16);
    assert!(v.capacity() >= 33)
}

#[test]
fn test_reserve_exact() {
    // This is all the same as test_reserve

    let bump: Bump = Bump::new();

    let mut v = BumpVec::new_in(&bump);
    assert_eq!(v.capacity(), 0);

    v.reserve_exact(2);
    assert!(v.capacity() >= 2);

    for i in 0..16 {
        v.push(i);
    }

    assert!(v.capacity() >= 16);
    v.reserve_exact(16);
    assert!(v.capacity() >= 32);

    v.push(16);

    v.reserve_exact(16);
    assert!(v.capacity() >= 33)
}

#[test]
#[cfg_attr(miri, ignore)] // Miri does not support signalling OOM
fn test_try_with_capacity() {
    let bump: Bump = Bump::new();

    let mut vec = BumpVec::<i32, _>::try_with_capacity_in(5, &bump).unwrap();
    assert_eq!(0, vec.len());
    assert!(vec.capacity() >= 5 && vec.capacity() <= isize::MAX as usize / 4);
    assert!(vec.spare_capacity_mut().len() >= 5);

    assert!(BumpVec::<u16, _>::try_with_capacity_in(isize::MAX as usize + 1, &bump).is_err());
}

#[test]
fn test_zst_capacity() {
    let bump: Bump = Bump::new();

    assert_eq!(BumpVec::<(), _>::new_in(&bump).capacity(), usize::MAX);
}

#[test]
fn test_indexing() {
    let bump: Bump = Bump::new();

    let v: BumpVec<isize, _> = bump_vec![in &bump; 10, 20];
    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    let mut x: usize = 0;
    assert_eq!(v[x], 10);
    assert_eq!(v[x + 1], 20);
    x = x + 1;
    assert_eq!(v[x], 20);
    assert_eq!(v[x - 1], 10);
}

#[test]
fn test_debug_fmt() {
    let bump1: Bump = Bump::new();
    let bump2: Bump = Bump::new();

    let vec1: BumpVec<isize, _> = bump_vec![in &bump1; ];
    assert_eq!("[]", format!("{:?}", vec1));

    let vec2 = bump_vec![in &bump2; 0, 1];
    assert_eq!("[0, 1]", format!("{:?}", vec2));

    let slice: &[isize] = &[4, 5];
    assert_eq!("[4, 5]", format!("{slice:?}"));
}

#[test]
fn test_push() {
    let bump: Bump = Bump::new();

    let mut v = bump_vec![in &bump; ];
    v.push(1);
    assert_eq!(v, [1]);
    v.push(2);
    assert_eq!(v, [1, 2]);
    v.push(3);
    assert_eq!(v, [1, 2, 3]);
}

#[test]
fn test_extend() {
    let bump_v: Bump = Bump::new();
    let bump_w: Bump = Bump::new();

    let mut v = BumpVec::<i32, _>::new_in(&bump_v);
    let mut w = BumpVec::<i32, _>::new_in(&bump_w);

    v.extend(&w);
    assert!(v.is_empty());

    v.extend(0..3);
    for i in 0..3 {
        w.push(i)
    }

    assert_eq!(v, w);

    v.extend(3..10);
    for i in 3..10 {
        w.push(i)
    }

    assert_eq!(v, w);

    v.extend(&w); // specializes to `append`
    assert!(v.iter().eq(w.iter().chain(w.iter())));

    // Zero sized types
    #[derive(PartialEq, Debug)]
    struct Foo;

    let bump_a: Bump = Bump::new();
    let bump_b: Bump = Bump::new();

    let mut a = BumpVec::new_in(&bump_a);
    let b = bump_vec![in &bump_b; Foo, Foo];

    a.extend(b);
    assert_eq!(a, &[Foo, Foo]);

    // Double drop
    let mut count_x = 0;
    {
        let bump_x: Bump = Bump::new();
        let bump_y: Bump = Bump::new();

        let mut x = BumpVec::new_in(&bump_x);
        let y = bump_vec![in &bump_y; DropCounter { count: &mut count_x }];
        x.extend(y);
    }
    assert_eq!(count_x, 1);
}

#[test]
fn test_extend_from_slice() {
    let bump_a: Bump = Bump::new();
    let bump_b: Bump = Bump::new();

    let a: BumpVec<isize, _> = bump_vec![in &bump_a; 1, 2, 3, 4, 5];
    let b: BumpVec<isize, _> = bump_vec![in &bump_b; 6, 7, 8, 9, 0];

    let mut v: BumpVec<isize, _> = a;

    v.extend_from_slice_copy(&b);

    assert_eq!(v, [1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
}

#[test]
fn test_extend_ref() {
    let bump_v: Bump = Bump::new();
    let bump_w: Bump = Bump::new();

    let mut v = bump_vec![in &bump_v; 1, 2];
    v.extend(&[3, 4, 5]);

    assert_eq!(v.len(), 5);
    assert_eq!(v, [1, 2, 3, 4, 5]);

    let w = bump_vec![in &bump_w; 6, 7];
    v.extend(&w);

    assert_eq!(v.len(), 7);
    assert_eq!(v, [1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn test_slice_from_ref() {
    let bump: Bump = Bump::new();

    let values = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let slice = &values[1..3];

    assert_eq!(slice, [2, 3]);
}

#[test]
fn test_slice_from_mut() {
    let bump: Bump = Bump::new();

    let mut values = bump_vec![in &bump; 1, 2, 3, 4, 5];
    {
        let slice = &mut values[2..];
        assert!(slice == [3, 4, 5]);
        for p in slice {
            *p += 2;
        }
    }

    assert!(values == [1, 2, 5, 6, 7]);
}

#[test]
fn test_slice_to_mut() {
    let bump: Bump = Bump::new();

    let mut values = bump_vec![in &bump; 1, 2, 3, 4, 5];
    {
        let slice = &mut values[..2];
        assert!(slice == [1, 2]);
        for p in slice {
            *p += 1;
        }
    }

    assert!(values == [2, 3, 3, 4, 5]);
}

#[test]
fn test_split_at_mut() {
    let bump: Bump = Bump::new();

    let mut values = bump_vec![in &bump; 1, 2, 3, 4, 5];
    {
        let (left, right) = values.split_at_mut(2);
        {
            let left: &[_] = left;
            assert!(&left[..left.len()] == &[1, 2]);
        }
        for p in left {
            *p += 1;
        }

        {
            let right: &[_] = right;
            assert!(&right[..right.len()] == &[3, 4, 5]);
        }
        for p in right {
            *p += 2;
        }
    }

    assert_eq!(values, [2, 3, 5, 6, 7]);
}

#[test]
fn test_retain() {
    let bump: Bump = Bump::new();

    let mut vec = bump_vec![in &bump; 1, 2, 3, 4];
    vec.retain(|&mut x| x % 2 == 0);
    assert_eq!(vec, [2, 4]);
}

#[test]
fn test_retain_predicate_order() {
    let bump: Bump = Bump::new();

    for to_keep in [true, false] {
        let mut number_of_executions = 0;
        let mut vec = bump_vec![in &bump; 1, 2, 3, 4];
        let mut next_expected = 1;
        vec.retain(|&mut x| {
            assert_eq!(next_expected, x);
            next_expected += 1;
            number_of_executions += 1;
            to_keep
        });
        assert_eq!(number_of_executions, 4);
    }
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_retain_pred_panic_with_hole() {
    let bump: Bump = Bump::new();
    let mut v = BumpVec::new_in(&bump);

    v.extend((0..5).map(Rc::new));

    catch_unwind(AssertUnwindSafe(|| {
        let bump_for_clone: Bump = Bump::new();
        let mut v_clone = BumpVec::new_in(&bump_for_clone);
        v_clone.extend(v.iter().cloned());

        v.retain(|r| match **r {
            0 => true,
            1 => false,
            2 => true,
            _ => panic!(),
        });
    }))
    .unwrap_err();
    // Everything is dropped when predicate panicked.
    assert!(v.iter().all(|r| Rc::strong_count(r) == 1));
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_retain_pred_panic_no_hole() {
    let bump: Bump = Bump::new();
    let v = bump.alloc_iter((0..5).map(Rc::new));

    catch_unwind(AssertUnwindSafe(|| {
        let mut v = bump.alloc_slice_clone(&v);
        v.retain(|r| match **r {
            0 | 1 | 2 => true,
            _ => panic!(),
        });
    }))
    .unwrap_err();

    // Everything is dropped when predicate panicked.
    assert!(v.iter().all(|r| Rc::strong_count(r) == 1));
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_retain_drop_panic() {
    struct Wrap(Rc<i32>);

    impl Drop for Wrap {
        fn drop(&mut self) {
            if *self.0 == 3 {
                panic!();
            }
        }
    }

    let bump: Bump = Bump::new();
    let v = bump.alloc_iter((0..5).map(Rc::new));

    catch_unwind(AssertUnwindSafe(|| {
        let mut v = bump.alloc_iter(v.iter().map(|r| Wrap(r.clone())));

        v.retain(|w| match *w.0 {
            0 => true,
            1 => false,
            2 => true,
            3 => false, // Drop panic.
            _ => true,
        });
    }))
    .unwrap_err();
    // Other elements are dropped when `drop` of one element panicked.
    // The panicked wrapper also has its Rc dropped.
    assert!(v.iter().all(|r| Rc::strong_count(r) == 1));
}

#[test]
fn test_retain_maybeuninits() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    let bumps = core::iter::repeat_with::<Bump, _>(Bump::new).take(4).collect::<Vec<_>>();

    // This test aimed to be run under miri.
    use core::mem::MaybeUninit;
    let mut vec = bump0.alloc_iter(
        [1i32, 2, 3, 4]
            .iter()
            .zip(&bumps)
            .map(|(v, bump)| MaybeUninit::new(bump_vec![in bump; v])),
    );

    vec.retain(|x| {
        // SAFETY: Retain must visit every element of BumpVec in original order and exactly once.
        // Our values is initialized at creation of BumpVec.
        let v = unsafe { x.assume_init_ref()[0] };
        if v & 1 == 0 {
            return true;
        }
        // SAFETY: Value is initialized.
        // Value wouldn't be dropped by `BumpVec::retain`
        // because `MaybeUninit` doesn't drop content.
        drop(unsafe { x.assume_init_read() });
        false
    });

    let vec = bump1.alloc_iter(vec.into_iter().map(|x| unsafe {
        // SAFETY: All values dropped in retain predicate must be removed by `BumpVec::retain`.
        // Remaining values are initialized.
        *x.assume_init()[0]
    }));

    assert_eq!(vec.as_slice(), &[2, 4]);
}

#[test]
fn test_dedup() {
    fn case(a: BumpVec<i32, &Bump>, b: BumpVec<i32, &Bump>) {
        let mut v = a;
        v.dedup();
        assert_eq!(v, b);
    }

    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    case(bump_vec![in &bump0; ], bump_vec![in &bump1; ]);
    case(bump_vec![in &bump0; 1], bump_vec![in &bump1; 1]);
    case(bump_vec![in &bump0; 1, 1], bump_vec![in &bump1; 1]);
    case(bump_vec![in &bump0; 1, 2, 3], bump_vec![in &bump1; 1, 2, 3]);
    case(bump_vec![in &bump0; 1, 1, 2, 3], bump_vec![in &bump1; 1, 2, 3]);
    case(bump_vec![in &bump0; 1, 2, 2, 3], bump_vec![in &bump1; 1, 2, 3]);
    case(bump_vec![in &bump0; 1, 2, 3, 3], bump_vec![in &bump1; 1, 2, 3]);
    case(bump_vec![in &bump0; 1, 1, 2, 2, 2, 3, 3], bump_vec![in &bump1; 1, 2, 3]);
}

#[test]
fn test_dedup_by_key() {
    fn case(a: BumpVec<i32, &Bump>, b: BumpVec<i32, &Bump>) {
        let mut v = a;
        v.dedup_by_key(|i| *i / 10);
        assert_eq!(v, b);
    }

    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    case(bump_vec![in &bump0; ], bump_vec![in &bump1; ]);
    case(bump_vec![in &bump0; 10], bump_vec![in &bump1; 10]);
    case(bump_vec![in &bump0; 10, 11], bump_vec![in &bump1; 10]);
    case(bump_vec![in &bump0; 10, 20, 30], bump_vec![in &bump1; 10, 20, 30]);
    case(bump_vec![in &bump0; 10, 11, 20, 30], bump_vec![in &bump1; 10, 20, 30]);
    case(bump_vec![in &bump0; 10, 20, 21, 30], bump_vec![in &bump1; 10, 20, 30]);
    case(bump_vec![in &bump0; 10, 20, 30, 31], bump_vec![in &bump1; 10, 20, 30]);
    case(
        bump_vec![in &bump0; 10, 11, 20, 21, 22, 30, 31],
        bump_vec![in &bump1; 10, 20, 30],
    );
}

#[test]
fn test_dedup_by() {
    let bump: Bump = Bump::new();

    let mut vec = bump_vec![in &bump; "foo", "bar", "Bar", "baz", "bar"];
    vec.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    assert_eq!(vec, ["foo", "bar", "baz", "bar"]);
    drop(vec);

    let mut vec = bump_vec![in &bump; ("foo", 1), ("foo", 2), ("bar", 3), ("bar", 4), ("bar", 5)];
    vec.dedup_by(|a, b| {
        a.0 == b.0 && {
            b.1 += a.1;
            true
        }
    });

    assert_eq!(vec, [("foo", 3), ("bar", 12)]);
}

#[test]
fn test_dedup_unique() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();
    let bump2: Bump = Bump::new();

    let mut v0: BumpVec<Box<_>, _> = bump_vec![in &bump0; Box::new(1), Box::new(1), Box::new(2), Box::new(3)];
    v0.dedup();
    let mut v1: BumpVec<Box<_>, _> = bump_vec![in &bump1; Box::new(1), Box::new(2), Box::new(2), Box::new(3)];
    v1.dedup();
    let mut v2: BumpVec<Box<_>, _> = bump_vec![in &bump2; Box::new(1), Box::new(2), Box::new(3), Box::new(3)];
    v2.dedup();
    // If the boxed pointers were leaked or otherwise misused, valgrind
    // and/or rt should raise errors.
}

#[test]
fn zero_sized_values() {
    let bump: Bump = Bump::new();

    let mut v = BumpVec::new_in(&bump);
    assert_eq!(v.len(), 0);
    v.push(());
    assert_eq!(v.len(), 1);
    v.push(());
    assert_eq!(v.len(), 2);
    assert_eq!(v.pop(), Some(()));
    assert_eq!(v.pop(), Some(()));
    assert_eq!(v.pop(), None);

    assert_eq!(v.iter().count(), 0);
    v.push(());
    assert_eq!(v.iter().count(), 1);
    v.push(());
    assert_eq!(v.iter().count(), 2);

    for &() in &v {}

    assert_eq!(v.iter_mut().count(), 2);
    v.push(());
    assert_eq!(v.iter_mut().count(), 3);
    v.push(());
    assert_eq!(v.iter_mut().count(), 4);

    for &mut () in &mut v {}
    unsafe {
        v.set_len(0);
    }
    assert_eq!(v.iter_mut().count(), 0);
}

#[test]
fn test_partition() {
    let bump: Bump = Bump::new();

    assert_eq!(
        bump.alloc_slice_copy(&[]).partition(|x: &i32| *x < 3),
        (bump.alloc_slice_copy(&[]), bump.alloc_slice_copy(&[]))
    );
    assert_eq!(
        bump.alloc_slice_copy(&[1, 2, 3]).partition(|x| *x < 4),
        (bump.alloc_slice_copy(&[1, 2, 3]), bump.alloc_slice_copy(&[]))
    );
    assert_eq!(
        bump.alloc_slice_copy(&[1, 2, 3]).partition(|x| *x < 2),
        (bump.alloc_slice_copy(&[1]), bump.alloc_slice_copy(&[2, 3]))
    );
    assert_eq!(
        bump.alloc_slice_copy(&[1, 2, 3]).partition(|x| *x < 0),
        (bump.alloc_slice_copy(&[]), bump.alloc_slice_copy(&[1, 2, 3]))
    );
}

#[test]
fn test_cmp() {
    let bump: Bump = Bump::new();

    let x: &[isize] = &[1, 2, 3, 4, 5];
    let cmp: &[isize] = &[1, 2, 3, 4, 5];
    assert_eq!(&x[..], cmp);
    let cmp: &[isize] = &[3, 4, 5];
    assert_eq!(&x[2..], cmp);
    let cmp: &[isize] = &[1, 2, 3];
    assert_eq!(&x[..3], cmp);
    let cmp: &[isize] = &[2, 3, 4];
    assert_eq!(&x[1..4], cmp);

    let x: BumpVec<isize, _> = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let cmp: &[isize] = &[1, 2, 3, 4, 5];
    assert_eq!(&x[..], cmp);
    let cmp: &[isize] = &[3, 4, 5];
    assert_eq!(&x[2..], cmp);
    let cmp: &[isize] = &[1, 2, 3];
    assert_eq!(&x[..3], cmp);
    let cmp: &[isize] = &[2, 3, 4];
    assert_eq!(&x[1..4], cmp);
}

#[test]
fn test_vec_truncate_drop() {
    static mut DROPS: u32 = 0;
    struct Elem(#[allow(dead_code)] i32);
    impl Drop for Elem {
        fn drop(&mut self) {
            unsafe {
                DROPS += 1;
            }
        }
    }

    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; Elem(1), Elem(2), Elem(3), Elem(4), Elem(5)];
    assert_eq!(unsafe { DROPS }, 0);
    v.truncate(3);
    assert_eq!(unsafe { DROPS }, 2);
    v.truncate(0);
    assert_eq!(unsafe { DROPS }, 5);
}

#[test]
#[should_panic]
fn test_vec_truncate_fail() {
    struct BadElem(i32);
    impl Drop for BadElem {
        fn drop(&mut self) {
            let BadElem(ref mut x) = *self;
            if *x == 0xbadbeef {
                panic!("BadElem panic: 0xbadbeef")
            }
        }
    }

    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; BadElem(1), BadElem(2), BadElem(0xbadbeef), BadElem(4)];
    v.truncate(0);
}

#[test]
fn test_index() {
    let bump: Bump = Bump::new();
    let vec = bump_vec![in &bump; 1, 2, 3];
    assert!(vec[1] == 2);
}

#[test]
#[should_panic]
fn test_index_out_of_bounds() {
    let bump: Bump = Bump::new();
    let vec = bump_vec![in &bump; 1, 2, 3];
    let _ = vec[3];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_1() {
    let bump: Bump = Bump::new();
    let x = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let _ = &x[!0..];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_2() {
    let bump: Bump = Bump::new();
    let x = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let _ = &x[..6];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_3() {
    let bump: Bump = Bump::new();
    let x = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let _ = &x[!0..4];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_4() {
    let bump: Bump = Bump::new();
    let x = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let _ = &x[1..6];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_5() {
    let bump: Bump = Bump::new();
    let x = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let _ = &x[3..2];
}

#[test]
#[should_panic]
fn test_swap_remove_empty() {
    let bump: Bump = Bump::new();
    let mut vec = BumpVec::<i32, _>::new_in(&bump);
    vec.swap_remove(0);
}

#[test]
fn test_move_items() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();
    let vec = bump_vec![in &bump0; 1, 2, 3];
    let mut vec2 = bump_vec![in &bump1; ];
    for i in vec {
        vec2.push(i);
    }
    assert_eq!(vec2, [1, 2, 3]);
}

#[test]
fn test_move_items_reverse() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();
    let vec = bump_vec![in &bump0; 1, 2, 3];
    let mut vec2 = bump_vec![in &bump1; ];
    for i in vec.into_iter().rev() {
        vec2.push(i);
    }
    assert_eq!(vec2, [3, 2, 1]);
}

#[test]
fn test_move_items_zero_sized() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();
    let vec = bump_vec![in &bump0; (), (), ()];
    let mut vec2 = bump_vec![in &bump1; ];
    for i in vec {
        vec2.push(i);
    }
    assert_eq!(vec2, [(), (), ()]);
}

#[test]
fn test_drain_empty_vec() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    let mut vec: BumpVec<i32, _> = bump_vec![in &bump0; ];
    let mut vec2: BumpVec<i32, _> = bump_vec![in &bump1; ];
    for i in vec.drain(..) {
        vec2.push(i);
    }
    assert!(vec.is_empty());
    assert!(vec2.is_empty());
}

#[test]
fn test_drain_items() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    let mut vec = bump_vec![in &bump0; 1, 2, 3];
    let mut vec2 = bump_vec![in &bump1; ];
    for i in vec.drain(..) {
        vec2.push(i);
    }
    assert!(vec.is_empty());
    assert_eq!(vec2, [1, 2, 3]);
}

#[test]
fn test_drain_items_reverse() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    let mut vec = bump_vec![in &bump0; 1, 2, 3];
    let mut vec2 = bump_vec![in &bump1; ];
    for i in vec.drain(..).rev() {
        vec2.push(i);
    }
    assert!(vec.is_empty());
    assert_eq!(vec2, [3, 2, 1]);
}

#[test]
fn test_drain_items_zero_sized() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    let mut vec = bump_vec![in &bump0; (), (), ()];
    let mut vec2 = bump_vec![in &bump1; ];
    for i in vec.drain(..) {
        vec2.push(i);
    }
    assert_eq!(vec, []);
    assert_eq!(vec2, [(), (), ()]);
}

#[test]
#[should_panic]
fn test_drain_out_of_bounds() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 1, 2, 3, 4, 5];
    v.drain(5..6);
}

#[test]
fn test_drain_range() {
    let bump: Bump = Bump::new();

    let mut v = bump_vec![in &bump; 1, 2, 3, 4, 5];
    for _ in v.drain(4..) {}
    assert_eq!(v.as_slice(), &[1, 2, 3, 4]);
    drop(v);

    let mut v = bump.alloc_iter((1..6).map(|x| x.to_string()));
    for _ in v.drain(1..4) {}
    assert_eq!(v.as_slice(), &[1.to_string(), 5.to_string()]);
    drop(v);

    let mut v = bump.alloc_iter((1..6).map(|x| x.to_string()));
    for _ in v.drain(1..4).rev() {}
    assert_eq!(v.as_slice(), &[1.to_string(), 5.to_string()]);
    drop(v);

    let mut v: BumpVec<_, _> = bump_vec![in &bump; (); 5];
    for _ in v.drain(1..4).rev() {}
    assert_eq!(v, &[(), ()]);
    drop(v);
}

#[test]
fn test_drain_inclusive_range() {
    let bump: Bump = Bump::new();

    let mut v = bump_vec![in &bump; 'a', 'b', 'c', 'd', 'e'];
    for _ in v.drain(1..=3) {}
    assert_eq!(v, &['a', 'e']);
    drop(v);

    let mut v = bump.alloc_iter((0..=5).map(|x| x.to_string()));
    for _ in v.drain(1..=5) {}
    assert_eq!(v.as_slice(), &["0".to_string()]);
    drop(v);

    let mut v = bump.alloc_iter((0..=5).map(|x| x.to_string()));
    for _ in v.drain(0..=5) {}
    assert!(v.as_slice().is_empty());
    drop(v);

    let mut v = bump.alloc_iter((0..=5).map(|x| x.to_string()));
    for _ in v.drain(0..=3) {}
    assert_eq!(v.as_slice(), &["4".to_string(), "5".to_string()]);
    drop(v);

    let mut v = bump.alloc_iter((0..=1).map(|x| x.to_string()));
    for _ in v.drain(..=0) {}
    assert_eq!(v.as_slice(), &["1".to_string()]);
    drop(v);
}

#[test]
fn test_drain_max_vec_size() {
    let bump: Bump = Bump::new();

    let mut v = BumpVec::<(), _>::with_capacity_in(usize::MAX, &bump);
    unsafe {
        v.set_len(usize::MAX);
    }
    for _ in v.drain(usize::MAX - 1..) {}
    assert_eq!(v.len(), usize::MAX - 1);
    drop(v);

    let mut v = BumpVec::<(), _>::with_capacity_in(usize::MAX, &bump);
    unsafe {
        v.set_len(usize::MAX);
    }
    for _ in v.drain(usize::MAX - 1..=usize::MAX - 1) {}
    assert_eq!(v.len(), usize::MAX - 1);
    drop(v);
}

#[test]
#[should_panic]
fn test_drain_index_overflow() {
    let bump: Bump = Bump::new();
    let mut v = BumpVec::<(), _>::with_capacity_in(usize::MAX, &bump);
    unsafe {
        v.set_len(usize::MAX);
    }
    v.drain(0..=usize::MAX);
}

#[test]
#[should_panic]
fn test_drain_inclusive_out_of_bounds() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 1, 2, 3, 4, 5];
    v.drain(5..=5);
}

#[test]
#[should_panic]
fn test_drain_start_overflow() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 1, 2, 3];
    v.drain((Excluded(usize::MAX), Included(0)));
}

#[test]
#[should_panic]
fn test_drain_end_overflow() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 1, 2, 3];
    v.drain((Included(0), Included(usize::MAX)));
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_drain_leak() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    static mut DROPS: i32 = 0;

    #[derive(Debug, PartialEq)]
    struct D(u32, bool);

    impl Drop for D {
        fn drop(&mut self) {
            unsafe {
                DROPS += 1;
                dbg!(self.0);
                dbg!(DROPS);
            }

            if self.1 {
                panic!("panic in `drop`");
            }
        }
    }

    let mut v = bump_vec![in &bump0;
        D(0, false),
        D(1, false),
        D(2, false),
        D(3, false),
        D(4, true),
        D(5, false),
        D(6, false),
    ];

    catch_unwind(AssertUnwindSafe(|| {
        v.drain(2..=5);
    }))
    .ok();

    assert_eq!(unsafe { DROPS }, 4);
    assert_eq!(v, bump_vec![in &bump1; D(0, false), D(1, false), D(6, false),]);
}

#[test]
fn test_drain_keep_rest() {
    let bump: Bump = Bump::new();

    let mut v = bump_vec![in &bump; 0, 1, 2, 3, 4, 5, 6];
    let mut drain = v.drain(1..6);
    assert_eq!(drain.next(), Some(1));
    assert_eq!(drain.next_back(), Some(5));
    assert_eq!(drain.next(), Some(2));

    drain.keep_rest();
    assert_eq!(v, &[0, 3, 4, 6]);
}

#[test]
fn test_drain_keep_rest_all() {
    let bump: Bump = Bump::new();

    let mut v = bump_vec![in &bump; 0, 1, 2, 3, 4, 5, 6];
    v.drain(1..6).keep_rest();
    assert_eq!(v, &[0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_drain_keep_rest_none() {
    let bump: Bump = Bump::new();

    let mut v = bump_vec![in &bump; 0, 1, 2, 3, 4, 5, 6];
    let mut drain = v.drain(1..6);

    drain.by_ref().for_each(drop);

    drain.keep_rest();
    assert_eq!(v, &[0, 6]);
}

#[test]
fn test_splice() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let a = [10, 11, 12];
    v.splice(2..4, a);
    assert_eq!(v, &[1, 2, 10, 11, 12, 5]);
    v.splice(1..3, Some(20));
    assert_eq!(v, &[1, 20, 11, 12, 5]);
}

#[test]
fn test_splice_inclusive_range() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let a = [10, 11, 12];
    let t1: Vec<_> = v.splice(2..=3, a).collect();
    assert_eq!(v, &[1, 2, 10, 11, 12, 5]);
    assert_eq!(t1, &[3, 4]);
    let t2: Vec<_> = v.splice(1..=2, Some(20)).collect();
    assert_eq!(v, &[1, 20, 11, 12, 5]);
    assert_eq!(t2, &[2, 10]);
}

#[test]
#[should_panic]
fn test_splice_out_of_bounds() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let a = [10, 11, 12];
    v.splice(5..6, a);
}

#[test]
#[should_panic]
fn test_splice_inclusive_out_of_bounds() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let a = [10, 11, 12];
    v.splice(5..=5, a);
}

#[test]
fn test_splice_items_zero_sized() {
    let bump: Bump = Bump::new();
    let mut vec = bump_vec![in &bump; (), (), ()];
    let vec2 = bump_vec![in &bump; ];
    let t: Vec<_> = vec.splice(1..2, vec2.iter().cloned()).collect();
    assert_eq!(vec, &[(), ()]);
    assert_eq!(t, &[()]);
}

#[test]
fn test_splice_unbounded() {
    let bump: Bump = Bump::new();
    let mut vec = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let t: Vec<_> = vec.splice(.., None).collect();
    assert!(vec.is_empty());
    assert_eq!(t, &[1, 2, 3, 4, 5]);
}

#[test]
fn test_splice_forget() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 1, 2, 3, 4, 5];
    let a = [10, 11, 12];
    std::mem::forget(v.splice(2..4, a));
    assert_eq!(v, &[1, 2]);
}

#[test]
fn test_into_boxed_slice() {
    let bump: Bump = Bump::new();
    let xs = bump_vec![in &bump; 1, 2, 3];
    let ys = xs.into_boxed_slice();
    assert_eq!(&*ys, [1, 2, 3]);
}

#[test]
fn test_append() {
    let bump: Bump = Bump::new();
    let bump = bump.as_scope();
    let mut vec = bump_vec![in &bump; 1, 2, 3];
    let mut vec2 = bump_vec![in &bump; 4, 5, 6];
    vec.append(&mut vec2);
    assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
    assert!(vec2.is_empty());
}

#[test]
fn test_split_off() {
    let bump: Bump = Bump::new();
    let mut vec = bump_vec![in &bump; 1, 2, 3, 4, 5, 6];
    let orig_ptr = vec.as_ptr();
    let orig_capacity = vec.capacity();

    let split_off = vec.split_off(4..);
    assert_eq!(vec, [1, 2, 3, 4]);
    assert_eq!(split_off, [5, 6]);
    assert_eq!(vec.capacity(), vec.len());
    assert_eq!(vec.as_ptr(), orig_ptr);
    assert_eq!(split_off.capacity(), orig_capacity - vec.capacity());
}

#[test]
fn test_split_off_take_all() {
    let bump: Bump = Bump::new();

    // Allocate enough capacity that we can tell whether the split-off vector's
    // capacity is based on its size, or on the original capacity.
    let mut vec = BumpVec::with_capacity_in(1000, &bump);
    vec.extend([1, 2, 3, 4, 5, 6]);
    let orig_ptr = vec.as_ptr();
    let orig_capacity = vec.capacity();

    let split_off = vec.split_off(0..);
    assert!(vec.is_empty());
    assert_eq!(split_off, [1, 2, 3, 4, 5, 6]);
    assert_eq!(vec.capacity(), vec.len());
    assert_eq!(vec.as_ptr(), orig_ptr);
    assert_eq!(split_off.as_ptr(), orig_ptr);
    assert_eq!(split_off.capacity(), orig_capacity - vec.capacity());
}

#[test]
fn test_into_iter_as_slice() {
    let bump: Bump = Bump::new();
    let vec = bump_vec![in &bump; 'a', 'b', 'c'];
    let mut into_iter = vec.into_iter();
    assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    let _ = into_iter.next().unwrap();
    assert_eq!(into_iter.as_slice(), &['b', 'c']);
    let _ = into_iter.next().unwrap();
    let _ = into_iter.next().unwrap();
    assert_eq!(into_iter.as_slice(), &[]);
}

#[test]
fn test_into_iter_as_mut_slice() {
    let bump: Bump = Bump::new();
    let vec = bump_vec![in &bump; 'a', 'b', 'c'];
    let mut into_iter = vec.into_iter();
    assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    into_iter.as_mut_slice()[0] = 'x';
    into_iter.as_mut_slice()[1] = 'y';
    assert_eq!(into_iter.next().unwrap(), 'x');
    assert_eq!(into_iter.as_slice(), &['y', 'c']);
}

#[test]
fn test_into_iter_debug() {
    let bump: Bump = Bump::new();
    let vec = bump_vec![in &bump; 'a', 'b', 'c'];
    let into_iter = vec.into_iter();
    let debug = format!("{into_iter:?}");
    assert_eq!(debug, "IntoIter(['a', 'b', 'c'])");
}

#[test]
fn test_into_iter_count() {
    let bump: Bump = Bump::new();
    let v = BumpVec::from_array_in([1, 2, 3], &bump);
    assert_eq!(v.into_iter().count(), 3);
}

#[test]
fn test_into_iter_next_chunk() {
    let bump: Bump = Bump::new();
    let mut iter = BumpVec::from_array_in(*b"lorem", &bump).into_iter();

    assert_eq!(iter.next_chunk().unwrap(), [b'l', b'o']); // N is inferred as 2
    assert_eq!(iter.next_chunk().unwrap(), [b'r', b'e', b'm']); // N is inferred as 3
    assert!(iter.next_chunk::<4>().unwrap_err().as_slice().is_empty()); // N is explicitly 4
}

#[test]
fn test_into_iter_clone() {
    fn iter_equal<I: Iterator<Item = i32>>(it: I, slice: &[i32]) {
        let bump: Bump = Bump::new();
        let mut v = bump_vec![in &bump];
        v.extend(it);
        assert_eq!(&v[..], slice);
    }
    let mut it = [1, 2, 3].into_iter();
    iter_equal(it.clone(), &[1, 2, 3]);
    assert_eq!(it.next(), Some(1));
    let mut it = it.rev();
    iter_equal(it.clone(), &[3, 2]);
    assert_eq!(it.next(), Some(3));
    iter_equal(it.clone(), &[2]);
    assert_eq!(it.next(), Some(2));
    iter_equal(it.clone(), &[]);
    assert_eq!(it.next(), None);
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_into_iter_leak() {
    static mut DROPS: i32 = 0;

    struct D(bool);

    impl Drop for D {
        fn drop(&mut self) {
            unsafe {
                DROPS += 1;
            }

            if self.0 {
                panic!("panic in `drop`");
            }
        }
    }

    let bump: Bump = Bump::new();
    let v = bump_vec![in &bump; D(false), D(true), D(false)];

    catch_unwind(move || drop(v.into_iter())).ok();

    assert_eq!(unsafe { DROPS }, 3);
}

#[test]
fn test_into_iter_advance_by() {
    let bump: Bump = Bump::new();
    let mut i = bump_vec![in &bump; 1, 2, 3, 4, 5].into_iter();
    assert_eq!(i.advance_by(0), Ok(()));
    assert_eq!(i.advance_back_by(0), Ok(()));
    assert_eq!(i.as_slice(), [1, 2, 3, 4, 5]);

    assert_eq!(i.advance_by(1), Ok(()));
    assert_eq!(i.advance_back_by(1), Ok(()));
    assert_eq!(i.as_slice(), [2, 3, 4]);

    assert_eq!(
        i.advance_back_by(usize::MAX),
        Err(NonZero::<usize>::new(usize::MAX - 3).unwrap())
    );

    assert_eq!(i.advance_by(usize::MAX), Err(NonZero::<usize>::new(usize::MAX).unwrap()));

    assert_eq!(i.advance_by(0), Ok(()));
    assert_eq!(i.advance_back_by(0), Ok(()));

    assert_eq!(i.len(), 0);
}

#[test]
fn test_into_iter_drop_allocator() {
    #[allow(dead_code)]
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
    let bump = Bump::<_, 1, true>::new_in(allocator.clone());
    drop(bump);
    assert_eq!(drop_count.get(), 1);

    let bump = Bump::<_, 1, true>::new_in(allocator);
    bump.reserve_bytes(1024);
    drop(bump);
    assert_eq!(drop_count.get(), 3);
}

#[test]
fn test_into_iter_zst() {
    #[derive(Debug, Clone)]
    struct AlignedZstWithDrop([u64; 0]);
    impl Drop for AlignedZstWithDrop {
        fn drop(&mut self) {
            let addr = self as *mut _ as usize;
            assert!(hint::black_box(addr) % mem::align_of::<u64>() == 0);
        }
    }

    const C: AlignedZstWithDrop = AlignedZstWithDrop([0u64; 0]);

    let bump: Bump = Bump::new();
    for _ in bump_vec![in &bump; C].into_iter() {}
    for _ in bump_vec![in &bump; C; 5].into_iter().rev() {}

    let mut it = bump_vec![in &bump; C, C].into_iter();
    assert_eq!(it.advance_by(1), Ok(()));
    drop(it);

    let mut it = bump_vec![in &bump; C, C].into_iter();
    it.next_chunk::<1>().unwrap();
    drop(it);

    let mut it = bump_vec![in &bump; C, C].into_iter();
    it.next_chunk::<4>().unwrap_err();
    drop(it);
}

#[allow(dead_code)]
fn assert_covariance() {
    fn drain<'new>(d: Drain<'static, &'static str>) -> Drain<'new, &'new str> {
        d
    }
    fn into_iter<'new>(i: IntoIter<&'static str>) -> IntoIter<&'new str> {
        i
    }
}

#[test]
fn from_into_inner() {
    let bump: Bump = Bump::new();
    let vec = bump.alloc_slice_copy(&[1, 2, 3]);
    let ptr = vec.as_ptr();
    let vec = vec.into_iter().into_boxed_slice();
    assert_eq!(vec.as_slice(), [1, 2, 3]);
    assert_eq!(vec.as_ptr(), ptr);

    let ptr = &vec[1] as *const _;
    let mut it = vec.into_iter();
    it.next().unwrap();
    let vec = it.into_boxed_slice();
    assert_eq!(vec.as_slice(), &[2, 3]);
    assert!(ptr == vec.as_ptr());
}

#[test]
fn overaligned_allocations() {
    let bump: Bump = Bump::new();

    #[repr(align(256))]
    struct Foo(usize);
    let mut v = bump_vec![in &bump; Foo(273)];
    for i in 0..0x1000 {
        v.reserve_exact(i);
        assert!(v[0].0 == 273);
        assert!(v.as_ptr() as usize & 0xff == 0);
        v.shrink_to_fit();
        assert!(v[0].0 == 273);
        assert!(v.as_ptr() as usize & 0xff == 0);
    }
}

// TODO: extract_if ranges?

#[test]
fn extract_if_empty() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();
    let mut vec: BumpVec<i32, _> = bump_vec![in &bump0; ];

    {
        let mut iter = vec.extract_if(|_| true);
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }
    assert_eq!(vec.len(), 0);

    let empty: BumpVec<i32, _> = bump_vec![in &bump1; ];
    assert_eq!(vec, empty);
}

#[test]
fn extract_if_zst() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();
    let mut vec = bump_vec![in &bump0; (), (), (), (), ()];
    let initial_len = vec.len();
    let mut count = 0;
    {
        let mut iter = vec.extract_if(|_| true);
        assert_eq!(iter.size_hint(), (0, Some(initial_len)));
        while let Some(_) = iter.next() {
            count += 1;
            assert_eq!(iter.size_hint(), (0, Some(initial_len - count)));
        }
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    assert_eq!(count, initial_len);
    assert_eq!(vec.len(), 0);
    assert_eq!(vec, bump_vec![in &bump1; ]);
}

#[test]
fn extract_if_false() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();
    let mut vec = bump_vec![in &bump0; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let initial_len = vec.len();
    let mut count = 0;
    {
        let mut iter = vec.extract_if(|_| false);
        assert_eq!(iter.size_hint(), (0, Some(initial_len)));
        for _ in iter.by_ref() {
            count += 1;
        }
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    assert_eq!(count, 0);
    assert_eq!(vec.len(), initial_len);
    assert_eq!(vec, bump_vec![in &bump1; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn extract_if_true() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();
    let mut vec = bump_vec![in &bump0; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let initial_len = vec.len();
    let mut count = 0;
    {
        let mut iter = vec.extract_if(|_| true);
        assert_eq!(iter.size_hint(), (0, Some(initial_len)));
        while let Some(_) = iter.next() {
            count += 1;
            assert_eq!(iter.size_hint(), (0, Some(initial_len - count)));
        }
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    assert_eq!(count, initial_len);
    assert_eq!(vec.len(), 0);

    let empty: BumpVec<i32, _> = bump_vec![in &bump1; ];
    assert_eq!(vec, empty);
}

#[test]
fn extract_if_complex() {
    let bump: Bump = Bump::new();

    {
        //                [+xxx++++++xxxxx++++x+x++]
        let mut vec = bump_vec![in &bump;
            1, 2, 4, 6, 7, 9, 11, 13, 15, 17, 18, 20, 22, 24, 26, 27, 29, 31, 33, 34, 35, 36, 37, 39,
        ];

        let removed = vec.extract_if(|x| *x % 2 == 0).collect::<Vec<_>>();
        assert_eq!(removed.len(), 10);
        assert_eq!(removed, &[2, 4, 6, 18, 20, 22, 24, 26, 34, 36]);

        assert_eq!(vec.len(), 14);
        assert_eq!(vec, &[1, 7, 9, 11, 13, 15, 17, 27, 29, 31, 33, 35, 37, 39]);
    }

    {
        //                [xxx++++++xxxxx++++x+x++]
        let mut vec = bump_vec![in &bump;
            2, 4, 6, 7, 9, 11, 13, 15, 17, 18, 20, 22, 24, 26, 27, 29, 31, 33, 34, 35, 36, 37, 39,
        ];

        let removed = vec.extract_if(|x| *x % 2 == 0).collect::<Vec<_>>();
        assert_eq!(removed.len(), 10);
        assert_eq!(removed, &[2, 4, 6, 18, 20, 22, 24, 26, 34, 36]);

        assert_eq!(vec.len(), 13);
        assert_eq!(vec, &[7, 9, 11, 13, 15, 17, 27, 29, 31, 33, 35, 37, 39]);
    }

    {
        //                [xxx++++++xxxxx++++x+x]
        let mut vec = bump_vec![in &bump; 2, 4, 6, 7, 9, 11, 13, 15, 17, 18, 20, 22, 24, 26, 27, 29, 31, 33, 34, 35, 36];

        let removed = vec.extract_if(|x| *x % 2 == 0).collect::<Vec<_>>();
        assert_eq!(removed.len(), 10);
        assert_eq!(removed, &[2, 4, 6, 18, 20, 22, 24, 26, 34, 36]);

        assert_eq!(vec.len(), 11);
        assert_eq!(vec, &[7, 9, 11, 13, 15, 17, 27, 29, 31, 33, 35]);
    }

    {
        //                [xxxxxxxxxx+++++++++++]
        let mut vec = bump_vec![in &bump; 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19];

        let removed = vec.extract_if(|x| *x % 2 == 0).collect::<Vec<_>>();
        assert_eq!(removed.len(), 10);
        assert_eq!(removed, &[2, 4, 6, 8, 10, 12, 14, 16, 18, 20]);

        assert_eq!(vec.len(), 10);
        assert_eq!(vec, &[1, 3, 5, 7, 9, 11, 13, 15, 17, 19]);
    }

    {
        //                [+++++++++++xxxxxxxxxx]
        let mut vec = bump_vec![in &bump; 1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20];

        let removed = vec.extract_if(|x| *x % 2 == 0).collect::<Vec<_>>();
        assert_eq!(removed.len(), 10);
        assert_eq!(removed, &[2, 4, 6, 8, 10, 12, 14, 16, 18, 20]);

        assert_eq!(vec.len(), 10);
        assert_eq!(vec, &[1, 3, 5, 7, 9, 11, 13, 15, 17, 19]);
    }
}

#[test]
#[cfg(not(target_os = "emscripten"))]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn extract_if_consumed_panic() {
    use std::{rc::Rc, sync::Mutex};

    struct Check<'a> {
        index: usize,
        drop_counts: Rc<Mutex<BumpVec<usize, &'a Bump>>>,
    }

    impl Drop for Check<'_> {
        fn drop(&mut self) {
            self.drop_counts.lock().unwrap()[self.index] += 1;
            println!("drop: {}", self.index);
        }
    }

    let bump: Bump = Bump::new();
    let check_count = 10;
    let drop_counts = Rc::new(Mutex::new(bump_vec![in &bump; 0_usize; check_count]));
    let mut data: BumpVec<Check, _> = bump_vec![in &bump];
    data.extend((0..check_count).map(|index| Check {
        index,
        drop_counts: Rc::clone(&drop_counts),
    }));

    let _ = std::panic::catch_unwind(move || {
        let filter = |c: &mut Check| {
            if c.index == 2 {
                panic!("panic at index: {}", c.index);
            }
            // Verify that if the filter could panic again on another element
            // that it would not cause a double panic and all elements of the
            // vec would still be dropped exactly once.
            if c.index == 4 {
                panic!("panic at index: {}", c.index);
            }
            c.index < 6
        };
        let drain = data.extract_if(filter);

        // NOTE: The ExtractIf is explicitly consumed
        drain.for_each(drop);
    });

    let drop_counts = drop_counts.lock().unwrap();
    assert_eq!(check_count, drop_counts.len());

    for (index, count) in drop_counts.iter().cloned().enumerate() {
        assert_eq!(1, count, "unexpected drop count at index: {} (count: {})", index, count);
    }
}

#[test]
#[cfg(not(target_os = "emscripten"))]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn extract_if_unconsumed_panic() {
    use std::{rc::Rc, sync::Mutex};

    struct Check<'a> {
        index: usize,
        drop_counts: Rc<Mutex<BumpVec<usize, &'a Bump>>>,
    }

    impl Drop for Check<'_> {
        fn drop(&mut self) {
            self.drop_counts.lock().unwrap()[self.index] += 1;
            println!("drop: {}", self.index);
        }
    }

    let bump: Bump = Bump::new();
    let check_count = 10;
    let drop_counts = Rc::new(Mutex::new(bump_vec![in &bump; 0_usize; check_count]));
    let mut data: BumpVec<Check, _> = bump_vec![in &bump];
    data.extend((0..check_count).map(|index| Check {
        index,
        drop_counts: Rc::clone(&drop_counts),
    }));

    let _ = std::panic::catch_unwind(move || {
        let filter = |c: &mut Check| {
            if c.index == 2 {
                panic!("panic at index: {}", c.index);
            }
            // Verify that if the filter could panic again on another element
            // that it would not cause a double panic and all elements of the
            // vec would still be dropped exactly once.
            if c.index == 4 {
                panic!("panic at index: {}", c.index);
            }
            c.index < 6
        };
        let _drain = data.extract_if(filter);

        // NOTE: The ExtractIf is dropped without being consumed
    });

    let drop_counts = drop_counts.lock().unwrap();
    assert_eq!(check_count, drop_counts.len());

    for (index, count) in drop_counts.iter().cloned().enumerate() {
        assert_eq!(1, count, "unexpected drop count at index: {} (count: {})", index, count);
    }
}

#[test]
fn extract_if_unconsumed() {
    let bump: Bump = Bump::new();
    let mut vec = bump_vec![in &bump; 1, 2, 3, 4];
    let drain = vec.extract_if(|&mut x| x % 2 != 0);
    drop(drain);
    assert_eq!(vec, [1, 2, 3, 4]);
}

#[test]
fn test_stable_pointers() {
    /// Pull an element from the iterator, then drop it.
    /// Useful to cover both the `next` and `drop` paths of an iterator.
    fn next_then_drop<I: Iterator>(mut i: I) {
        i.next().unwrap();
        drop(i);
    }

    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    // Test that, if we reserved enough space, adding and removing elements does not
    // invalidate references into the vector (such as `v0`). This test also
    // runs in Miri, which would detect such problems.
    // Note that this test does *not* constitute a stable guarantee that all these functions do not
    // reallocate! Only what is explicitly documented at
    // <https://doc.rust-lang.org/nightly/std/vec/struct.BumpVec.html#guarantees> is stably guaranteed.
    let mut v = BumpVec::with_capacity_in(128, &bump0);
    v.push(13);

    // Laundering the lifetime -- we take care that `v` does not reallocate, so that's okay.
    let v0 = &mut v[0];
    let v0 = unsafe { &mut *(v0 as *mut _) };
    // Now do a bunch of things and occasionally use `v0` again to assert it is still valid.

    // Pushing/inserting and popping/removing
    v.push(1);
    v.push(2);
    v.insert(1, 1);
    assert_eq!(*v0, 13);
    v.remove(1);
    v.pop().unwrap();
    assert_eq!(*v0, 13);
    v.push(1);
    v.swap_remove(1);
    assert_eq!(v.len(), 2);
    v.swap_remove(1); // swap_remove the last element
    assert_eq!(*v0, 13);

    // Appending
    v.append(&mut bump_vec![in &bump1; 27, 19].into_boxed_slice());
    assert_eq!(*v0, 13);

    // Extending
    v.extend_from_slice_copy(&[1, 2]);
    v.extend(&[1, 2]); // `slice::Iter` (with `T: Copy`) specialization
    v.extend(bump_vec![in &bump1; 2, 3]); // `vec::IntoIter` specialization
    v.extend(std::iter::once(3)); // `TrustedLen` specialization
    v.extend(std::iter::empty::<i32>()); // `TrustedLen` specialization with empty iterator
    v.extend(std::iter::once(3).filter(|_| true)); // base case
    v.extend(std::iter::once(&3)); // `cloned` specialization
    assert_eq!(*v0, 13);

    // Truncation
    v.truncate(2);
    assert_eq!(*v0, 13);

    // Resizing
    v.resize_with(v.len() + 10, || 42);
    assert_eq!(*v0, 13);
    v.resize_with(2, || panic!());
    assert_eq!(*v0, 13);

    // No-op reservation
    v.reserve(32);
    v.reserve_exact(32);
    assert_eq!(*v0, 13);

    // Partial draining
    v.resize_with(10, || 42);
    next_then_drop(v.drain(5..));
    assert_eq!(*v0, 13);

    // Splicing
    v.resize_with(10, || 42);
    next_then_drop(v.splice(5.., bump_vec![in &bump1; 1, 2, 3, 4, 5])); // empty tail after range
    assert_eq!(*v0, 13);
    next_then_drop(v.splice(5..8, bump_vec![in &bump1; 1])); // replacement is smaller than original range
    assert_eq!(*v0, 13);
    next_then_drop(v.splice(5..6, [1; 10].into_iter().filter(|_| true))); // lower bound not exact
    assert_eq!(*v0, 13);

    // spare_capacity_mut
    v.spare_capacity_mut();
    assert_eq!(*v0, 13);

    // Smoke test that would fire even outside Miri if an actual relocation happened.
    // Also ensures the pointer is still writeable after all this.
    *v0 -= 13;
    assert_eq!(v[0], 0);
}

macro_rules! generate_assert_eq_vec_and_prim {
    ($name:ident<$B:ident>($type:ty)) => {
        fn $name<A: PartialEq<$B> + Debug, $B: Debug>(a: BumpVec<A, &Bump>, b: $type) {
            assert!(a == b);
            assert_eq!(a, b);
        }
    };
}

generate_assert_eq_vec_and_prim! { assert_eq_vec_and_slice  <B>(&[B])   }
generate_assert_eq_vec_and_prim! { assert_eq_vec_and_array_3<B>([B; 3]) }

#[test]
fn partialeq_vec_and_prim() {
    let bump: Bump = Bump::new();
    assert_eq_vec_and_slice(bump_vec![in &bump; 1, 2, 3], &[1, 2, 3]);
    assert_eq_vec_and_array_3(bump_vec![in &bump; 1, 2, 3], [1, 2, 3]);
}

macro_rules! assert_partial_eq_valid {
    ($a2:expr, $a3:expr; $b2:expr, $b3: expr) => {
        assert!($a2 == $b2);
        assert!($a2 != $b3);
        assert!($a3 != $b2);
        assert!($a3 == $b3);
        assert_eq!($a2, $b2);
        assert_ne!($a2, $b3);
        assert_ne!($a3, $b2);
        assert_eq!($a3, $b3);
    };
}

#[test]
fn partialeq_vec_full() {
    let bump2: Bump = Bump::new();
    let bump3: Bump = Bump::new();
    let vec2: BumpVec<_, _> = bump_vec![in &bump2; 1, 2];
    let vec3: BumpVec<_, _> = bump_vec![in &bump3; 1, 2, 3];
    let slice2: &[_] = &[1, 2];
    let slice3: &[_] = &[1, 2, 3];
    let slicemut2: &[_] = &mut [1, 2];
    let slicemut3: &[_] = &mut [1, 2, 3];
    let array2: [_; 2] = [1, 2];
    let array3: [_; 3] = [1, 2, 3];
    let arrayref2: &[_; 2] = &[1, 2];
    let arrayref3: &[_; 3] = &[1, 2, 3];

    assert_partial_eq_valid!(vec2,vec3; vec2,vec3);
    assert_partial_eq_valid!(vec2,vec3; slice2,slice3);
    assert_partial_eq_valid!(vec2,vec3; slicemut2,slicemut3);
    assert_partial_eq_valid!(slice2,slice3; vec2,vec3);
    assert_partial_eq_valid!(slicemut2,slicemut3; vec2,vec3);
    assert_partial_eq_valid!(vec2,vec3; array2,array3);
    assert_partial_eq_valid!(vec2,vec3; arrayref2,arrayref3);
    assert_partial_eq_valid!(vec2,vec3; arrayref2[..],arrayref3[..]);
}

#[test]
fn test_zero_sized_capacity() {
    for len in [0, 1, 2, 4, 8, 16, 32, 64, 128, 256] {
        let bump: Bump = Bump::new();
        let v = BumpVec::<(), _>::with_capacity_in(len, &bump);
        assert_eq!(v.len(), 0);
        assert_eq!(v.capacity(), usize::MAX);
    }
}

#[test]
fn test_zero_sized_vec_push() {
    const N: usize = 8;

    for len in 0..N {
        let bump: Bump = Bump::new();
        let mut tester = BumpVec::with_capacity_in(len, &bump);
        assert_eq!(tester.len(), 0);
        assert!(tester.capacity() >= len);
        for _ in 0..len {
            tester.push(());
        }
        assert_eq!(tester.len(), len);
        assert_eq!(tester.iter().count(), len);
        tester.clear();
    }
}

#[test]
fn test_vec_macro_repeat() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    assert_eq!(bump_vec![in &bump0; 1; 3], bump_vec![in &bump1; 1, 1, 1]);
    assert_eq!(bump_vec![in &bump0; 1; 2], bump_vec![in &bump1; 1, 1]);
    assert_eq!(bump_vec![in &bump0; 1; 1], bump_vec![in &bump1; 1]);
    // assert_eq!(bump_vec![in &bump0; 1; 0], bump_vec![in &bump1; ]);

    // from_elem syntax (see RFC 832)
    let el = Box::new(1);
    let n = 3;
    assert_eq!(
        bump_vec![in &bump0; el; n],
        bump_vec![in &bump1; Box::new(1), Box::new(1), Box::new(1)]
    );
}

#[test]
fn test_vec_swap() {
    let bump: Bump = Bump::new();
    let mut a: BumpVec<isize, _> = bump_vec![in &bump; 0, 1, 2, 3, 4, 5, 6];
    a.swap(2, 4);
    assert_eq!(a[2], 4);
    assert_eq!(a[4], 2);
    let mut n = 42;
    swap(&mut n, &mut a[0]);
    assert_eq!(a[0], 42);
    assert_eq!(n, 0);
}

#[test]
fn test_extend_from_within_spec() {
    #[derive(Copy)]
    struct CopyOnly;

    impl Clone for CopyOnly {
        fn clone(&self) -> Self {
            panic!("extend_from_within must use specialization on copy");
        }
    }

    let bump: Bump = Bump::new();
    bump_vec![in &bump; CopyOnly, CopyOnly].extend_from_within_copy(..);
}

#[test]
fn test_extend_from_within_clone() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; String::from("sssss"), String::from("12334567890"), String::from("c")];
    v.extend_from_within_clone(1..);
    assert_eq!(v, ["sssss", "12334567890", "c", "12334567890", "c"]);
}

#[test]
fn test_extend_from_within_complete_rande() {
    {
        let bump: Bump = Bump::new();
        let mut v = bump_vec![in &bump; 0, 1, 2, 3];
        v.extend_from_within_copy(..);
        assert_eq!(v, [0, 1, 2, 3, 0, 1, 2, 3]);
    }
    {
        let bump: Bump = Bump::new();
        let mut v = bump_vec![in &bump; 0, 1, 2, 3];
        v.extend_from_within_clone(..);
        assert_eq!(v, [0, 1, 2, 3, 0, 1, 2, 3]);
    }
}

#[test]
fn test_extend_from_within_empty_rande() {
    {
        let bump: Bump = Bump::new();
        let mut v = bump_vec![in &bump; 0, 1, 2, 3];
        v.extend_from_within_copy(1..1);
        assert_eq!(v, [0, 1, 2, 3]);
    }
    {
        let bump: Bump = Bump::new();
        let mut v = bump_vec![in &bump; 0, 1, 2, 3];
        v.extend_from_within_clone(1..1);
        assert_eq!(v, [0, 1, 2, 3]);
    }
}

#[test]
#[should_panic]
fn test_extend_from_within_out_of_rande() {
    {
        let bump: Bump = Bump::new();
        let mut v = bump_vec![in &bump; 0, 1];
        v.extend_from_within_copy(..3);
    }

    {
        let bump: Bump = Bump::new();
        let mut v = bump_vec![in &bump; 0, 1];
        v.extend_from_within_clone(..3);
    }
}

#[test]
fn test_extend_from_within_zst() {
    {
        let bump: Bump = Bump::new();
        let mut v = bump_vec![in &bump; (); 8];
        v.extend_from_within_copy(3..7);
        assert_eq!(v, [(); 12]);
    }
    {
        let bump: Bump = Bump::new();
        let mut v = bump_vec![in &bump; (); 8];
        v.extend_from_within_clone(3..7);
        assert_eq!(v, [(); 12]);
    }
}

#[test]
fn test_extend_from_within_empty_vec() {
    {
        let bump: Bump = Bump::new();
        let mut v = BumpVec::<i32, _>::new_in(&bump);
        v.extend_from_within_copy(..);
        assert!(v.is_empty());
    }
    {
        let bump: Bump = Bump::new();
        let mut v = BumpVec::<i32, _>::new_in(&bump);
        v.extend_from_within_clone(..);
        assert!(v.is_empty());
    }
}

#[test]
fn test_extend_from_within() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; String::from("a"), String::from("b"), String::from("c")];
    v.extend_from_within_clone(1..=2);
    v.extend_from_within_clone(..=1);
    assert_eq!(v, ["a", "b", "c", "b", "c", "a", "b"]);
}

#[test]
fn test_vec_dedup_by() {
    let bump: Bump = Bump::new();
    let mut vec: BumpVec<i32, _> = bump_vec![in &bump; 1, -1, 2, 3, 1, -5, 5, -2, 2];

    vec.dedup_by(|a, b| a.abs() == b.abs());

    assert_eq!(vec, [1, 2, 3, 1, -5, -2]);
}

#[test]
fn test_vec_dedup_empty() {
    let bump: Bump = Bump::new();
    let mut vec: BumpVec<i32, _> = BumpVec::new_in(&bump);

    vec.dedup();

    assert!(vec.is_empty());
}

#[test]
fn test_vec_dedup_one() {
    let bump: Bump = Bump::new();
    let mut vec = bump_vec![in &bump; 12i32];

    vec.dedup();

    assert_eq!(vec, [12]);
}

#[test]
fn test_vec_dedup_multiple_ident() {
    let bump: Bump = Bump::new();
    let mut vec = bump_vec![in &bump; 12, 12, 12, 12, 12, 11, 11, 11, 11, 11, 11];

    vec.dedup();

    assert_eq!(vec, [12, 11]);
}

#[test]
fn test_vec_dedup_partialeq() {
    #[derive(Debug)]
    #[allow(dead_code)]
    struct Foo(i32, i32);

    impl PartialEq for Foo {
        fn eq(&self, other: &Foo) -> bool {
            self.0 == other.0
        }
    }

    let bump: Bump = Bump::new();
    let mut vec = bump_vec![in &bump; Foo(0, 1), Foo(0, 5), Foo(1, 7), Foo(1, 9)];

    vec.dedup();
    assert_eq!(vec, [Foo(0, 1), Foo(1, 7)]);
}

#[test]
fn test_vec_dedup() {
    let bump0: Bump = Bump::new();
    let bump1: Bump = Bump::new();

    let mut vec: BumpVec<bool, _> = BumpVec::with_capacity_in(8, &bump0);

    let mut template = BumpVec::new_in(&bump1);
    template.extend(vec.iter());

    for x in 0u8..255u8 {
        vec.clear();
        template.clear();

        let iter = (0..8).map(move |bit| (x >> bit) & 1 == 1);
        vec.extend(iter);
        template.extend_from_slice_copy(&vec);

        let (dedup, _) = template.partition_dedup();
        vec.dedup();

        assert_eq!(vec, dedup);
    }
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_vec_dedup_panicking() {
    #[derive(Debug)]
    struct Panic<'a> {
        drop_counter: &'a Cell<u32>,
        value: bool,
        index: usize,
    }

    impl PartialEq for Panic<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.value == other.value
        }
    }

    impl Drop for Panic<'_> {
        fn drop(&mut self) {
            self.drop_counter.set(self.drop_counter.get() + 1);
            if !std::thread::panicking() {
                assert!(self.index != 4);
            }
        }
    }

    let bump: Bump = Bump::new();
    let drop_counter = &Cell::new(0);
    let expected = [
        Panic {
            drop_counter,
            value: false,
            index: 0,
        },
        Panic {
            drop_counter,
            value: false,
            index: 5,
        },
        Panic {
            drop_counter,
            value: true,
            index: 6,
        },
        Panic {
            drop_counter,
            value: true,
            index: 7,
        },
    ];
    let mut vec = bump_vec![in &bump;
        Panic {
            drop_counter,
            value: false,
            index: 0,
        },
        // these elements get deduplicated
        Panic {
            drop_counter,
            value: false,
            index: 1,
        },
        Panic {
            drop_counter,
            value: false,
            index: 2,
        },
        Panic {
            drop_counter,
            value: false,
            index: 3,
        },
        Panic {
            drop_counter,
            value: false,
            index: 4,
        },
        // here it panics while dropping the item with index==4
        Panic {
            drop_counter,
            value: false,
            index: 5,
        },
        Panic {
            drop_counter,
            value: true,
            index: 6,
        },
        Panic {
            drop_counter,
            value: true,
            index: 7,
        },
    ];

    let _ = catch_unwind(AssertUnwindSafe(|| vec.dedup())).unwrap_err();

    assert_eq!(drop_counter.get(), 4);

    let ok = vec.iter().zip(expected.iter()).all(|(x, y)| x.index == y.index);

    if !ok {
        panic!("expected: {expected:?}\ngot: {vec:?}\n");
    }
}

// Regression test for issue #82533
#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_extend_from_within_panicking_clone() {
    struct Panic<'dc> {
        drop_count: &'dc AtomicU32,
        aaaaa: bool,
    }

    impl Clone for Panic<'_> {
        fn clone(&self) -> Self {
            if self.aaaaa {
                panic!("panic! at the clone");
            }

            Self { ..*self }
        }
    }

    impl Drop for Panic<'_> {
        fn drop(&mut self) {
            self.drop_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    let count = core::sync::atomic::AtomicU32::new(0);
    let bump: Bump = Bump::new();

    let mut vec = bump_vec![in &bump;
        Panic {
            drop_count: &count,
            aaaaa: false,
        },
        Panic {
            drop_count: &count,
            aaaaa: true,
        },
        Panic {
            drop_count: &count,
            aaaaa: false,
        },
    ];

    // This should clone&append one Panic{..} at the end, and then panic while
    // cloning second Panic{..}. This means that `Panic::drop` should be called
    // 4 times (3 for items already in vector, 1 for just appended).
    //
    // Previously just appended item was leaked, making drop_count = 3, instead of 4.
    std::panic::catch_unwind(move || vec.extend_from_within_clone(..)).unwrap_err();

    assert_eq!(count.load(Ordering::SeqCst), 4);
}

#[test]
#[should_panic = "vec len overflow"]
fn test_into_flattened_size_overflow() {
    let bump: Bump = Bump::new();
    let v = bump_vec![in &bump; [(); usize::MAX]; 2];
    let _ = v.into_flattened();
}

/*
TODO

#[test]
fn test_pop_if() {
    let mut v = vec![1, 2, 3, 4];
    let pred = |x: &mut i32| *x % 2 == 0;

    assert_eq!(v.pop_if(pred), Some(4));
    assert_eq!(v, [1, 2, 3]);

    assert_eq!(v.pop_if(pred), None);
    assert_eq!(v, [1, 2, 3]);
}

#[test]
fn test_pop_if_empty() {
    let mut v = Vec::<i32>::new();
    assert_eq!(v.pop_if(|_| true), None);
    assert!(v.is_empty());
}

#[test]
fn test_pop_if_mutates() {
    let mut v = vec![1];
    let pred = |x: &mut i32| {
        *x += 1;
        false
    };
    assert_eq!(v.pop_if(pred), None);
    assert_eq!(v, [2]);
}
*/

/// This assortment of tests, in combination with miri, verifies we handle UB on fishy arguments
/// in the stdlib. Draining and extending the allocation are fairly well-tested earlier, but
/// `vec.insert(usize::MAX, val)` once slipped by!
///
/// All code that manipulates the collection types should be tested with "trivially wrong" args.
#[test]
fn max_dont_panic() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 0];
    let _ = v.get(usize::MAX);
    // v.shrink_to(usize::MAX); // TODO
    v.truncate(usize::MAX);
}

#[test]
#[should_panic]
fn max_insert() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 0];
    v.insert(usize::MAX, 1);
}

#[test]
#[should_panic]
fn max_remove() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 0];
    v.remove(usize::MAX);
}

#[test]
#[should_panic]
fn max_splice() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 0];
    v.splice(usize::MAX.., core::iter::once(1));
}

#[test]
#[should_panic]
fn max_swap_remove() {
    let bump: Bump = Bump::new();
    let mut v = bump_vec![in &bump; 0];
    v.swap_remove(usize::MAX);
}
