#![forbid(unsafe_code)]
#![allow(clippy::needless_pass_by_value)]
//! Make sure you sync this file `src/bumping.rs`
//! with `crates/fuzzing-support/src/from_bump_scope/bumping.rs`.
//!
//! This file intentionally doesn't import anything other than `core`
//! to make it easy to fuzz (see above) and debug.

use core::{alloc::Layout, num::NonZeroUsize, ops::Range};

#[cold]
#[inline(always)]
pub(crate) fn cold() {}

#[inline(always)]
pub(crate) fn unlikely(condition: bool) -> bool {
    if condition {
        cold();
    } else {
        // ...
    }

    condition
}

pub(crate) const MIN_CHUNK_ALIGN: usize = 16;

macro_rules! debug_assert_aligned {
    ($addr:expr, $align:expr) => {
        let addr = $addr;
        let align = $align;

        debug_assert!(align.is_power_of_two());
        let is_aligned = addr & (align - 1) == 0;

        debug_assert!(
            is_aligned,
            "expected `{}` ({}) to be aligned to `{}` ({})",
            stringify!($addr),
            addr,
            stringify!($align),
            align,
        );
    };
}

macro_rules! debug_assert_ge {
    ($lhs:expr, $rhs:expr) => {
        let lhs = $lhs;
        let rhs = $rhs;

        debug_assert!(
            lhs >= rhs,
            "expected `{}` ({}) to be greater or equal to `{}` ({})",
            stringify!($lhs),
            lhs,
            stringify!($rhs),
            rhs,
        )
    };
}

macro_rules! debug_assert_le {
    ($lhs:expr, $rhs:expr) => {
        let lhs = $lhs;
        let rhs = $rhs;

        debug_assert!(
            lhs <= rhs,
            "expected `{}` ({}) to be less than or equal to `{}` ({})",
            stringify!($lhs),
            lhs,
            stringify!($rhs),
            rhs,
        )
    };
}

/// Arguments for [`bump_up`], [`bump_down`], [`bump_prepare_up`] and [`bump_prepare_down`].
pub(crate) struct BumpProps {
    /// The start of the remaining free allocation region.
    ///
    /// This must not be zero.
    /// This must be less than or equal to [`end`](Self::end).
    pub(crate) start: usize,

    /// The end of the remaining free allocation region.
    ///
    /// This must not be zero.
    /// This must be greater or equal to [`start`](Self::start).
    pub(crate) end: usize,

    /// The minimum alignment of the bump allocator.
    ///
    /// This must be less or equal to [`MIN_CHUNK_ALIGN`].
    pub(crate) min_align: usize,

    /// The allocation layout.
    pub(crate) layout: Layout,

    /// Whether the allocation layout's alignment is known at compile time.
    ///
    /// This is an optimization hint. `false` is always valid.
    pub(crate) align_is_const: bool,

    /// Whether the allocation layout's size is known at compile time.
    ///
    /// This is an optimization hint. `false` is always valid.
    pub(crate) size_is_const: bool,

    /// Whether the allocation layout's size is a multiple of its alignment and that is known at compile time.
    ///
    /// This is an optimization hint. `false` is always valid.
    ///
    /// This must only be true if `layout.size() % layout.align()` is also true.
    pub(crate) size_is_multiple_of_align: bool,
}

impl BumpProps {
    #[inline(always)]
    fn debug_assert_valid(&self) {
        debug_assert_ne!(self.start, 0);
        debug_assert_ne!(self.end, 0);

        debug_assert!(self.min_align.is_power_of_two());
        debug_assert!(self.min_align <= MIN_CHUNK_ALIGN);

        if self.size_is_multiple_of_align {
            debug_assert_eq!(self.layout.size() % self.layout.align(), 0);
        }

        debug_assert_le!(self.start, self.end);
    }
}

/// A successful upwards bump allocation. Returned from [`bump_up`].
pub(crate) struct BumpUp {
    /// The new position of the bump allocator (the next [`BumpProps::start`]).
    pub(crate) new_pos: usize,

    /// The address of the allocation's pointer.
    pub(crate) ptr: usize,
}

/// Does upwards bump allocation.
///
/// - `end` must be a multiple of [`MIN_CHUNK_ALIGN`]
#[inline(always)]
pub(crate) fn bump_up(props: BumpProps) -> Option<BumpUp> {
    props.debug_assert_valid();

    let BumpProps {
        mut start,
        end,
        layout,
        min_align,
        align_is_const,
        size_is_const,
        size_is_multiple_of_align,
    } = props;

    debug_assert_eq!(start % min_align, 0);
    debug_assert_eq!(end % MIN_CHUNK_ALIGN, 0);

    // Used for assertion at the end of the function.
    let original_start = start;

    let mut new_pos;

    // Doing the `layout.size() < MIN_CHUNK_ALIGN` trick here (as seen in !UP)
    // results in worse codegen, so we don't.

    if align_is_const && layout.align() <= MIN_CHUNK_ALIGN {
        // Constant, small alignment fast path!

        if align_is_const && layout.align() <= min_align {
            // Alignment is already sufficient.
        } else {
            // Aligning an address that is `<= range.end` with an alignment `<= MIN_CHUNK_ALIGN`
            // can't exceed `range.end` nor overflow as `range.end` is always aligned to `MIN_CHUNK_ALIGN`.
            start = up_align_unchecked(start, layout.align());
        }

        let remaining = end - start;

        if unlikely(layout.size() > remaining) {
            return None;
        }

        // Doesn't exceed `end` because of the check above.
        new_pos = start + layout.size();
    } else {
        // Alignment is `> MIN_CHUNK_ALIGN` or not const.

        // `start` and `align` are both nonzero.
        // `aligned_down` is the aligned pointer minus `layout.align()`.
        let aligned_down = (start - 1) & !(layout.align() - 1);

        // `align + size` cannot overflow as per `Layout`'s rules.
        //
        // This could also be a `checked_add`, but we use `saturating_add` to save us a branch.
        // The `if` below will return None if the addition saturated and returned `usize::MAX`.
        new_pos = aligned_down.saturating_add(layout.align() + layout.size());

        // Note that `new_pos` being `usize::MAX` is an invalid value for `new_pos` and we MUST return None.
        // Due to `end` being always aligned to `MIN_CHUNK_ALIGN`, it can't be `usize::MAX`.
        // Thus when `new_pos` is `usize::MAX` this will always return None.
        if unlikely(new_pos > end) {
            return None;
        }

        // Doesn't exceed `end` because `aligned_down + align + size` didn't.
        start = aligned_down + layout.align();
    }

    if (align_is_const && size_is_multiple_of_align && layout.align() >= min_align)
        || (size_is_const && (layout.size() % min_align == 0))
    {
        // We are already aligned to `min_align`.
    } else {
        // Up aligning an address `<= range.end` with an alignment `<= MIN_CHUNK_ALIGN` (which `min_align` is)
        // can't exceed `range.end` and thus also can't overflow.
        new_pos = up_align_unchecked(new_pos, min_align);
    }

    debug_assert_aligned!(start, layout.align());
    debug_assert_aligned!(start, min_align);
    debug_assert_aligned!(new_pos, min_align);
    debug_assert_ne!(new_pos, 0);
    debug_assert_ne!(start, 0);
    debug_assert_le!(start, end);
    debug_assert_ge!(start, original_start);
    debug_assert_ge!(new_pos - start, layout.size());
    debug_assert_le!(new_pos, end);
    debug_assert_ge!(new_pos, start);

    Some(BumpUp { new_pos, ptr: start })
}

/// Does downwards bump allocation.
#[inline(always)]
pub(crate) fn bump_down(props: BumpProps) -> Option<usize> {
    props.debug_assert_valid();

    let BumpProps {
        start,
        mut end,
        layout,
        min_align,
        align_is_const,
        size_is_const,
        size_is_multiple_of_align,
    } = props;

    debug_assert_eq!(start % MIN_CHUNK_ALIGN, 0);
    debug_assert_eq!(end % min_align, 0);

    // Used for assertions only.
    let original_end = end;

    // This variables is meant to be computed at compile time.
    let needs_aligning = {
        // The bump pointer must end up aligned to the layout's alignment.
        //
        // Manual alignment for the layout's alignment can be elided
        // if the layout's size is a multiple of its alignment
        // and its alignment is less or equal to the minimum alignment.
        let can_elide_aligning_for_layout = size_is_multiple_of_align && align_is_const && layout.align() <= min_align;

        // The bump pointer must end up aligned to the minimum alignment again.
        //
        // Manual alignment for the minimum alignment can be elided if:
        // - the layout's size is a multiple of its alignment
        //   and its alignment is greater or equal to the minimum alignment
        // - the layout's size is a multiple of the minimum alignment
        //
        // In either case the bump pointer will end up inherently aligned when the pointer
        // is bumped downwards by the layout's size.
        let can_elide_aligning_for_min_align = {
            let due_to_layout_align = size_is_multiple_of_align && align_is_const && layout.align() >= min_align;
            let due_to_layout_size = size_is_const && (layout.size() % min_align == 0);
            due_to_layout_align || due_to_layout_size
        };

        let can_elide_aligning = can_elide_aligning_for_layout && can_elide_aligning_for_min_align;

        !can_elide_aligning
    };

    if size_is_const && layout.size() <= MIN_CHUNK_ALIGN {
        // When `size <= MIN_CHUNK_ALIGN` subtracting it from `end` can't overflow, as the lowest value for `end` would be `start` which is aligned to `MIN_CHUNK_ALIGN`,
        // thus its address can't be smaller than it.
        end -= layout.size();

        if needs_aligning {
            // At this point layout's align is const, because we assume `size_is_const` implies `align_is_const`.
            // That means `max` is evaluated at compile time, so we don't bother having different cases for either alignment.
            end = down_align(end, layout.align().max(min_align));
        }

        if unlikely(end < start) {
            return None;
        }
    } else if align_is_const && layout.align() <= MIN_CHUNK_ALIGN {
        // Constant, small alignment fast path!
        let remaining = end - start;

        if unlikely(layout.size() > remaining) {
            return None;
        }

        // Doesn't overflow because of the check above.
        end -= layout.size();

        if needs_aligning {
            // Down aligning an address `>= range.start` with an alignment `<= MIN_CHUNK_ALIGN` (which `layout.align()` is)
            // can't exceed `range.start`, and thus also can't overflow.
            end = down_align(end, layout.align().max(min_align));
        }
    } else {
        // Alignment is `> MIN_CHUNK_ALIGN` or not const.

        // This could also be a `checked_sub`, but we use `saturating_sub` to save us a branch.
        // The `if` below will return None if the addition saturated and returned `0`.
        end = end.saturating_sub(layout.size());
        end = down_align(end, layout.align().max(min_align));

        // Note that `end` being `0` is an invalid value for `end` and we MUST return None.
        // Due to `start` being `NonNull`, it can't be `0`.
        // Thus when `end` is `0` this will always return None.
        if unlikely(end < start) {
            return None;
        }
    }

    debug_assert_aligned!(end, layout.align());
    debug_assert_aligned!(end, min_align);
    debug_assert_ne!(end, 0);
    debug_assert_ge!(end, start);
    debug_assert_ge!(original_end - end, layout.size());

    Some(end)
}

/// Prepares a slice allocation by returning the start and end address for a maximally sized region
/// where both start and end are aligned to `layout.align()`.
///
/// - `end` must be a multiple of [`MIN_CHUNK_ALIGN`]
#[inline(always)]
pub(crate) fn bump_prepare_up(props: BumpProps) -> Option<Range<usize>> {
    props.debug_assert_valid();

    let BumpProps {
        mut start,
        end,
        layout,
        min_align,
        align_is_const,
        size_is_const: _,
        size_is_multiple_of_align,
    } = props;

    debug_assert!(size_is_multiple_of_align);
    debug_assert_eq!(end % MIN_CHUNK_ALIGN, 0);

    if align_is_const && layout.align() <= min_align {
        // Alignment is already sufficient.
    } else {
        // `start` needs to be aligned.
        if align_is_const && layout.align() <= MIN_CHUNK_ALIGN {
            // Aligning an address that is `<= range.end` with an alignment
            // that is `<= MIN_CHUNK_ALIGN` can't exceed `range.end` and
            // thus can't overflow.
            start = up_align_unchecked(start, layout.align());
        } else {
            start = up_align(start, layout.align())?.get();

            if unlikely(start > end) {
                return None;
            }
        }
    }

    let remaining = end - start;

    if unlikely(layout.size() > remaining) {
        return None;
    }

    // Layout fits, we just trim off the excess to make end aligned.
    //
    // Note that `end` does *not* need to be aligned to `min_align` because the `prepare` operation doesn't
    // move the bump pointer which is the thing that does need to be aligned.
    //
    // Aligning to `min_align` will happen once `use_prepared_slice_allocation` is called.
    let end = down_align(end, layout.align());

    debug_assert_aligned!(start, layout.align());
    debug_assert_aligned!(end, layout.align());
    debug_assert_ne!(start, 0);
    debug_assert_ne!(end, 0);

    Some(start..end)
}

/// Prepares a slice allocation by returning the start and end address for a maximally sized region
/// where both start and end are aligned to `layout.align()`.
#[inline(always)]
pub(crate) fn bump_prepare_down(props: BumpProps) -> Option<Range<usize>> {
    props.debug_assert_valid();

    let BumpProps {
        start,
        mut end,
        layout,
        min_align,
        align_is_const,
        size_is_const: _,
        size_is_multiple_of_align,
    } = props;

    debug_assert!(size_is_multiple_of_align);

    if align_is_const && layout.align() <= min_align {
        // Alignment is already sufficient.
    } else {
        end = down_align(end, layout.align());

        if align_is_const && layout.align() <= MIN_CHUNK_ALIGN {
            // End is valid.
        } else {
            // End could be less than start at this point.
            if unlikely(end < start) {
                return None;
            }
        }
    }

    let remaining = end - start;

    if unlikely(layout.size() > remaining) {
        return None;
    }

    // Layout fits, we just trim off the excess to make start aligned.
    //
    // Note that `start` doesn't need to be aligned to `min_align` because the `prepare` operation doesn't
    // move the bump pointer which is the thing that needs to be aligned.
    //
    // Aligning to `min_align` will happen once `use_prepared_slice_allocation` is called.
    let start = up_align_unchecked(start, layout.align());

    debug_assert_aligned!(start, layout.align());
    debug_assert_aligned!(end, layout.align());
    debug_assert_ne!(start, 0);
    debug_assert_ne!(end, 0);

    Some(start..end)
}

#[inline(always)]
const fn down_align(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}

/// Doesn't check for overflow.
#[inline(always)]
const fn up_align_unchecked(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    (addr + mask) & !mask
}

#[inline(always)]
const fn up_align(addr: usize, align: usize) -> Option<NonZeroUsize> {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    let addr_plus_mask = match addr.checked_add(mask) {
        Some(addr_plus_mask) => addr_plus_mask,
        None => return None,
    };
    NonZeroUsize::new(addr_plus_mask & !mask)
}
