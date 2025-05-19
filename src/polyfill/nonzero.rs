use core::num::NonZeroUsize;

#[inline(always)]
#[allow(dead_code)]
pub(crate) fn prev_power_of_two(value: NonZeroUsize) -> NonZeroUsize {
    let highest_bit = (usize::BITS - 1) - value.leading_zeros();
    unsafe { NonZeroUsize::new_unchecked(1 << highest_bit) }
}

#[inline(always)]
#[allow(dead_code)]
pub(crate) fn down_align(addr: NonZeroUsize, align: NonZeroUsize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align.get() - 1;
    addr.get() & !mask
}
