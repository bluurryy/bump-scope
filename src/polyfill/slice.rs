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

/// See [`std::slice::range`].
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

/// See [`slice::split_at_unchecked`].
#[inline]
#[must_use]
pub(crate) unsafe fn split_at_unchecked<T>(slice: &[T], mid: usize) -> (&[T], &[T]) {
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
