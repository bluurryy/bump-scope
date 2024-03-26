#[inline(always)]
pub unsafe fn unchecked_mul(lhs: usize, rhs: usize) -> usize {
    lhs * rhs
}

#[cfg(test)]
#[inline(always)]
pub const fn max(lhs: usize, rhs: usize) -> usize {
    if lhs > rhs {
        lhs
    } else {
        rhs
    }
}
