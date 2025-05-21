use core::mem;

/// See `<*mut T>::addr`.
#[must_use]
#[inline(always)]
pub(crate) fn addr<T>(ptr: *mut T) -> usize {
    // A pointer-to-integer transmute currently has exactly the right semantics: it returns the
    // address without exposing the provenance. Note that this is *not* a stable guarantee about
    // transmute semantics, it relies on sysroot crates having special status.
    // SAFETY: Pointer-to-integer transmutes are valid (if you are okay with losing the
    // provenance).
    unsafe { mem::transmute(ptr.cast::<()>()) }
}

/// See `<*mut T>::with_addr`.
#[inline]
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub(crate) fn with_addr<T>(ptr: *mut T, addr: usize) -> *mut T {
    // This should probably be an intrinsic to avoid doing any sort of arithmetic, but
    // meanwhile, we can implement it with `wrapping_offset`, which preserves the pointer's
    // provenance.
    let self_addr = self::addr(ptr) as isize;
    let dest_addr = addr as isize;
    let offset = dest_addr.wrapping_sub(self_addr);
    wrapping_byte_offset(ptr, offset)
}

/// See `<*mut T>::wrapping_byte_offset`.
#[must_use]
#[inline(always)]
pub(crate) const fn wrapping_byte_offset<T>(ptr: *mut T, count: isize) -> *mut T {
    ptr.cast::<u8>().wrapping_offset(count).cast()
}
