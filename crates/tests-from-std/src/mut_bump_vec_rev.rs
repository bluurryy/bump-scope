//! Adapted from rust's `library/alloc/tests/vec.rs` commit f7ca9df69549470541fbf542f87a03eb9ed024b6

use std::{
    alloc::System,
    assert_eq,
    boxed::Box,
    fmt::Debug,
    format, hint,
    iter::IntoIterator,
    mem::{self, size_of, swap},
    num::NonZeroUsize,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr::NonNull,
    string::String,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU32, Ordering},
    },
    vec::{Drain, IntoIter},
};

use std::alloc::{AllocError, Allocator, Layout};

use bump_scope::{Bump, MutBumpVecRev, mut_bump_vec_rev};

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
    assert_eq!(size_of::<MutBumpVecRev<u8, Bump>>(), size_of::<usize>() * 4);
}

#[test]
fn test_double_drop() {
    let mut bump_x: Bump = Bump::new();
    let mut bump_y: Bump = Bump::new();

    struct TwoVec<'a, T> {
        x: MutBumpVecRev<T, &'a mut Bump>,
        y: MutBumpVecRev<T, &'a mut Bump>,
    }

    let (mut count_x, mut count_y) = (0, 0);
    {
        let mut tv = TwoVec {
            x: MutBumpVecRev::new_in(&mut bump_x),
            y: MutBumpVecRev::new_in(&mut bump_y),
        };
        tv.x.push(DropCounter { count: &mut count_x });
        tv.y.push(DropCounter { count: &mut count_y });

        // If MutBumpVecRev had a drop flag, here is where it would be zeroed.
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
    let mut bump: Bump = Bump::new();

    let mut v = MutBumpVecRev::new_in(&mut bump);
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
    let mut bump: Bump = Bump::new();

    assert_eq!(MutBumpVecRev::<(), &mut Bump>::new_in(&mut bump).capacity(), usize::MAX);
}

#[test]
fn test_indexing() {
    let mut bump: Bump = Bump::new();

    let v: MutBumpVecRev<isize, _> = mut_bump_vec_rev![in &mut bump; 10, 20];
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
    let mut bump1: Bump = Bump::new();
    let mut bump2: Bump = Bump::new();

    let vec1: MutBumpVecRev<isize, _> = mut_bump_vec_rev![in &mut bump1; ];
    assert_eq!("[]", format!("{:?}", vec1));

    let vec2 = mut_bump_vec_rev![in &mut bump2; 0, 1];
    assert_eq!("[0, 1]", format!("{:?}", vec2));

    let slice: &[isize] = &[4, 5];
    assert_eq!("[4, 5]", format!("{slice:?}"));
}

#[test]
fn test_push() {
    let mut bump: Bump = Bump::new();

    let mut v = mut_bump_vec_rev![in &mut bump; ];
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

    let mut v = MutBumpVecRev::<i32, _>::new_in(&mut bump_v);
    let mut w = MutBumpVecRev::<i32, _>::new_in(&mut bump_w);

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
    assert!(v.iter().eq(w.iter().rev().chain(w.iter())));

    // Zero sized types
    #[derive(PartialEq, Debug)]
    struct Foo;

    let mut bump_a: Bump = Bump::new();
    let mut bump_b: Bump = Bump::new();

    let mut a = MutBumpVecRev::new_in(&mut bump_a);
    let b = mut_bump_vec_rev![in &mut bump_b; Foo, Foo];

    a.extend(b);
    assert_eq!(a, &[Foo, Foo]);

    // Double drop
    let mut count_x = 0;
    {
        let mut bump_x: Bump = Bump::new();
        let mut bump_y: Bump = Bump::new();

        let mut x = MutBumpVecRev::new_in(&mut bump_x);
        let y = mut_bump_vec_rev![in &mut bump_y; DropCounter { count: &mut count_x }];
        x.extend(y);
    }
    assert_eq!(count_x, 1);
}

#[test]
fn test_extend_from_slice() {
    let mut bump_a: Bump = Bump::new();
    let mut bump_b: Bump = Bump::new();

    let b: MutBumpVecRev<isize, _> = mut_bump_vec_rev![in &mut bump_b; 6, 7, 8, 9, 0];
    let a: MutBumpVecRev<isize, _> = mut_bump_vec_rev![in &mut bump_a; 1, 2, 3, 4, 5];

    let mut v: MutBumpVecRev<isize, _> = a;

    v.extend_from_slice_copy(&b);

    assert_eq!(v, [6, 7, 8, 9, 0, 1, 2, 3, 4, 5]);
}

#[test]
fn test_extend_ref() {
    let mut bump_v: Bump = Bump::new();
    let mut bump_w: Bump = Bump::new();

    let mut v = mut_bump_vec_rev![in &mut bump_v; 1, 2];
    v.extend(&[3, 4, 5]);

    assert_eq!(v.len(), 5);
    assert_eq!(v, [5, 4, 3, 1, 2]);

    let w = mut_bump_vec_rev![in &mut bump_w; 6, 7];
    v.extend(&w);

    assert_eq!(v.len(), 7);
    assert_eq!(v, [7, 6, 5, 4, 3, 1, 2]);
}

#[test]
fn test_slice_from_ref() {
    let mut bump: Bump = Bump::new();

    let values = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
    let slice = &values[1..3];

    assert_eq!(slice, [2, 3]);
}

#[test]
fn test_slice_from_mut() {
    let mut bump: Bump = Bump::new();

    let mut values = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
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
    let mut bump: Bump = Bump::new();

    let mut values = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
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
    let mut bump: Bump = Bump::new();

    let mut values = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
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
fn zero_sized_values() {
    let mut bump: Bump = Bump::new();

    let mut v = MutBumpVecRev::new_in(&mut bump);
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
    let mut bump: Bump = Bump::new();

    let x: &[isize] = &[1, 2, 3, 4, 5];
    let cmp: &[isize] = &[1, 2, 3, 4, 5];
    assert_eq!(&x[..], cmp);
    let cmp: &[isize] = &[3, 4, 5];
    assert_eq!(&x[2..], cmp);
    let cmp: &[isize] = &[1, 2, 3];
    assert_eq!(&x[..3], cmp);
    let cmp: &[isize] = &[2, 3, 4];
    assert_eq!(&x[1..4], cmp);

    let x: MutBumpVecRev<isize, _> = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
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
    #[allow(dead_code)]
    struct Elem(i32);
    impl Drop for Elem {
        fn drop(&mut self) {
            unsafe {
                DROPS += 1;
            }
        }
    }

    let mut bump: Bump = Bump::new();
    let mut v = mut_bump_vec_rev![in &mut bump; Elem(1), Elem(2), Elem(3), Elem(4), Elem(5)];
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

    let mut bump: Bump = Bump::new();
    let mut v = mut_bump_vec_rev![in &mut bump; BadElem(1), BadElem(2), BadElem(0xbadbeef), BadElem(4)];
    v.truncate(0);
}

#[test]
fn test_index() {
    let mut bump: Bump = Bump::new();
    let vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    assert!(vec[1] == 2);
}

#[test]
#[should_panic]
fn test_index_out_of_bounds() {
    let mut bump: Bump = Bump::new();
    let vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    let _ = vec[3];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_1() {
    let mut bump: Bump = Bump::new();
    let x = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
    let _ = &x[!0..];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_2() {
    let mut bump: Bump = Bump::new();
    let x = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
    let _ = &x[..6];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_3() {
    let mut bump: Bump = Bump::new();
    let x = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
    let _ = &x[!0..4];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_4() {
    let mut bump: Bump = Bump::new();
    let x = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
    let _ = &x[1..6];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_5() {
    let mut bump: Bump = Bump::new();
    let x = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
    let _ = &x[3..2];
}

#[test]
#[should_panic]
fn test_swap_remove_empty() {
    let mut bump: Bump = Bump::new();
    let mut vec = MutBumpVecRev::<i32, _>::new_in(&mut bump);
    vec.swap_remove(0);
}

#[test]
fn test_move_items() {
    let mut bump0: Bump = Bump::new();
    let mut bump1: Bump = Bump::new();
    let vec = mut_bump_vec_rev![in &mut bump0; 1, 2, 3];
    let mut vec2 = mut_bump_vec_rev![in &mut bump1; ];
    for i in vec {
        vec2.push(i);
    }
    assert_eq!(vec2, [3, 2, 1]);
}

#[test]
fn test_move_items_reverse() {
    let mut bump0: Bump = Bump::new();
    let mut bump1: Bump = Bump::new();
    let vec = mut_bump_vec_rev![in &mut bump0; 1, 2, 3];
    let mut vec2 = mut_bump_vec_rev![in &mut bump1; ];
    for i in vec.into_iter().rev() {
        vec2.push(i);
    }
    assert_eq!(vec2, [1, 2, 3]);
}

#[test]
fn test_move_items_zero_sized() {
    let mut bump0: Bump = Bump::new();
    let mut bump1: Bump = Bump::new();
    let vec = mut_bump_vec_rev![in &mut bump0; (), (), ()];
    let mut vec2 = mut_bump_vec_rev![in &mut bump1; ];
    for i in vec {
        vec2.push(i);
    }
    assert_eq!(vec2, [(), (), ()]);
}

#[test]
fn test_into_boxed_slice() {
    let mut bump: Bump = Bump::new();
    let xs = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    let ys = xs.into_boxed_slice();
    assert_eq!(&*ys, [1, 2, 3]);
}

#[test]
fn test_append() {
    let mut bump: Bump = Bump::new();
    let bump = bump.as_mut_scope();
    let mut slice = bump.alloc_slice_copy(&[4, 5, 6]);
    let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3];
    vec.append(&mut slice);
    assert_eq!(vec, [4, 5, 6, 1, 2, 3]);
    assert!(slice.is_empty());
}

#[test]
fn test_into_iter_as_slice() {
    let mut bump: Bump = Bump::new();
    let vec = mut_bump_vec_rev![in &mut bump; 'a', 'b', 'c'];
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
    let mut bump: Bump = Bump::new();
    let vec = mut_bump_vec_rev![in &mut bump; 'a', 'b', 'c'];
    let mut into_iter = vec.into_iter();
    assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    into_iter.as_mut_slice()[0] = 'x';
    into_iter.as_mut_slice()[1] = 'y';
    assert_eq!(into_iter.next().unwrap(), 'x');
    assert_eq!(into_iter.as_slice(), &['y', 'c']);
}

#[test]
fn test_into_iter_debug() {
    let mut bump: Bump = Bump::new();
    let vec = mut_bump_vec_rev![in &mut bump; 'a', 'b', 'c'];
    let into_iter = vec.into_iter();
    let debug = format!("{into_iter:?}");
    assert_eq!(debug, "IntoIter(['a', 'b', 'c'])");
}

#[test]
fn test_into_iter_count() {
    let mut bump: Bump = Bump::new();
    let v = MutBumpVecRev::from_array_in([1, 2, 3], &mut bump);
    assert_eq!(v.into_iter().count(), 3);
}

#[test]
fn test_into_iter_next_chunk() {
    let mut bump: Bump = Bump::new();
    let mut iter = MutBumpVecRev::from_array_in(*b"lorem", &mut bump).into_iter();

    assert_eq!(iter.next_chunk().unwrap(), [b'l', b'o']); // N is inferred as 2
    assert_eq!(iter.next_chunk().unwrap(), [b'r', b'e', b'm']); // N is inferred as 3
    assert!(iter.next_chunk::<4>().unwrap_err().as_slice().is_empty()); // N is explicitly 4
}

#[test]
fn test_into_iter_clone() {
    fn iter_rev_equal<I: DoubleEndedIterator<Item = i32>>(it: I, slice: &[i32]) {
        let mut bump: Bump = Bump::new();
        let mut v = mut_bump_vec_rev![in &mut bump];
        v.extend(it.rev());
        assert_eq!(&v[..], slice);
    }
    let mut it = [1, 2, 3].into_iter();
    iter_rev_equal(it.clone(), &[1, 2, 3]);
    assert_eq!(it.next(), Some(1));
    let mut it = it.rev();
    iter_rev_equal(it.clone(), &[3, 2]);
    assert_eq!(it.next(), Some(3));
    iter_rev_equal(it.clone(), &[2]);
    assert_eq!(it.next(), Some(2));
    iter_rev_equal(it.clone(), &[]);
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

    let mut bump: Bump = Bump::new();
    let v = mut_bump_vec_rev![in &mut bump; D(false), D(true), D(false)];

    catch_unwind(AssertUnwindSafe(move || drop(v.into_iter()))).ok();

    assert_eq!(unsafe { DROPS }, 3);
}

#[test]
fn test_into_iter_advance_by() {
    let mut bump: Bump = Bump::new();
    let mut i = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5].into_iter();
    assert_eq!(i.advance_by(0), Ok(()));
    assert_eq!(i.advance_back_by(0), Ok(()));
    assert_eq!(i.as_slice(), [1, 2, 3, 4, 5]);

    assert_eq!(i.advance_by(1), Ok(()));
    assert_eq!(i.advance_back_by(1), Ok(()));
    assert_eq!(i.as_slice(), [2, 3, 4]);

    assert_eq!(i.advance_back_by(usize::MAX), Err(NonZeroUsize::new(usize::MAX - 3).unwrap()));

    assert_eq!(i.advance_by(usize::MAX), Err(NonZeroUsize::new(usize::MAX).unwrap()));

    assert_eq!(i.advance_by(0), Ok(()));
    assert_eq!(i.advance_back_by(0), Ok(()));

    assert_eq!(i.len(), 0);
}

#[test]
fn test_drop_allocator() {
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

    let mut bump: Bump = Bump::new();
    for _ in mut_bump_vec_rev![in &mut bump; C].into_iter() {}
    for _ in mut_bump_vec_rev![in &mut bump; C; 5].into_iter().rev() {}

    let mut it = mut_bump_vec_rev![in &mut bump; C, C].into_iter();
    assert_eq!(it.advance_by(1), Ok(()));
    drop(it);

    let mut it = mut_bump_vec_rev![in &mut bump; C, C].into_iter();
    it.next_chunk::<1>().unwrap();
    drop(it);

    let mut it = mut_bump_vec_rev![in &mut bump; C, C].into_iter();
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
    let mut bump: Bump = Bump::new();

    #[repr(align(256))]
    struct Foo(usize);
    let mut v = mut_bump_vec_rev![in &mut bump; Foo(273)];
    for i in 0..0x1000 {
        v.reserve(i);
        assert!(v[0].0 == 273);
        assert!(v.as_ptr() as usize & 0xff == 0);
    }
}

macro_rules! generate_assert_eq_vec_and_prim {
    ($name:ident<$B:ident>($type:ty)) => {
        fn $name<A: PartialEq<$B> + Debug, $B: Debug>(a: MutBumpVecRev<A, &mut Bump>, b: $type) {
            assert!(a == b);
            assert_eq!(a, b);
        }
    };
}

generate_assert_eq_vec_and_prim! { assert_eq_vec_and_slice  <B>(&[B])   }
generate_assert_eq_vec_and_prim! { assert_eq_vec_and_array_3<B>([B; 3]) }

#[test]
fn partialeq_vec_and_prim() {
    let mut bump: Bump = Bump::new();
    assert_eq_vec_and_slice(mut_bump_vec_rev![in &mut bump; 1, 2, 3], &[1, 2, 3]);
    assert_eq_vec_and_array_3(mut_bump_vec_rev![in &mut bump; 1, 2, 3], [1, 2, 3]);
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
    let mut bump2: Bump = Bump::new();
    let mut bump3: Bump = Bump::new();
    let vec2: MutBumpVecRev<_, _> = mut_bump_vec_rev![in &mut bump2; 1, 2];
    let vec3: MutBumpVecRev<_, _> = mut_bump_vec_rev![in &mut bump3; 1, 2, 3];
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
        let mut bump: Bump = Bump::new();
        let v = MutBumpVecRev::<(), _>::with_capacity_in(len, &mut bump);
        assert_eq!(v.len(), 0);
        assert_eq!(v.capacity(), usize::MAX);
    }
}

#[test]
fn test_zero_sized_vec_push() {
    const N: usize = 8;

    for len in 0..N {
        let mut bump: Bump = Bump::new();
        let mut tester = MutBumpVecRev::with_capacity_in(len, &mut bump);
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
    let mut bump0: Bump = Bump::new();
    let mut bump1: Bump = Bump::new();

    assert_eq!(
        mut_bump_vec_rev![in &mut bump0; 1; 3],
        mut_bump_vec_rev![in &mut bump1; 1, 1, 1]
    );
    assert_eq!(mut_bump_vec_rev![in &mut bump0; 1; 2], mut_bump_vec_rev![in &mut bump1; 1, 1]);
    assert_eq!(mut_bump_vec_rev![in &mut bump0; 1; 1], mut_bump_vec_rev![in &mut bump1; 1]);
    // assert_eq!(mut_bump_vec_rev![in &mut bump0; 1; 0], mut_bump_vec_rev![in &mut bump1; ]);

    // from_elem syntax (see RFC 832)
    let el = Box::new(1);
    let n = 3;
    assert_eq!(
        mut_bump_vec_rev![in &mut bump0; el; n],
        mut_bump_vec_rev![in &mut bump1; Box::new(1), Box::new(1), Box::new(1)]
    );
}

#[test]
fn test_vec_swap() {
    let mut bump: Bump = Bump::new();
    let mut a: MutBumpVecRev<isize, _> = mut_bump_vec_rev![in &mut bump; 0, 1, 2, 3, 4, 5, 6];
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

    let mut bump: Bump = Bump::new();
    mut_bump_vec_rev![in &mut bump; CopyOnly, CopyOnly].extend_from_within_copy(..);
}

#[test]
fn test_extend_from_within_clone() {
    let mut bump: Bump = Bump::new();
    let mut v = mut_bump_vec_rev![in &mut bump; String::from("sssss"), String::from("12334567890"), String::from("c")];
    v.extend_from_within_clone(1..);
    assert_eq!(v, ["12334567890", "c", "sssss", "12334567890", "c"]);
}

#[test]
fn test_extend_from_within_complete_rande() {
    {
        let mut bump: Bump = Bump::new();
        let mut v = mut_bump_vec_rev![in &mut bump; 0, 1, 2, 3];
        v.extend_from_within_copy(..);
        assert_eq!(v, [0, 1, 2, 3, 0, 1, 2, 3]);
    }
    {
        let mut bump: Bump = Bump::new();
        let mut v = mut_bump_vec_rev![in &mut bump; 0, 1, 2, 3];
        v.extend_from_within_clone(..);
        assert_eq!(v, [0, 1, 2, 3, 0, 1, 2, 3]);
    }
}

#[test]
fn test_extend_from_within_empty_rande() {
    {
        let mut bump: Bump = Bump::new();
        let mut v = mut_bump_vec_rev![in &mut bump; 0, 1, 2, 3];
        v.extend_from_within_copy(1..1);
        assert_eq!(v, [0, 1, 2, 3]);
    }
    {
        let mut bump: Bump = Bump::new();
        let mut v = mut_bump_vec_rev![in &mut bump; 0, 1, 2, 3];
        v.extend_from_within_clone(1..1);
        assert_eq!(v, [0, 1, 2, 3]);
    }
}

#[test]
#[should_panic]
fn test_extend_from_within_out_of_rande() {
    {
        let mut bump: Bump = Bump::new();
        let mut v = mut_bump_vec_rev![in &mut bump; 0, 1];
        v.extend_from_within_copy(..3);
    }

    {
        let mut bump: Bump = Bump::new();
        let mut v = mut_bump_vec_rev![in &mut bump; 0, 1];
        v.extend_from_within_clone(..3);
    }
}

#[test]
fn test_extend_from_within_zst() {
    {
        let mut bump: Bump = Bump::new();
        let mut v = mut_bump_vec_rev![in &mut bump; (); 8];
        v.extend_from_within_copy(3..7);
        assert_eq!(v, [(); 12]);
    }
    {
        let mut bump: Bump = Bump::new();
        let mut v = mut_bump_vec_rev![in &mut bump; (); 8];
        v.extend_from_within_clone(3..7);
        assert_eq!(v, [(); 12]);
    }
}

#[test]
fn test_extend_from_within_empty_vec() {
    {
        let mut bump: Bump = Bump::new();
        let mut v = MutBumpVecRev::<i32, _>::new_in(&mut bump);
        v.extend_from_within_copy(..);
        assert!(v.is_empty());
    }
    {
        let mut bump: Bump = Bump::new();
        let mut v = MutBumpVecRev::<i32, _>::new_in(&mut bump);
        v.extend_from_within_clone(..);
        assert!(v.is_empty());
    }
}

#[test]
fn test_extend_from_within() {
    let mut bump: Bump = Bump::new();
    let mut v = mut_bump_vec_rev![in &mut bump; String::from("a"), String::from("b"), String::from("c")];
    v.extend_from_within_clone(1..=2);
    v.extend_from_within_clone(..=1);
    assert_eq!(v, ["b", "c", "b", "c", "a", "b", "c"]);
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
    let mut bump: Bump = Bump::new();

    let mut vec = mut_bump_vec_rev![in &mut bump;
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
    std::panic::catch_unwind(AssertUnwindSafe(move || vec.extend_from_within_clone(..))).unwrap_err();

    assert_eq!(count.load(Ordering::SeqCst), 4);
}

#[test]
#[should_panic = "vec len overflow"]
fn test_into_flattened_size_overflow() {
    let mut bump: Bump = Bump::new();
    let v = mut_bump_vec_rev![in &mut bump; [(); usize::MAX]; 2];
    let _ = v.into_flattened();
}
