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
