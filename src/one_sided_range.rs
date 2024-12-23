// taken straight from nightly

use core::ops::{Bound, RangeBounds, RangeFrom, RangeTo, RangeToInclusive};

use crate::polyfill::slice::{slice_end_index_overflow_fail, slice_start_index_overflow_fail};

/// `OneSidedRange` is implemented for built-in range types that are unbounded
/// on one side. For example, `a..`, `..b` and `..=c` implement `OneSidedRange`,
/// but `..`, `d..e`, and `f..=g` do not.
///
/// Types that implement `OneSidedRange<T>` must return `Bound::Unbounded`
/// from one of `RangeBounds::start_bound` or `RangeBounds::end_bound`.
pub trait OneSidedRange<T: ?Sized>: RangeBounds<T> {}

impl<T> OneSidedRange<T> for RangeTo<T> where Self: RangeBounds<T> {}

impl<T> OneSidedRange<T> for RangeFrom<T> where Self: RangeBounds<T> {}

impl<T> OneSidedRange<T> for RangeToInclusive<T> where Self: RangeBounds<T> {}

pub(crate) fn direction(range: impl OneSidedRange<usize>, bounds: RangeTo<usize>) -> (Direction, usize) {
    let len = bounds.end;

    let (direction, index) = match (range.start_bound(), range.end_bound()) {
        (Bound::Included(&start), Bound::Unbounded) => (Direction::From, start),
        (Bound::Excluded(&start), Bound::Unbounded) => (
            Direction::From,
            start.checked_add(1).unwrap_or_else(|| slice_start_index_overflow_fail()),
        ),
        (Bound::Unbounded, Bound::Included(&end)) => (
            Direction::To,
            end.checked_add(1).unwrap_or_else(|| slice_end_index_overflow_fail()),
        ),
        (Bound::Unbounded, Bound::Excluded(&end)) => (Direction::To, end),
        _ => unreachable!("`OneSidedRange` is not a one sided range"),
    };

    if index > len {
        slice_index_len_fail(index, len);
    }

    (direction, index)
}

#[cold]
#[inline(never)]
#[track_caller]
pub fn slice_index_len_fail(index: usize, len: usize) -> ! {
    panic!("range index {index} out of range for slice of length {len}")
}

pub(crate) enum Direction {
    From,
    To,
}
