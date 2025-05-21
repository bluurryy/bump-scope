/// See [`std::ptr::from_ref`].
#[must_use]
#[inline(always)]
pub(crate) fn from_ref<T: ?Sized>(r: &T) -> *const T {
    r
}

/// See [`std::ptr::from_mut`].
#[must_use]
#[inline(always)]
pub(crate) fn from_mut<T: ?Sized>(r: &mut T) -> *mut T {
    r
}

/// See [`std::ptr::without_provenance_mut`].
#[must_use]
#[inline(always)]
#[cfg(feature = "alloc")]
pub(crate) const fn without_provenance_mut<T>(addr: usize) -> *mut T {
    // An int-to-pointer transmute currently has exactly the intended semantics: it creates a
    // pointer without provenance. Note that this is *not* a stable guarantee about transmute
    // semantics, it relies on sysroot crates having special status.
    // SAFETY: every valid integer is also a valid pointer (as long as you don't dereference that
    // pointer).
    unsafe { core::mem::transmute(addr) }
}
