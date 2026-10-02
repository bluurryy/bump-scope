use core::ops;

pub(crate) use core::slice::*;

/// See [`std::slice::range`].
#[track_caller]
#[must_use]
pub(crate) fn range<R>(range: R, bounds: ops::RangeTo<usize>) -> ops::Range<usize>
where
    R: ops::RangeBounds<usize>,
{
    let len = bounds.end;
    into_slice_range(len, (range.start_bound().map(|x| *x), range.end_bound().map(|x| *x)))
}

/// Converts pair of `ops::Bound`s into `ops::Range`.
/// Panics on overflowing indices.
#[inline]
pub(crate) fn into_slice_range(len: usize, (start, end): (ops::Bound<usize>, ops::Bound<usize>)) -> ops::Range<usize> {
    let end = match end {
        ops::Bound::Included(end) if end >= len => slice_index_fail(0, end, len),
        // Cannot overflow because `end < len` implies `end < usize::MAX`.
        ops::Bound::Included(end) => end + 1,

        ops::Bound::Excluded(end) if end > len => slice_index_fail(0, end, len),
        ops::Bound::Excluded(end) => end,

        ops::Bound::Unbounded => len,
    };

    let start = match start {
        ops::Bound::Excluded(start) if start >= end => slice_index_fail(start, end, len),
        // Cannot overflow because `start < end` implies `start < usize::MAX`.
        ops::Bound::Excluded(start) => start + 1,

        ops::Bound::Included(start) if start > end => slice_index_fail(start, end, len),
        ops::Bound::Included(start) => start,

        ops::Bound::Unbounded => 0,
    };

    start..end
}

#[cfg_attr(not(panic = "immediate-abort"), inline(never), cold)]
#[cfg_attr(panic = "immediate-abort", inline)]
#[track_caller]
fn slice_index_fail(start: usize, end: usize, len: usize) -> ! {
    if start > len {
        panic!("range start index {start} out of range for slice of length {len}");
    }

    if end > len {
        panic!("range end index {end} out of range for slice of length {len}");
    }

    if start > end {
        panic!("slice index starts at {start} but ends at {end}");
    }

    // Only reachable if the range was a `RangeInclusive` or a
    // `RangeToInclusive`, with `end == len`.
    panic!("range end index {end} out of range for slice of length {len}");
}
