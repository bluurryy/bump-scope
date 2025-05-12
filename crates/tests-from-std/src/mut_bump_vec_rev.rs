//! Adapted from rust's `library/alloctests/tests/vec.rs` commit fb04372dc56129d69e39af80cac6e81694bd285f

use core::alloc::Layout;
use core::num::NonZero;
use core::ptr::NonNull;
use core::{assert_eq, assert_ne};
use std::fmt::Debug;
use std::hint;
use std::mem::swap;
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicU32, Ordering};

use bump_scope::alloc::{AllocError, Allocator};
use bump_scope::{Bump, BumpAllocator};

type Vec<T, A = bump_scope::Bump> = bump_scope::MutBumpVecRev<T, A>;

trait VecNew: Sized {
    fn new() -> Self;
    fn with_capacity(n: usize) -> Self;
    fn try_with_capacity(n: usize) -> Result<Self, AllocError>;
}

impl<T> VecNew for Vec<T> {
    fn new() -> Self {
        Vec::new_in(Default::default())
    }

    fn with_capacity(n: usize) -> Self {
        Vec::with_capacity_in(n, Default::default())
    }

    fn try_with_capacity(n: usize) -> Result<Self, AllocError> {
        Vec::try_with_capacity_in(n, Default::default()).map_err(Into::into)
    }
}

macro_rules! vec {
    (in $($tt:tt)*) => {
        bump_scope::mut_bump_vec_rev![in $($tt)*]
    };
    ($($tt:tt)*) => {
        bump_scope::mut_bump_vec_rev![in <bump_scope::Bump>::default(); $($tt)*]
    };
}

struct DropCounter<'a> {
    count: &'a mut u32,
}

impl Drop for DropCounter<'_> {
    fn drop(&mut self) {
        *self.count += 1;
    }
}

#[test]
fn test_small_vec_struct() {
    assert_eq!(size_of::<Vec<u8>>(), size_of::<usize>() * 4);
}

#[test]
fn test_double_drop() {
    struct TwoVec<T> {
        x: Vec<T>,
        y: Vec<T>,
    }

    let (mut count_x, mut count_y) = (0, 0);
    {
        let mut tv = TwoVec { x: Vec::new(), y: Vec::new() };
        tv.x.push(DropCounter { count: &mut count_x });
        tv.y.push(DropCounter { count: &mut count_y });

        // If Vec had a drop flag, here is where it would be zeroed.
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
    let mut v = Vec::new();
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
fn test_zst_capacity() {
    assert_eq!(Vec::<()>::new().capacity(), usize::MAX);
}

#[test]
fn test_indexing() {
    let v: Vec<isize> = vec![10, 20];
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
    let vec1: Vec<isize> = vec![];
    assert_eq!("[]", format!("{:?}", vec1));

    let vec2 = vec![0, 1];
    assert_eq!("[0, 1]", format!("{:?}", vec2));

    let slice: &[isize] = &[4, 5];
    assert_eq!("[4, 5]", format!("{slice:?}"));
}

#[test]
fn test_push() {
    let mut v = vec![];
    v.push(1);
    assert_eq!(v, [1]);
    v.push(2);
    assert_eq!(v, [2, 1]);
    v.push(3);
    assert_eq!(v, [3, 2, 1]);
}

#[test]
fn test_extend() {
    let mut bump_v: Bump = Bump::new();
    let mut bump_w: Bump = Bump::new();

    let mut v = Vec::new_in(&mut bump_v);
    let mut w = Vec::new_in(&mut bump_w);

    v.extend(w.iter().copied());
    assert_eq!(v, &[]);

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

    v.extend(w.iter().copied()); // specializes to `append` (no it doesn't)
    assert!(v.iter().eq(w.iter().rev().chain(w.iter())));

    // Zero sized types
    #[derive(PartialEq, Debug)]
    struct Foo;

    let mut a = Vec::new();
    let b = vec![Foo, Foo];

    a.extend(b);
    assert_eq!(a, &[Foo, Foo]);

    // Double drop
    let mut count_x = 0;
    {
        let mut x = Vec::new();
        let y = vec![DropCounter { count: &mut count_x }];
        x.extend(y);
    }
    assert_eq!(count_x, 1);
}

#[test]
fn test_extend_from_slice() {
    let a: Vec<isize> = vec![1, 2, 3, 4, 5];
    let b: Vec<isize> = vec![6, 7, 8, 9, 0];

    let mut v: Vec<isize> = a;

    v.extend_from_slice_copy(&b);

    assert_eq!(v, [6, 7, 8, 9, 0, 1, 2, 3, 4, 5]);
}

#[test]
fn test_extend_ref() {
    let mut v = vec![1, 2];
    v.extend(&[3, 4, 5]);

    assert_eq!(v.len(), 5);
    assert_eq!(v, [5, 4, 3, 1, 2]);

    let w = vec![6, 7];
    v.extend(&w);

    assert_eq!(v.len(), 7);
    assert_eq!(v, [7, 6, 5, 4, 3, 1, 2]);
}

#[test]
fn test_slice_from_ref() {
    let values = vec![1, 2, 3, 4, 5];
    let slice = &values[1..3];

    assert_eq!(slice, [2, 3]);
}

#[test]
fn test_slice_from_mut() {
    let mut values = vec![1, 2, 3, 4, 5];
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
    let mut values = vec![1, 2, 3, 4, 5];
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
    let mut values = vec![1, 2, 3, 4, 5];
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

#[cfg(any())] // not applicable
#[test]
fn test_clone() {}

#[cfg(any())] // not applicable
#[test]
fn test_clone_from() {}

#[cfg(any())] // not yet implemented
#[test]
fn test_retain() {}

#[cfg(any())] // not yet implemented
fn test_retain_predicate_order() {}

#[cfg(any())] // not yet implemented
fn test_retain_pred_panic_with_hole() {}

#[cfg(any())] // not yet implemented
fn test_retain_pred_panic_no_hole() {}

#[cfg(any())] // not yet implemented
fn test_retain_drop_panic() {}

#[cfg(any())] // not yet implemented
fn test_retain_maybeuninits() {}

#[cfg(any())] // not yet implemented
fn test_dedup() {}

#[cfg(any())] // not yet implemented
fn test_dedup_by_key() {}

#[cfg(any())] // not yet implemented
fn test_dedup_by() {}

#[cfg(any())] // not yet implemented
fn test_dedup_unique() {}

#[test]
fn zero_sized_values() {
    let mut v = Vec::new();
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
    assert_eq!([].into_iter().partition(|x: &i32| *x < 3), (vec![], vec![]));
    assert_eq!([1, 2, 3].into_iter().partition(|x| *x < 4), (vec![3, 2, 1], vec![]));
    assert_eq!([1, 2, 3].into_iter().partition(|x| *x < 2), (vec![1], vec![3, 2]));
    assert_eq!([1, 2, 3].into_iter().partition(|x| *x < 0), (vec![], vec![3, 2, 1]));
}

#[test]
fn test_zip_unzip() {
    let z1 = vec![(1, 4), (2, 5), (3, 6)];

    let (left, right): (Vec<_>, Vec<_>) = z1.iter().cloned().unzip();

    assert_eq!((3, 6), (left[0], right[0]));
    assert_eq!((2, 5), (left[1], right[1]));
    assert_eq!((1, 4), (left[2], right[2]));
}

#[test]
fn test_cmp() {
    let x: &[isize] = &[1, 2, 3, 4, 5];
    let cmp: &[isize] = &[1, 2, 3, 4, 5];
    assert_eq!(&x[..], cmp);
    let cmp: &[isize] = &[3, 4, 5];
    assert_eq!(&x[2..], cmp);
    let cmp: &[isize] = &[1, 2, 3];
    assert_eq!(&x[..3], cmp);
    let cmp: &[isize] = &[2, 3, 4];
    assert_eq!(&x[1..4], cmp);

    let x: Vec<isize> = vec![1, 2, 3, 4, 5];
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

    let mut v = vec![Elem(1), Elem(2), Elem(3), Elem(4), Elem(5)];
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

    let mut v = vec![BadElem(1), BadElem(2), BadElem(0xbadbeef), BadElem(4)];
    v.truncate(0);
}

#[test]
fn test_index() {
    let vec = vec![1, 2, 3];
    assert!(vec[1] == 2);
}

#[test]
#[should_panic]
fn test_index_out_of_bounds() {
    let vec = vec![1, 2, 3];
    let _ = vec[3];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_1() {
    let x = vec![1, 2, 3, 4, 5];
    let _ = &x[!0..];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_2() {
    let x = vec![1, 2, 3, 4, 5];
    let _ = &x[..6];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_3() {
    let x = vec![1, 2, 3, 4, 5];
    let _ = &x[!0..4];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_4() {
    let x = vec![1, 2, 3, 4, 5];
    let _ = &x[1..6];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_5() {
    let x = vec![1, 2, 3, 4, 5];
    let _ = &x[3..2];
}

#[test]
#[should_panic]
fn test_swap_remove_empty() {
    let mut vec = Vec::<i32>::new();
    vec.swap_remove(0);
}

#[test]
fn test_move_items() {
    let vec = vec![1, 2, 3];
    let mut vec2 = vec![];
    for i in vec {
        vec2.push(i);
    }
    assert_eq!(vec2, [3, 2, 1]);
}

#[test]
fn test_move_items_reverse() {
    let vec = vec![1, 2, 3];
    let mut vec2 = vec![];
    for i in vec.into_iter().rev() {
        vec2.push(i);
    }
    assert_eq!(vec2, [1, 2, 3]);
}

#[test]
fn test_move_items_zero_sized() {
    let vec = vec![(), (), ()];
    let mut vec2 = vec![];
    for i in vec {
        vec2.push(i);
    }
    assert_eq!(vec2, [(), (), ()]);
}

#[cfg(any())] // not yet implemented
fn test_drain_empty_vec() {}

#[cfg(any())] // not yet implemented
fn test_drain_items() {}

#[cfg(any())] // not yet implemented
fn test_drain_items_reverse() {}

#[cfg(any())] // not yet implemented
fn test_drain_items_zero_sized() {}

#[cfg(any())] // not yet implemented
fn test_drain_out_of_bounds() {}

#[cfg(any())] // not yet implemented
fn test_drain_range() {}

#[cfg(any())] // not yet implemented
fn test_drain_inclusive_range() {}

#[cfg(any())] // not yet implemented
fn test_drain_max_vec_size() {}

#[cfg(any())] // not yet implemented
fn test_drain_index_overflow() {}

#[cfg(any())] // not yet implemented
fn test_drain_inclusive_out_of_bounds() {}

#[cfg(any())] // not yet implemented
fn test_drain_start_overflow() {}

#[cfg(any())] // not yet implemented
fn test_drain_end_overflow() {}

#[cfg(any())] // not yet implemented
fn test_drain_leak() {}

#[cfg(any())] // not yet implemented
fn test_drain_keep_rest() {}

#[cfg(any())] // not yet implemented
fn test_drain_keep_rest_all() {}

#[cfg(any())] // not yet implemented
fn test_drain_keep_rest_none() {}

#[cfg(any())] // not applicable
fn test_splice() {}

#[cfg(any())] // not applicable
fn test_splice_inclusive_range() {}

#[cfg(any())] // not applicable
fn test_splice_out_of_bounds() {}

#[cfg(any())] // not applicable
fn test_splice_inclusive_out_of_bounds() {}

#[cfg(any())] // not applicable
fn test_splice_items_zero_sized() {}

#[cfg(any())] // not applicable
fn test_splice_unbounded() {}

#[cfg(any())] // not applicable
fn test_splice_forget() {}

#[test]
fn test_into_boxed_slice() {
    let mut bump: Bump = Bump::new();
    let xs = vec![in &mut bump; 1, 2, 3];
    let ys = xs.into_boxed_slice();
    assert_eq!(&*ys, [1, 2, 3]);
}

#[test]
fn test_append() {
    let mut vec = vec![1, 2, 3];
    let mut vec2 = vec![4, 5, 6];
    vec.append(&mut vec2);
    assert_eq!(vec, [4, 5, 6, 1, 2, 3]);
    assert_eq!(vec2, []);
}

#[cfg(any())] // not applicable
#[test]
fn test_split_off() {}

#[cfg(any())] // not applicable
#[test]
fn test_split_off_take_all() {}

#[test]
fn test_into_iter_as_slice() {
    let vec = vec!['a', 'b', 'c'];
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
    let vec = vec!['a', 'b', 'c'];
    let mut into_iter = vec.into_iter();
    assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    into_iter.as_mut_slice()[0] = 'x';
    into_iter.as_mut_slice()[1] = 'y';
    assert_eq!(into_iter.next().unwrap(), 'x');
    assert_eq!(into_iter.as_slice(), &['y', 'c']);
}

#[test]
fn test_into_iter_debug() {
    let vec = vec!['a', 'b', 'c'];
    let into_iter = vec.into_iter();
    let debug = format!("{into_iter:?}");
    assert_eq!(debug, "IntoIter(['a', 'b', 'c'])");
}

#[test]
fn test_into_iter_count() {
    assert_eq!([1, 2, 3].into_iter().count(), 3);
}

#[test]
fn test_into_iter_next_chunk() {
    let mut iter = b"lorem".to_vec().into_iter();

    assert_eq!(iter.next_chunk().unwrap(), [b'l', b'o']); // N is inferred as 2
    assert_eq!(iter.next_chunk().unwrap(), [b'r', b'e', b'm']); // N is inferred as 3
    assert_eq!(iter.next_chunk::<4>().unwrap_err().as_slice(), &[]); // N is explicitly 4
}

#[test]
fn test_into_iter_clone() {
    fn iter_equal<I: DoubleEndedIterator<Item = i32>>(it: I, slice: &[i32]) {
        let mut v = Vec::new();
        v.extend(it.rev());
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

    let v = vec![D(false), D(true), D(false)];

    catch_unwind(move || drop(v.into_iter())).ok();

    assert_eq!(unsafe { DROPS }, 3);
}

#[test]
fn test_into_iter_advance_by() {
    let mut i = vec![1, 2, 3, 4, 5].into_iter();
    assert_eq!(i.advance_by(0), Ok(()));
    assert_eq!(i.advance_back_by(0), Ok(()));
    assert_eq!(i.as_slice(), [1, 2, 3, 4, 5]);

    assert_eq!(i.advance_by(1), Ok(()));
    assert_eq!(i.advance_back_by(1), Ok(()));
    assert_eq!(i.as_slice(), [2, 3, 4]);

    assert_eq!(i.advance_back_by(usize::MAX), Err(NonZero::new(usize::MAX - 3).unwrap()));

    assert_eq!(i.advance_by(usize::MAX), Err(NonZero::new(usize::MAX).unwrap()));

    assert_eq!(i.advance_by(0), Ok(()));
    assert_eq!(i.advance_back_by(0), Ok(()));

    assert_eq!(i.len(), 0);
}

#[test]
fn test_into_iter_drop_allocator() {
    struct ReferenceCountedAllocator<'a> {
        bump: Bump,
        #[allow(dead_code)]
        counter: DropCounter<'a>,
    }

    unsafe impl Allocator for ReferenceCountedAllocator<'_> {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            self.bump.allocate(layout)
        }

        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            // Safety: Invariants passed to caller.
            unsafe { self.bump.deallocate(ptr, layout) }
        }
    }

    unsafe impl BumpAllocator for ReferenceCountedAllocator<'_> {}

    let mut drop_count = 0;

    let allocator = ReferenceCountedAllocator {
        bump: Bump::new(),
        counter: DropCounter { count: &mut drop_count },
    };
    let _ = Vec::<u32, _>::new_in(allocator);
    assert_eq!(drop_count, 1);

    let allocator = ReferenceCountedAllocator {
        bump: Bump::new(),
        counter: DropCounter { count: &mut drop_count },
    };
    let _ = Vec::<u32, _>::new_in(allocator).into_iter();
    assert_eq!(drop_count, 2);
}

#[test]
fn test_into_iter_zst() {
    #[derive(Debug, Clone)]
    struct AlignedZstWithDrop([u64; 0]);
    impl Drop for AlignedZstWithDrop {
        fn drop(&mut self) {
            let addr = self as *mut _ as usize;
            assert!(hint::black_box(addr) % align_of::<u64>() == 0);
        }
    }

    const C: AlignedZstWithDrop = AlignedZstWithDrop([0u64; 0]);

    for _ in vec![C].into_iter() {}
    for _ in vec![C; 5].into_iter().rev() {}

    let mut it = vec![C, C].into_iter();
    assert_eq!(it.advance_by(1), Ok(()));
    drop(it);

    let mut it = vec![C, C].into_iter();
    it.next_chunk::<1>().unwrap();
    drop(it);

    let mut it = vec![C, C].into_iter();
    it.next_chunk::<4>().unwrap_err();
    drop(it);
}

#[cfg(any())] // not applicable
fn test_from_iter_specialization() {}

#[cfg(any())] // not applicable
fn test_from_iter_partially_drained_in_place_specialization() {}

#[cfg(any())] // not applicable
fn test_from_iter_specialization_with_iterator_adapters() {}

#[cfg(any())] // not applicable
fn test_in_place_specialization_step_up_down() {}

#[cfg(any())] // not applicable
fn test_from_iter_specialization_head_tail_drop() {}

#[cfg(any())] // not applicable
fn test_from_iter_specialization_panic_during_iteration_drops() {}

#[cfg(any())] // not applicable
fn test_from_iter_specialization_panic_during_drop_doesnt_leak() {}

#[cfg(any())] // not applicable
#[test]
fn test_collect_after_iterator_clone() {}

#[cfg(any())] // not applicable
#[test]
fn test_flatten_clone() {}

#[cfg(any())] // not applicable
#[test]
fn test_cow_from() {}

#[cfg(any())] // not applicable
#[test]
fn test_from_cow() {}

#[cfg(any())] // TODO: fix this
#[allow(dead_code)]
fn assert_covariance() {
    fn drain<'new>(d: Drain<'static, &'static str>) -> Drain<'new, &'new str> {
        d
    }
    fn into_iter<'new>(i: IntoIter<'static, &'static str>) -> IntoIter<'new, &'new str> {
        i
    }
}

#[cfg(any())] // not applicable (no `FromIterator` impl nor specialization)
#[test]
fn from_into_inner() {}

#[test]
#[cfg(not(miri))] // too slow
fn overaligned_allocations() {
    #[repr(align(256))]
    struct Foo(usize);
    let mut v = vec![Foo(273)];
    for i in 0..0x1000 {
        v.reserve_exact(i);
        assert!(v[0].0 == 273);
        assert!(v.as_ptr() as usize & 0xff == 0);
        // `MutBumpVec can't shrink`
        // v.shrink_to_fit();
        // assert!(v[0].0 == 273);
        // assert!(v.as_ptr() as usize & 0xff == 0);
    }
}

#[cfg(any())] // not yet implemented
fn extract_if_empty() {}

#[cfg(any())] // not yet implemented
fn extract_if_zst() {}

#[cfg(any())] // not yet implemented
fn extract_if_false() {}

#[cfg(any())] // not yet implemented
fn extract_if_true() {}

#[cfg(any())] // not yet implemented
fn extract_if_ranges() {}

#[cfg(any())] // not yet implemented
fn extract_if_out_of_bounds() {}

#[cfg(any())] // not yet implemented
fn extract_if_complex() {}

#[cfg(any())] // not yet implemented
fn extract_if_consumed_panic() {}

#[cfg(any())] // not yet implemented
fn extract_if_unconsumed_panic() {}

#[cfg(any())] // not yet implemented
fn extract_if_unconsumed() {}

#[test]
fn test_reserve_exact() {
    // This is all the same as test_reserve

    let mut v = Vec::new();
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
    let mut vec: Vec<u32> = Vec::try_with_capacity(5).unwrap();
    assert_eq!(0, vec.len());
    assert!(vec.capacity() >= 5 && vec.capacity() <= isize::MAX as usize / 4);
    assert!(vec.spare_capacity_mut().len() >= 5);

    assert!(Vec::<u16>::try_with_capacity(isize::MAX as usize + 1).is_err());
}

#[cfg(any())] // we don't have try reserve error variants
fn test_try_reserve() {}

#[cfg(any())] // we don't have try reserve error variants
fn test_try_reserve_exact() {}

// TODO: implement `MutBumpVec::splice`
#[cfg(any())]
#[test]
fn test_stable_pointers() {
    /// Pull an element from the iterator, then drop it.
    /// Useful to cover both the `next` and `drop` paths of an iterator.
    fn next_then_drop<I: Iterator>(mut i: I) {
        i.next().unwrap();
        drop(i);
    }

    // Test that, if we reserved enough space, adding and removing elements does not
    // invalidate references into the vector (such as `v0`). This test also
    // runs in Miri, which would detect such problems.
    // Note that this test does *not* constitute a stable guarantee that all these functions do not
    // reallocate! Only what is explicitly documented at
    // <https://doc.rust-lang.org/nightly/std/vec/struct.Vec.html#guarantees> is stably guaranteed.
    let mut v = Vec::with_capacity(128);
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
    v.append(&mut vec![27, 19]);
    assert_eq!(*v0, 13);

    // Extending
    v.extend_from_slice_copy(&[1, 2]);
    v.extend(&[1, 2]); // `slice::Iter` (with `T: Copy`) specialization
    v.extend(vec![2, 3]); // `vec::IntoIter` specialization
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
    next_then_drop(v.splice(5.., vec![1, 2, 3, 4, 5])); // empty tail after range
    assert_eq!(*v0, 13);
    next_then_drop(v.splice(5..8, vec![1])); // replacement is smaller than original range
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

// https://github.com/rust-lang/rust/pull/49496 introduced specialization based on:
//
// ```
// unsafe impl<T: ?Sized> IsZero for *mut T {
//     fn is_zero(&self) -> bool {
//         (*self).is_null()
//     }
// }
// ```
//
// … to call `RawVec::with_capacity_zeroed` for creating `Vec<*mut T>`,
// which is incorrect for fat pointers since `<*mut T>::is_null` only looks at the data component.
// That is, a fat pointer can be “null” without being made entirely of zero bits.
#[test]
fn vec_macro_repeating_null_raw_fat_pointer() {
    let raw_dyn = &mut (|| ()) as &mut dyn Fn() as *mut dyn Fn();
    let vtable = dbg!(ptr_metadata(raw_dyn));
    let null_raw_dyn = ptr_from_raw_parts(std::ptr::null_mut(), vtable);
    assert!(null_raw_dyn.is_null());

    let vec = vec![null_raw_dyn; 1];
    dbg!(ptr_metadata(vec[0]));
    assert!(std::ptr::eq(vec[0], null_raw_dyn));

    // Polyfill for https://github.com/rust-lang/rfcs/pull/2580

    fn ptr_metadata(ptr: *mut dyn Fn()) -> *mut () {
        unsafe { std::mem::transmute::<*mut dyn Fn(), DynRepr>(ptr).vtable }
    }

    fn ptr_from_raw_parts(data: *mut (), vtable: *mut ()) -> *mut dyn Fn() {
        unsafe { std::mem::transmute::<DynRepr, *mut dyn Fn()>(DynRepr { data, vtable }) }
    }

    #[repr(C)]
    struct DynRepr {
        data: *mut (),
        vtable: *mut (),
    }
}

// TODO: test `MutBumpVec` growth
#[cfg(any())]
#[test]
fn test_push_growth_strategy() {}

macro_rules! generate_assert_eq_vec_and_prim {
    ($name:ident<$B:ident>($type:ty)) => {
        fn $name<A: PartialEq<$B> + Debug, $B: Debug>(a: Vec<A>, b: $type) {
            assert!(a == b);
            assert_eq!(a, b);
        }
    };
}

generate_assert_eq_vec_and_prim! { assert_eq_vec_and_slice  <B>(&[B])   }
generate_assert_eq_vec_and_prim! { assert_eq_vec_and_array_3<B>([B; 3]) }

#[test]
fn partialeq_vec_and_prim() {
    assert_eq_vec_and_slice(vec![1, 2, 3], &[1, 2, 3]);
    assert_eq_vec_and_array_3(vec![1, 2, 3], [1, 2, 3]);
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
    let vec2: Vec<_> = vec![1, 2];
    let vec3: Vec<_> = vec![1, 2, 3];
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

#[cfg(any())] // TODO: `#[may_dangle]`?
#[test]
fn test_vec_cycle() {
    #[derive(Debug)]
    struct C<'a> {
        v: Vec<Cell<Option<&'a C<'a>>>>,
    }

    impl<'a> C<'a> {
        fn new() -> C<'a> {
            C { v: Vec::new() }
        }
    }

    let mut c1 = C::new();
    let mut c2 = C::new();
    let mut c3 = C::new();

    // Push
    c1.v.push(Cell::new(None));
    c1.v.push(Cell::new(None));

    c2.v.push(Cell::new(None));
    c2.v.push(Cell::new(None));

    c3.v.push(Cell::new(None));
    c3.v.push(Cell::new(None));

    // Set
    c1.v[0].set(Some(&c2));
    c1.v[1].set(Some(&c3));

    c2.v[0].set(Some(&c2));
    c2.v[1].set(Some(&c3));

    c3.v[0].set(Some(&c1));
    c3.v[1].set(Some(&c2));
}

#[cfg(any())] // TODO: `#[may_dangle]`?
#[test]
fn test_vec_cycle_wrapped() {
    struct Refs<'a> {
        v: Vec<Cell<Option<&'a C<'a>>>>,
    }

    struct C<'a> {
        refs: Refs<'a>,
    }

    impl<'a> Refs<'a> {
        fn new() -> Refs<'a> {
            Refs { v: Vec::new() }
        }
    }

    impl<'a> C<'a> {
        fn new() -> C<'a> {
            C { refs: Refs::new() }
        }
    }

    let mut c1 = C::new();
    let mut c2 = C::new();
    let mut c3 = C::new();

    c1.refs.v.push(Cell::new(None));
    c1.refs.v.push(Cell::new(None));
    c2.refs.v.push(Cell::new(None));
    c2.refs.v.push(Cell::new(None));
    c3.refs.v.push(Cell::new(None));
    c3.refs.v.push(Cell::new(None));

    c1.refs.v[0].set(Some(&c2));
    c1.refs.v[1].set(Some(&c3));
    c2.refs.v[0].set(Some(&c2));
    c2.refs.v[1].set(Some(&c3));
    c3.refs.v[0].set(Some(&c1));
    c3.refs.v[1].set(Some(&c2));
}

#[test]
fn test_zero_sized_capacity() {
    for len in [0, 1, 2, 4, 8, 16, 32, 64, 128, 256] {
        let v = Vec::<()>::with_capacity(len);
        assert_eq!(v.len(), 0);
        assert_eq!(v.capacity(), usize::MAX);
    }
}

#[test]
fn test_zero_sized_vec_push() {
    const N: usize = 8;

    for len in 0..N {
        let mut tester = Vec::with_capacity(len);
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
    assert_eq!(vec![1; 3], vec![1, 1, 1]);
    assert_eq!(vec![1; 2], vec![1, 1]);
    assert_eq!(vec![1; 1], vec![1]);
    assert_eq!(vec![1; 0], vec![]);

    // from_elem syntax (see RFC 832)
    let el = Box::new(1);
    let n = 3;
    assert_eq!(vec![el; n], vec![Box::new(1), Box::new(1), Box::new(1)]);
}

#[test]
fn test_vec_swap() {
    let mut a: Vec<isize> = vec![0, 1, 2, 3, 4, 5, 6];
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

    vec![CopyOnly, CopyOnly].extend_from_within_copy(..);
}

#[test]
fn test_extend_from_within_clone() {
    let mut v = vec![String::from("sssss"), String::from("12334567890"), String::from("c")];
    v.extend_from_within_clone(1..);

    assert_eq!(v, ["12334567890", "c", "sssss", "12334567890", "c"]);
}

#[test]
fn test_extend_from_within_complete_rande() {
    let mut v = vec![0, 1, 2, 3];
    v.extend_from_within_copy(..);

    assert_eq!(v, [0, 1, 2, 3, 0, 1, 2, 3]);
}

#[test]
fn test_extend_from_within_empty_rande() {
    let mut v = vec![0, 1, 2, 3];
    v.extend_from_within_copy(1..1);

    assert_eq!(v, [0, 1, 2, 3]);
}

#[test]
#[should_panic]
fn test_extend_from_within_out_of_rande() {
    let mut v = vec![0, 1];
    v.extend_from_within_copy(..3);
}

#[test]
fn test_extend_from_within_zst() {
    let mut v = vec![(); 8];
    v.extend_from_within_copy(3..7);

    assert_eq!(v, [(); 12]);
}

#[test]
fn test_extend_from_within_empty_vec() {
    let mut v = Vec::<i32>::new();
    v.extend_from_within_copy(..);

    assert_eq!(v, []);
}

#[test]
fn test_extend_from_within() {
    let mut v = vec![String::from("a"), String::from("b"), String::from("c")];
    v.extend_from_within_clone(1..=2);
    v.extend_from_within_clone(..=1);

    assert_eq!(v, ["b", "c", "b", "c", "a", "b", "c"]);
}

#[cfg(any())] // not yet implemented
fn test_vec_dedup_by() {}

#[cfg(any())] // not yet implemented
fn test_vec_dedup_empty() {}

#[cfg(any())] // not yet implemented
fn test_vec_dedup_one() {}

#[cfg(any())] // not yet implemented
fn test_vec_dedup_multiple_ident() {}

#[cfg(any())] // not yet implemented
fn test_vec_dedup_partialeq() {}

#[cfg(any())] // not yet implemented
fn test_vec_dedup() {}

#[cfg(any())] // not yet implemented
fn test_vec_dedup_panicking() {}

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
    let mut vec = vec![
        Panic { drop_count: &count, aaaaa: false },
        Panic { drop_count: &count, aaaaa: true },
        Panic { drop_count: &count, aaaaa: false },
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
    let v = vec![[(); usize::MAX]; 2];
    let _ = v.into_flattened();
}

#[cfg(any())] // not applicable, `BumpAllocator` has special behavior that must accept any zero sized deallocations
fn test_box_zero_allocator() {}

#[cfg(any())] // not applicable
#[test]
fn test_vec_from_array_ref() {}

#[cfg(any())] // not applicable
#[test]
fn test_vec_from_array_mut_ref() {}

#[test]
fn test_pop_if() {
    let mut v = vec![4, 3, 2, 1];
    let pred = |x: &mut i32| *x % 2 == 0;

    assert_eq!(v.pop_if(pred), Some(4));
    assert_eq!(v, [3, 2, 1]);

    assert_eq!(v.pop_if(pred), None);
    assert_eq!(v, [3, 2, 1]);
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

/// This assortment of tests, in combination with miri, verifies we handle UB on fishy arguments
/// in the stdlib. Draining and extending the allocation are fairly well-tested earlier, but
/// `vec.insert(usize::MAX, val)` once slipped by!
///
/// All code that manipulates the collection types should be tested with "trivially wrong" args.
#[test]
fn max_dont_panic() {
    let mut v = vec![0];
    let _ = v.get(usize::MAX);
    // v.shrink_to(usize::MAX); TODO: implement shrink_to
    v.truncate(usize::MAX);
}

#[test]
#[should_panic]
fn max_insert() {
    let mut v = vec![0];
    v.insert(usize::MAX, 1);
}

#[test]
#[should_panic]
fn max_remove() {
    let mut v = vec![0];
    v.remove(usize::MAX);
}

#[cfg(any())] // TODO: implement `MutBumpVec::splice`
#[test]
#[should_panic]
fn max_splice() {
    let mut v = vec![0];
    v.splice(usize::MAX.., core::iter::once(1));
}

#[test]
#[should_panic]
fn max_swap_remove() {
    let mut v = vec![0];
    v.swap_remove(usize::MAX);
}

// Regression test for #135338
#[test]
fn vec_null_ptr_roundtrip() {
    let ptr = std::ptr::from_ref(&42);
    let zero = ptr.with_addr(0);
    let roundtripped = vec![zero; 1].pop().unwrap();
    let new = roundtripped.with_addr(ptr.addr());
    unsafe { new.read() };
}
