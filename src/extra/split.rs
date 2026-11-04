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

/// Splits the vector in two by removing the specified range.
///
/// Unlike <code>Vec::[split_off](alloc::vec::Vec::split_off)</code> this method
/// **does not allocate** and does not change the order of the elements.
///
/// The excess capacity may end up in either vector.
///
/// # Panics
/// Panics if the starting point is greater than the end point or if the end point is greater than the length of the vector.
///
/// # Complexity
/// This operation takes `O(1)` time if either the range starts at 0, ends at `len`, or is empty.
/// Otherwise it takes `O(min(end, len - start))` time.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, extra::vec_split_off};
/// let bump: Bump = Bump::new();
///
/// let mut vec = Vec::new_in(&bump);
/// vec.extend([1, 2, 3, 4, 5, 6, 7, 8]);
///
/// let front = vec_split_off(&mut vec, ..2);
/// assert_eq!(&*front, [1, 2]);
/// assert_eq!(&*vec, [3, 4, 5, 6, 7, 8]);
///
/// let back = vec_split_off(&mut vec, 4..);
/// assert_eq!(&*back, [7, 8]);
/// assert_eq!(&*vec, [3, 4, 5, 6]);
///
/// let middle = vec_split_off(&mut vec, 1..3);
/// assert_eq!(&*middle, [4, 5]);
/// assert_eq!(&*vec, [3, 6]);
///
/// let rest = vec_split_off(&mut vec, ..);
/// assert_eq!(&*rest, [3, 6]);
/// assert_eq!(&*vec, []);
/// ```
#[inline]
pub fn vec_split_off<T, A>(vec: &mut Vec<T, A>, range: impl RangeBounds<usize>) -> Vec<T, A>
where
    A: BumpAllocator + Clone,
{
    let len = vec.len();
    let ops::Range { start, end } = polyfill::slice::range(range, ..len);
    let alloc = vec.allocator().clone();

    unsafe {
        if T::IS_ZST {
            let range_len = end - start;
            let remaining_len = len - range_len;

            vec.set_len(remaining_len);
            // TODO: what capacity do we provide for zst? does it even matter?
            return Vec::from_raw_parts_in(ptr::dangling_mut(), range_len, range_len, alloc);
        }

        if end == len {
            let ptr = vec.as_mut_ptr();

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
            let ptr = vec.as_mut_ptr();

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
            vec.get_unchecked_mut(..end).rotate_right(range_len);

            let ptr = vec.as_mut_ptr();

            let lhs = ptr;
            let rhs = ptr.add(range_len);

            let lhs_len = range_len;
            let rhs_len = remaining_len;

            let lhs_cap = range_len;
            let rhs_cap = vec.capacity() - lhs_cap;

            vec_replace_buffer(vec, rhs, rhs_len, rhs_cap);
            Vec::from_raw_parts_in(lhs, lhs_len, lhs_cap, alloc)
        } else {
            // move the range of elements to split off to the end
            vec.get_unchecked_mut(start..).rotate_left(range_len);

            let ptr = vec.as_mut_ptr();

            let lhs = ptr;
            let rhs = ptr.add(remaining_len);

            let lhs_len = remaining_len;
            let rhs_len = range_len;

            let lhs_cap = remaining_len;
            let rhs_cap = vec.capacity() - lhs_cap;

            vec_replace_buffer(vec, lhs, lhs_len, lhs_cap);
            Vec::from_raw_parts_in(rhs, rhs_len, rhs_cap, alloc)
        }
    }
}

/// Splits the boxed slice in two by removing the specified range.
///
/// Unlike <code>Vec::[split_off](alloc::vec::Vec::split_off)</code> this method
/// **does not allocate** and does not change the order of the elements.
///
/// # Panics
/// Panics if the starting point is greater than the end point or if the end point is greater than the length of the vector.
///
/// # Complexity
/// This operation takes `O(1)` time if either the range starts at 0, ends at `len`, or is empty.
/// Otherwise it takes `O(min(end, len - start))` time.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::boxed_slice_split_off};
/// let bump: Bump = Bump::new();
///
/// let mut slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6, 7, 8]).into_boxu();
///
/// let front = boxed_slice_split_off(&mut slice, ..2);
/// assert_eq!(&*front, [1, 2]);
/// assert_eq!(&*slice, [3, 4, 5, 6, 7, 8]);
///
/// let back = boxed_slice_split_off(&mut slice, 4..);
/// assert_eq!(&*back, [7, 8]);
/// assert_eq!(&*slice, [3, 4, 5, 6]);
///
/// let middle = boxed_slice_split_off(&mut slice, 1..3);
/// assert_eq!(&*middle, [4, 5]);
/// assert_eq!(&*slice, [3, 6]);
///
/// let rest = boxed_slice_split_off(&mut slice, ..);
/// assert_eq!(&*rest, [3, 6]);
/// assert_eq!(&*slice, []);
/// ```
#[inline]
pub fn boxed_slice_split_off<T, A>(boxed: &mut Box<[T], A>, range: impl RangeBounds<usize>) -> Box<[T], A>
where
    A: BumpAllocator + Clone,
{
    let len = boxed.len();
    let ops::Range { start, end } = polyfill::slice::range(range, ..len);
    let alloc = Box::allocator(boxed).clone();

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
            let ptr = boxed.as_mut_ptr().cast::<T>();
            let lhs = ptr::slice_from_raw_parts_mut(ptr, start);
            let rhs = ptr::slice_from_raw_parts_mut(ptr.add(start), len - start);

            box_set_ptr(boxed, lhs);
            return Box::from_raw_in(rhs, alloc);
        }
    }

    if start == 0 {
        unsafe {
            let ptr = boxed.as_mut_ptr().cast::<T>();
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
            boxed.get_unchecked_mut(..end).rotate_right(range_len);

            let ptr = boxed.as_mut_ptr().cast::<T>();
            let lhs = ptr::slice_from_raw_parts_mut(ptr, range_len);
            let rhs = ptr::slice_from_raw_parts_mut(ptr.add(range_len), remaining_len);

            box_set_ptr(boxed, rhs);
            Box::from_raw_in(lhs, alloc)
        } else {
            // move the range of elements to split off to the end
            boxed.get_unchecked_mut(start..).rotate_left(range_len);

            let ptr = boxed.as_mut_ptr().cast::<T>();
            let lhs = ptr::slice_from_raw_parts_mut(ptr, remaining_len);
            let rhs = ptr::slice_from_raw_parts_mut(ptr.add(remaining_len), range_len);

            box_set_ptr(boxed, lhs);
            Box::from_raw_in(rhs, alloc)
        }
    }
}

/// Splits the boxed string in two by removing the specified range.
///
/// Unlike <code>Vec::[split_off](alloc::vec::Vec::split_off)</code> this method
/// **does not allocate** and does not change the order of the elements.
///
/// # Panics
/// Panics if the starting point is greater than the end point or if the end point is greater than the length of the vector.
///
/// Panics if the starting point or end point does not lie on a [`char`] boundary.
///
/// # Complexity
/// This operation takes `O(1)` time if either the range starts at 0, ends at `len`, or is empty.
/// Otherwise it takes `O(min(end, len - start))` time.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::boxed_str_split_off};
/// let bump: Bump = Bump::new();
///
/// let mut string = bump.alloc_str("foobarbazqux").into_boxu();
///
/// let foo = boxed_str_split_off(&mut string, ..3);
/// assert_eq!(&*foo, "foo");
/// assert_eq!(&*string, "barbazqux");
///
/// let qux = boxed_str_split_off(&mut string, 6..);
/// assert_eq!(&*qux, "qux");
/// assert_eq!(&*string, "barbaz");
///
/// let rb = boxed_str_split_off(&mut string, 2..4);
/// assert_eq!(&*rb, "rb");
/// assert_eq!(&*string, "baaz");
///
/// let rest = boxed_str_split_off(&mut string, ..);
/// assert_eq!(&*rest, "baaz");
/// assert_eq!(&*string, "");
/// ```
#[inline]
pub fn boxed_str_split_off<A>(boxed: &mut Box<str, A>, range: impl RangeBounds<usize>) -> Box<str, A>
where
    A: BumpAllocator + Clone,
{
    let len = boxed.len();
    let ops::Range { start, end } = polyfill::slice::range(range, ..len);
    let alloc = Box::allocator(boxed).clone();

    if end == len {
        assert_char_boundary(boxed, start);

        unsafe {
            let ptr = boxed.as_mut_ptr().cast::<u8>();
            let lhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr, start));
            let rhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr.add(start), len - start));

            box_set_ptr(boxed, lhs);
            return Box::from_raw_in(rhs, alloc);
        }
    }

    if start == 0 {
        assert_char_boundary(boxed, end);

        unsafe {
            let ptr = boxed.as_mut_ptr().cast::<u8>();
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

            let ptr = boxed.as_mut_ptr().cast::<u8>();
            let lhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr, range_len));
            let rhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr.add(range_len), remaining_len));

            box_set_ptr(boxed, rhs);
            Box::from_raw_in(lhs, alloc)
        } else {
            // move the range of elements to split off to the end
            boxed.as_bytes_mut().get_unchecked_mut(start..).rotate_left(range_len);

            let ptr = boxed.as_mut_ptr().cast::<u8>();
            let lhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr, remaining_len));
            let rhs = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr.add(remaining_len), range_len));

            box_set_ptr(boxed, lhs);
            Box::from_raw_in(rhs, alloc)
        }
    }
}
