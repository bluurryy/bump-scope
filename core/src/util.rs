use core::{mem::ManuallyDrop, num::NonZeroUsize, ptr};

use crate::config::{BumpConfig, config};

#[inline(always)]
pub(crate) const fn const_assert_upwards_matches<const C: BumpConfig, const NEW_C: BumpConfig>() {
    const {
        let old = config!(upwards);
        let new = config!(NEW_C, upwards);
        assert!(old == new, "can't change bump direction");
    }
}

/// A version of [`std::mem::transmute`] that can transmute between generic types.
#[allow(clippy::manual_assert_eq, reason = "wouldn't be const otherwise")]
pub(crate) unsafe fn transmute_value<A, B>(a: A) -> B {
    assert!(size_of::<A>() == size_of::<B>());
    unsafe { core::mem::transmute_copy(&ManuallyDrop::new(a)) }
}

/// A safer [`std::mem::transmute`] for references.
#[allow(clippy::manual_assert_eq, reason = "wouldn't be const otherwise")]
pub(crate) const unsafe fn transmute_ref<A, B>(a: &A) -> &B {
    assert!(size_of::<A>() == size_of::<B>());
    unsafe { &*ptr::from_ref(a).cast::<B>() }
}

/// A safer [`std::mem::transmute`] for mutable references.
#[allow(clippy::manual_assert_eq, reason = "wouldn't be const otherwise")]
pub(crate) unsafe fn transmute_mut<A, B>(a: &mut A) -> &mut B {
    assert!(size_of::<A>() == size_of::<B>());
    unsafe { &mut *ptr::from_mut(a).cast::<B>() }
}

/// Does not check for overflow.
#[inline(always)]
pub(crate) fn up_align_usize_unchecked(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    (addr + mask) & !mask
}

#[inline(always)]
pub(crate) fn down_align_usize(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}

#[inline(always)]
pub(crate) fn bump_down(addr: NonZeroUsize, size: usize, align: usize) -> usize {
    let subtracted = addr.get().saturating_sub(size);
    down_align_usize(subtracted, align)
}

pub(crate) trait SizedTypeProperties: Sized {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = align_of::<Self>();
}

impl<T> SizedTypeProperties for T {}

/// Aligns a bump address to the `min_align`.
///
/// This is a noop when `min_align` is `0`.
#[must_use]
#[inline(always)]
pub(crate) fn align_pos(up: bool, min_align: usize, pos: usize) -> usize {
    if up {
        // Aligning an address that is `<= range.end` with an alignment
        // that is `<= MIN_CHUNK_ALIGN` cannot exceed `range.end` and
        // cannot overflow as `range.end` is always aligned to `MIN_CHUNK_ALIGN`.
        up_align_usize_unchecked(pos, min_align)
    } else {
        // The chunk start is non-null and is aligned to `MIN_CHUNK_ALIGN`
        // `MIN_ALIGN <= MIN_CHUNK_ALIGN` will never pass the chunk start
        // and stay non-zero.
        down_align_usize(pos, min_align)
    }
}
