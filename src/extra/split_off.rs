use core::{
    ops::{self, RangeBounds},
    ptr,
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    BumpAllocator, SizedTypeProperties,
    extra::{
        assert_char_boundary, box_empty_slice_in, box_empty_str_in, box_set_len, box_set_ptr, ptr_from_utf8,
        vec_replace_buffer,
    },
    polyfill,
};

/// Splits the vector, boxed slice or boxed string into two by removing the specified range.
///
/// The allocator must satisfy `BumpAllocator + Clone`.
///
/// Unlike <code>Vec::[split_off](alloc::vec::Vec::split_off)</code> this method
/// **does not allocate** and does not change the order of the elements.
///
/// The excess capacity may end up in either vector.
///
/// # Panics
/// Panics if the starting point is greater than the end point or if the end point is greater than the length of the vector.
///
/// Panics for strings if the starting point or end point does not lie on a [`char`] boundary.
///
/// # Complexity
/// This operation takes `O(1)` time if either the range starts at 0, ends at `len`, or is empty.
/// Otherwise it takes `O(min(end, len - start))` time.
///
/// # Examples
///
/// A Vector:
/// ```
/// use bump_scope::{Bump, extra::split_off};
/// let bump: Bump = Bump::new();
/// let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
/// vec.append([1, 2, 3, 4, 5, 6, 7, 8]);
///
/// let front = vec.split_off(..2);
/// assert_eq!(front, [1, 2]);
/// assert_eq!(vec, [3, 4, 5, 6, 7, 8]);
///
/// let back = vec.split_off(4..);
/// assert_eq!(back, [7, 8]);
/// assert_eq!(vec, [3, 4, 5, 6]);
///
/// let middle = vec.split_off(1..3);
/// assert_eq!(middle, [4, 5]);
/// assert_eq!(vec, [3, 6]);
///
/// let rest = vec.split_off(..);
/// assert_eq!(rest, [3, 6]);
/// assert_eq!(vec, []);
/// ```
///
/// A boxed slice:
/// ```
/// # use bump_scope::Bump;
/// # let bump: Bump = Bump::new();
/// let mut slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6, 7, 8]);
///
/// let front = slice.split_off(..2);
/// assert_eq!(front, [1, 2]);
/// assert_eq!(slice, [3, 4, 5, 6, 7, 8]);
///
/// let back = slice.split_off(4..);
/// assert_eq!(back, [7, 8]);
/// assert_eq!(slice, [3, 4, 5, 6]);
///
/// let middle = slice.split_off(1..3);
/// assert_eq!(middle, [4, 5]);
/// assert_eq!(slice, [3, 6]);
///
/// let rest = slice.split_off(..);
/// assert_eq!(rest, [3, 6]);
/// assert_eq!(slice, []);
/// ```
///
/// A boxed str:
/// ```
/// use bump_scope::{Bump, extra::box_str_split_off};
///
/// let bump: Bump = Bump::new();
///
/// let mut string = bump.alloc_str("foobarbazqux");
///
/// let foo = box_str_split_off(string, ..3);
/// assert_eq!(foo, "foo");
/// assert_eq!(string, "barbazqux");
///
/// let qux = box_str_split_off(string, 6..);
/// assert_eq!(qux, "qux");
/// assert_eq!(string, "barbaz");
///
/// let rb = box_str_split_off(string, 2..4);
/// assert_eq!(rb, "rb");
/// assert_eq!(string, "baaz");
///
/// let rest = box_str_split_off(string, ..);
/// assert_eq!(rest, "baaz");
/// assert_eq!(string, "");
/// ```
#[expect(private_bounds)]
pub fn split_off<VecOrBox>(vec_or_box: &mut VecOrBox, range: impl RangeBounds<usize>) -> VecOrBox
where
    VecOrBox: SplitOff,
{
    SplitOff::split_off(vec_or_box, range)
}

trait SplitOff {
    fn split_off(this: &mut Self, range: impl RangeBounds<usize>) -> Self;
}

impl<T, A> SplitOff for Vec<T, A>
where
    A: BumpAllocator + Clone,
{
    #[inline]
    #[expect(clippy::return_self_not_must_use)]
    fn split_off(vec: &mut Vec<T, A>, range: impl RangeBounds<usize>) -> Vec<T, A> {
        let len = vec.len();
        let ops::Range { start, end } = polyfill::slice::range(range, ..len);
        let alloc = vec.allocator().clone();
        let ptr = vec.as_mut_ptr();

        unsafe {
            if T::IS_ZST {
                let range_len = end - start;
                let remaining_len = len - range_len;

                vec.set_len(remaining_len);
                // TODO: what capacity do we provide for zst? does it even matter?
                return Vec::from_raw_parts_in(ptr::dangling_mut(), range_len, range_len, alloc);
            }

            if end == len {
                let lhs = ptr;
                let rhs = ptr.add(start);

                let lhs_len = start;
                let rhs_len = len - start;

                let lhs_cap = start;
                let rhs_cap = vec.capacity() - lhs_cap;

                vec_replace_buffer(vec, lhs, lhs_len, lhs_cap);
                return Vec::from_raw_parts_in(rhs, rhs_len, rhs_cap, alloc);
            }

            if start == 0 {
                let lhs = ptr;
                let rhs = ptr.add(end);

                let lhs_len = end;
                let rhs_len = len - end;

                let lhs_cap = end;
                let rhs_cap = vec.capacity() - lhs_cap;

                vec_replace_buffer(vec, rhs, rhs_len, rhs_cap);
                return Vec::from_raw_parts_in(lhs, lhs_len, lhs_cap, alloc);
            }

            if start == end {
                return Vec::new_in(alloc);
            }

            let head_len = start;
            let tail_len = len - end;

            let range_len = end - start;
            let remaining_len = len - range_len;

            if head_len < tail_len {
                // move the range of elements to split off to the start
                vec.as_mut_slice().get_unchecked_mut(..end).rotate_right(range_len);

                let lhs = ptr;
                let rhs = ptr.add(range_len);

                let lhs_len = range_len;
                let rhs_len = remaining_len;

                let lhs_cap = range_len;
                let rhs_cap = vec.capacity() - lhs_cap;

                vec_replace_buffer(vec, rhs, rhs_len, rhs_cap);
                return Vec::from_raw_parts_in(lhs, lhs_len, lhs_cap, alloc);
            } else {
                // move the range of elements to split off to the end
                vec.as_mut_slice().get_unchecked_mut(start..).rotate_left(range_len);

                let lhs = ptr;
                let rhs = ptr.add(remaining_len);

                let lhs_len = remaining_len;
                let rhs_len = range_len;

                let lhs_cap = remaining_len;
                let rhs_cap = vec.capacity() - lhs_cap;

                vec_replace_buffer(vec, lhs, lhs_len, lhs_cap);
                return Vec::from_raw_parts_in(rhs, rhs_len, rhs_cap, alloc);
            }
        }
    }
}

impl<T, A> SplitOff for Box<[T], A>
where
    A: BumpAllocator + Clone,
{
    #[inline]
    fn split_off(boxed: &mut Box<[T], A>, range: impl RangeBounds<usize>) -> Box<[T], A> {
        let len = boxed.len();
        let ops::Range { start, end } = polyfill::slice::range(range, ..len);
        let alloc = Box::allocator(boxed).clone();
        let ptr = boxed.as_mut_ptr().cast::<T>();

        if T::IS_ZST {
            let range_len = end - start;
            let remaining_len = len - range_len;

            unsafe {
                box_set_len(boxed, remaining_len);
                return Box::from_raw_in(ptr::slice_from_raw_parts_mut(ptr::dangling_mut(), range_len), alloc);
            }
        }

        if end == len {
            unsafe {
                let lhs = ptr::slice_from_raw_parts_mut(ptr, start);
                let rhs = ptr::slice_from_raw_parts_mut(ptr.add(start), len - start);

                box_set_ptr(boxed, lhs);
                return Box::from_raw_in(rhs, alloc);
            }
        }

        if start == 0 {
            unsafe {
                let lhs = ptr::slice_from_raw_parts_mut(ptr, end);
                let rhs = ptr::slice_from_raw_parts_mut(ptr.add(end), len - end);

                box_set_ptr(boxed, rhs);
                return Box::from_raw_in(lhs, alloc);
            }
        }

        if start == end {
            return box_empty_slice_in(alloc);
        }

        let head_len = start;
        let tail_len = len - end;

        let range_len = end - start;
        let remaining_len = len - range_len;

        unsafe {
            if head_len < tail_len {
                // move the range of elements to split off to the start
                (&mut **boxed).get_unchecked_mut(..end).rotate_right(range_len);

                let lhs = ptr::slice_from_raw_parts_mut(ptr, range_len);
                let rhs = ptr::slice_from_raw_parts_mut(ptr.add(range_len), remaining_len);

                box_set_ptr(boxed, rhs);
                Box::from_raw_in(lhs, alloc)
            } else {
                // move the range of elements to split off to the end
                (&mut **boxed).get_unchecked_mut(start..).rotate_left(range_len);

                let lhs = ptr::slice_from_raw_parts_mut(ptr, remaining_len);
                let rhs = ptr::slice_from_raw_parts_mut(ptr.add(remaining_len), range_len);

                box_set_ptr(boxed, lhs);
                Box::from_raw_in(rhs, alloc)
            }
        }
    }
}

impl<A> SplitOff for Box<str, A>
where
    A: BumpAllocator + Clone,
{
    #[inline]
    fn split_off(boxed: &mut Box<str, A>, range: impl RangeBounds<usize>) -> Box<str, A> {
        let len = boxed.len();
        let ops::Range { start, end } = polyfill::slice::range(range, ..len);
        let alloc = Box::allocator(boxed).clone();
        let ptr = boxed.as_mut_ptr().cast::<u8>();

        if end == len {
            assert_char_boundary(boxed, start);

            unsafe {
                let lhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr, start));
                let rhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr.add(start), len - start));

                box_set_ptr(boxed, lhs);
                return Box::from_raw_in(rhs, alloc);
            }
        }

        if start == 0 {
            assert_char_boundary(boxed, end);

            unsafe {
                let lhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr, end));
                let rhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr.add(end), len - end));

                box_set_ptr(boxed, rhs);
                return Box::from_raw_in(lhs, alloc);
            }
        }

        if start == end {
            return box_empty_str_in(alloc);
        }

        assert_char_boundary(boxed, start);
        assert_char_boundary(boxed, end);

        let head_len = start;
        let tail_len = len - end;

        let range_len = end - start;
        let remaining_len = len - range_len;

        unsafe {
            if head_len < tail_len {
                // move the range of elements to split off to the start
                boxed.as_bytes_mut().get_unchecked_mut(..end).rotate_right(range_len);

                let lhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr, range_len));
                let rhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr.add(range_len), remaining_len));

                box_set_ptr(boxed, rhs);
                Box::from_raw_in(lhs, alloc)
            } else {
                // move the range of elements to split off to the end
                boxed.as_bytes_mut().get_unchecked_mut(start..).rotate_left(range_len);

                let lhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr, remaining_len));
                let rhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr.add(remaining_len), range_len));

                box_set_ptr(boxed, lhs);
                Box::from_raw_in(rhs, alloc)
            }
        }
    }
}
