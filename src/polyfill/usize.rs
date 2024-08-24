#[inline(always)]
pub(crate) unsafe fn unchecked_mul(lhs: usize, rhs: usize) -> usize {
    lhs * rhs
}

#[cfg(test)]
#[inline(always)]
pub(crate) const fn max(lhs: usize, rhs: usize) -> usize {
    if lhs > rhs {
        lhs
    } else {
        rhs
    }
}
