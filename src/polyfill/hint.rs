#[cfg(feature = "alloc")]
pub unsafe fn assert_unchecked(b: bool) {
    if !b {
        unsafe { core::hint::unreachable_unchecked() }
    }
}
