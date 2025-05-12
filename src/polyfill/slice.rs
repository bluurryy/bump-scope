use core::ops;

pub(crate) use core::slice::*;

use crate::polyfill::usize::unchecked_sub;

#[cold]
#[inline(never)]
#[track_caller]
pub(crate) const fn slice_start_index_overflow_fail() -> ! {
    panic!("attempted to index slice from after maximum usize");
}

#[cold]
#[inline(never)]
#[track_caller]
pub(crate) const fn slice_end_index_overflow_fail() -> ! {
    panic!("attempted to index slice up to maximum usize");
}

#[cold]
#[inline(never)]
#[track_caller]
fn slice_index_order_fail(index: usize, end: usize) -> ! {
    panic!("slice index starts at {index} but ends at {end}");
}

#[cold]
#[inline(never)]
#[track_caller]
fn slice_end_index_len_fail(index: usize, len: usize) -> ! {
    panic!("range end index {index} out of range for slice of length {len}")
}

/// Performs bounds-checking of a range.
///
/// This method is similar to [`Index::index`] for slices, but it returns a
/// [`Range`] equivalent to `range`. You can use this method to turn any range
/// into `start` and `end` values.
///
/// `bounds` is the range of the slice to use for bounds-checking. It should
/// be a [`RangeTo`] range that ends at the length of the slice.
///
/// The returned [`Range`] is safe to pass to [`slice::get_unchecked`] and
/// [`slice::get_unchecked_mut`] for slices with the given range.
///
/// [`Range`]: ops::Range
/// [`RangeTo`]: ops::RangeTo
/// [`slice::get_unchecked`]: slice::get_unchecked
/// [`slice::get_unchecked_mut`]: slice::get_unchecked_mut
///
/// # Panics
///
/// Panics if `range` would be out of bounds.
///
/// # Examples
///
/// ```ignore
/// #![feature(slice_range)]
///
/// use std::slice;
///
/// let v = [10, 40, 30];
/// assert_eq!(1..2, slice::range(1..2, ..v.len()));
/// assert_eq!(0..2, slice::range(..2, ..v.len()));
/// assert_eq!(1..3, slice::range(1.., ..v.len()));
/// ```
///
/// Panics when [`Index::index`] would panic:
///
/// ```ignore
/// #![feature(slice_range)]
///
/// use std::slice;
///
/// let _ = slice::range(2..1, ..3);
/// ```
///
/// ```ignore
/// #![feature(slice_range)]
///
/// use std::slice;
///
/// let _ = slice::range(1..4, ..3);
/// ```
///
/// ```ignore
/// #![feature(slice_range)]
///
/// use std::slice;
///
/// let _ = slice::range(1..=usize::MAX, ..3);
/// ```
///
/// [`Index::index`]: ops::Index::index
#[track_caller]
#[must_use]
pub(crate) fn range<R>(range: R, bounds: ops::RangeTo<usize>) -> ops::Range<usize>
where
    R: ops::RangeBounds<usize>,
{
    let len = bounds.end;

    let start: ops::Bound<&usize> = range.start_bound();
    let start = match start {
        ops::Bound::Included(&start) => start,
        ops::Bound::Excluded(start) => start.checked_add(1).unwrap_or_else(|| slice_start_index_overflow_fail()),
        ops::Bound::Unbounded => 0,
    };

    let end: ops::Bound<&usize> = range.end_bound();
    let end = match end {
        ops::Bound::Included(end) => end.checked_add(1).unwrap_or_else(|| slice_end_index_overflow_fail()),
        ops::Bound::Excluded(&end) => end,
        ops::Bound::Unbounded => len,
    };

    if start > end {
        slice_index_order_fail(start, end);
    }
    if end > len {
        slice_end_index_len_fail(end, len);
    }

    ops::Range { start, end }
}

/// Divides one slice into two at an index, without doing bounds checking.
///
/// The first will contain all indices from `[0, mid)` (excluding
/// the index `mid` itself) and the second will contain all
/// indices from `[mid, len)` (excluding the index `len` itself).
///
/// For a safe alternative see [`split_at`].
///
/// # Safety
///
/// Calling this method with an out-of-bounds index is *[undefined behavior]*
/// even if the resulting reference is not used. The caller has to ensure that
/// `0 <= mid <= self.len()`.
///
/// [`split_at`]: slice::split_at
/// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
///
/// # Examples
///
/// ```
/// let v = [1, 2, 3, 4, 5, 6];
///
/// unsafe {
///    let (left, right) = v.split_at_unchecked(0);
///    assert_eq!(left, []);
///    assert_eq!(right, [1, 2, 3, 4, 5, 6]);
/// }
///
/// unsafe {
///     let (left, right) = v.split_at_unchecked(2);
///     assert_eq!(left, [1, 2]);
///     assert_eq!(right, [3, 4, 5, 6]);
/// }
///
/// unsafe {
///     let (left, right) = v.split_at_unchecked(6);
///     assert_eq!(left, [1, 2, 3, 4, 5, 6]);
///     assert_eq!(right, []);
/// }
/// ```
#[inline]
#[must_use]
pub unsafe fn split_at_unchecked<T>(slice: &[T], mid: usize) -> (&[T], &[T]) {
    // STD-FIXME(const-hack): the const function `from_raw_parts` is used to make this
    // function const; previously the implementation used
    // `(self.get_unchecked(..mid), self.get_unchecked(mid..))`

    let len = slice.len();
    let ptr = slice.as_ptr();

    debug_assert!(
        mid <= len,
        "slice::split_at_unchecked requires the index to be within the slice"
    );

    // SAFETY: Caller has to check that `0 <= mid <= self.len()`
    unsafe {
        (
            from_raw_parts(ptr, mid),
            from_raw_parts(ptr.add(mid), unchecked_sub(len, mid)),
        )
    }
}
