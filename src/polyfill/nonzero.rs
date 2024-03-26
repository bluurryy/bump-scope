use super::const_unwrap;
use core::num::NonZeroUsize;

#[inline(always)]
pub const fn max(lhs: NonZeroUsize, rhs: usize) -> NonZeroUsize {
    let max = if lhs.get() > rhs { lhs.get() } else { rhs };
    // Panic can not happen and is optimized away.
    const_unwrap(NonZeroUsize::new(max))
}

#[inline(always)]
#[allow(dead_code)]
pub fn prev_power_of_two(value: NonZeroUsize) -> NonZeroUsize {
    let highest_bit = (usize::BITS - 1) - value.leading_zeros();
    unsafe { NonZeroUsize::new_unchecked(1 << highest_bit) }
}

#[inline(always)]
#[allow(dead_code)]
pub fn down_align(addr: NonZeroUsize, align: NonZeroUsize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align.get() - 1;
    addr.get() & !mask
}
