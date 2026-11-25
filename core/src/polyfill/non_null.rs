use core::ptr::NonNull;

/// See [`std::ptr::NonNull::is_aligned_to`].
#[inline(always)]
pub(crate) fn is_aligned_to(ptr: NonNull<u8>, align: usize) -> bool {
    debug_assert!(align.is_power_of_two());
    ptr.addr().get() & (align - 1) == 0
}

/// See [`core::ptr::NonNull::as_non_null_ptr`].
#[inline(always)]
pub(crate) const fn as_non_null_ptr<T>(ptr: NonNull<[T]>) -> NonNull<T> {
    ptr.cast()
}
