use core::ops;
pub use core::slice::*;

#[cold]
#[inline(never)]
#[track_caller]
const fn slice_start_index_overflow_fail() -> ! {
    panic!("attempted to index slice from after maximum usize");
}

#[cold]
#[inline(never)]
#[track_caller]
const fn slice_end_index_overflow_fail() -> ! {
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
/// ```
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
/// ```should_panic
/// #![feature(slice_range)]
///
/// use std::slice;
///
/// let _ = slice::range(2..1, ..3);
/// ```
///
/// ```should_panic
/// #![feature(slice_range)]
///
/// use std::slice;
///
/// let _ = slice::range(1..4, ..3);
/// ```
///
/// ```should_panic
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
pub fn range<R>(range: R, bounds: ops::RangeTo<usize>) -> ops::Range<usize>
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
