/// See [`usize::unchecked_mul`].
#[inline(always)]
pub(crate) unsafe fn unchecked_mul(lhs: usize, rhs: usize) -> usize {
    lhs * rhs
}
