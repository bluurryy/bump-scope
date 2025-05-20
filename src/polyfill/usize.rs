/// See [`usize::unchecked_mul`].
#[inline(always)]
pub(crate) unsafe fn unchecked_mul(lhs: usize, rhs: usize) -> usize {
    lhs * rhs
}

/// See [`usize::unchecked_sub`].
#[inline(always)]
pub(crate) unsafe fn unchecked_sub(lhs: usize, rhs: usize) -> usize {
    lhs - rhs
}
