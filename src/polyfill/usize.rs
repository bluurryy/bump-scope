#[inline(always)]
pub(crate) unsafe fn unchecked_mul(lhs: usize, rhs: usize) -> usize {
    lhs * rhs
}

#[inline(always)]
pub(crate) unsafe fn unchecked_sub(lhs: usize, rhs: usize) -> usize {
    lhs - rhs
}
