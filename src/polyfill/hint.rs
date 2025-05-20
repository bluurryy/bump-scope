/// See [`std::hint::assert_unchecked`].
pub unsafe fn assert_unchecked(condition: bool) {
    if !condition {
        unsafe { core::hint::unreachable_unchecked() }
    }
}
