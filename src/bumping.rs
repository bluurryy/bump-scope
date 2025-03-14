#![forbid(unsafe_code)]
//! Make sure you sync this file with `crates/fuzzing-support/src/from_bump_scope/bumping.rs`.
//!
//! This file intentionally doesn't import anything other than `core`
//! to make it easy to fuzz (see above) and debug.

use core::{alloc::Layout, num::NonZeroUsize, ops::Range};

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

/// Arguments for the [`bump_up`], [`bump_down`], [`bump_prepare_up`] and [`bump_prepare_down`].
///
/// The fields `min_align`, `align_is_const`, `size_is_const`, `size_is_multiple_of_align` are expected to be constants.
/// `bump_up` and `bump_down` are optimized for that case.
///
/// Choosing `false` for `align_is_const`, `size_is_const`, `size_is_multiple_of_align` is always valid.
pub(crate) struct BumpProps {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) layout: Layout,
    pub(crate) min_align: usize,
    pub(crate) align_is_const: bool,
    pub(crate) size_is_const: bool,
    pub(crate) size_is_multiple_of_align: bool,
}

pub(crate) struct BumpUp {
    pub(crate) new_pos: usize,
    pub(crate) ptr: usize,
}

#[inline(always)]
pub(crate) fn bump_up(
    BumpProps {
        mut start,
        end,
        layout,
        min_align,
        align_is_const,
        size_is_const,
        size_is_multiple_of_align,
    }: BumpProps,
) -> Option<BumpUp> {
    // Used for assertions only.
    let original_start = start;

    debug_assert_ne!(start, 0);
    debug_assert_ne!(end, 0);

    debug_assert!(min_align.is_power_of_two());
    debug_assert!(min_align <= MIN_CHUNK_ALIGN);

    if size_is_multiple_of_align {
        debug_assert_eq!(layout.size() % layout.align(), 0);
    }

    debug_assert!(start <= end);
    debug_assert_eq!(end % MIN_CHUNK_ALIGN, 0);

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

        if layout.size() > remaining {
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
        if new_pos > end {
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

#[inline(always)]
pub(crate) fn bump_down(
    BumpProps {
        start,
        mut end,
        layout,
        min_align,
        align_is_const,
        size_is_const,
        size_is_multiple_of_align,
    }: BumpProps,
) -> Option<usize> {
    // Used for assertions only.
    let original_end = end;

    debug_assert_ne!(start, 0);
    debug_assert_ne!(end, 0);

    debug_assert!(min_align.is_power_of_two());
    debug_assert!(min_align <= MIN_CHUNK_ALIGN);

    if size_is_multiple_of_align {
        debug_assert_eq!(layout.size() % layout.align(), 0);
    }

    debug_assert!(start <= end);

    // These are expected to be evaluated at compile time.
    let does_not_need_align_for_min_align_due_to_align =
        size_is_multiple_of_align && align_is_const && layout.align() >= min_align;
    let does_not_need_align_for_min_align_due_to_size = size_is_const && (layout.size() % min_align == 0);
    let does_not_need_align_for_min_align =
        does_not_need_align_for_min_align_due_to_align || does_not_need_align_for_min_align_due_to_size;

    let does_not_need_align_for_layout = size_is_multiple_of_align && align_is_const && layout.align() <= min_align;

    let does_not_need_align = does_not_need_align_for_min_align && does_not_need_align_for_layout;
    let needs_align = !does_not_need_align;

    if size_is_const && layout.size() <= MIN_CHUNK_ALIGN {
        // When `size <= MIN_CHUNK_ALIGN` subtracting it from `end` can't overflow, as the lowest value for `end` would be `start` which is aligned to `MIN_CHUNK_ALIGN`,
        // thus its address can't be smaller than it.
        end -= layout.size();

        if needs_align {
            // At this point layout's align is const, because we assume `size_is_const` implies `align_is_const`.
            // That means `max` is evaluated at compile time, so we don't bother having different cases for either alignment.
            end = down_align(end, layout.align().max(min_align));
        }

        if end < start {
            return None;
        }
    } else if align_is_const && layout.align() <= MIN_CHUNK_ALIGN {
        // Constant, small alignment fast path!
        let remaining = end - start;

        if layout.size() > remaining {
            return None;
        }

        // Doesn't overflow because of the check above.
        end -= layout.size();

        if needs_align {
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
        if end < start {
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

#[inline(always)]
pub(crate) fn bump_prepare_up(
    BumpProps {
        mut start,
        end,
        layout,
        min_align,
        align_is_const,
        size_is_const: _,
        size_is_multiple_of_align: _,
    }: BumpProps,
) -> Option<Range<usize>> {
    debug_assert!(layout.size() % layout.align() == 0);
    debug_assert!(start <= end);
    debug_assert!(end % MIN_CHUNK_ALIGN == 0);

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

            if start > end {
                return None;
            }
        }
    }

    let remaining = end - start;

    if layout.size() > remaining {
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

#[inline(always)]
pub(crate) fn bump_prepare_down(
    BumpProps {
        start,
        mut end,
        layout,
        min_align,
        align_is_const,
        size_is_const: _,
        size_is_multiple_of_align: _,
    }: BumpProps,
) -> Option<Range<usize>> {
    debug_assert!(layout.size() % layout.align() == 0);
    debug_assert!(start <= end);

    if align_is_const && layout.align() <= min_align {
        // Alignment is already sufficient.
    } else {
        end = down_align(end, layout.align());

        if align_is_const && layout.align() <= MIN_CHUNK_ALIGN {
            // End is valid.
        } else {
            // End could be less than start at this point.
            if end < start {
                return None;
            }
        }
    }

    let remaining = end - start;

    if layout.size() > remaining {
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
