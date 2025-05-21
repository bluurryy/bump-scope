use core::{mem, ptr};

use crate::polyfill;

/// See `<*const T>::len`.
///
/// This implementation has an additional safety invariant though.
///
/// # Safety
/// `ptr` must be valid to be turned into a reference.
#[must_use]
#[inline(always)]
pub(crate) unsafe fn len<T>(ptr: *const [T]) -> usize {
    // if we followed clippy's advice, check would instead complain about `dangerous_implicit_autorefs`
    #[allow(clippy::needless_borrow)]
    (&(*ptr)).len()
}

/// See `<*const T>::offset_from_unsigned`.
#[inline]
#[cfg_attr(miri, track_caller)] // even without panics, this helps for Miri backtraces
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::checked_conversions)]
pub(crate) unsafe fn offset_from_unsigned<T>(this: *const T, origin: *const T) -> usize {
    polyfill::hint::assert_unchecked(this >= origin);
    let pointee_size = mem::size_of::<T>();
    assert!(0 < pointee_size && pointee_size <= isize::MAX as usize);
    this.offset_from(origin) as usize
}

/// Not part of std.
///
/// Putting the expression in a function helps llvm to realize that it can initialize the value
/// at this pointer instead of allocating it on the stack and then copying it over.
#[inline(always)]
pub(crate) unsafe fn write_with<T>(ptr: *mut T, f: impl FnOnce() -> T) {
    ptr::write(ptr, f());
}

/// See `<*const T>::cast_mut`.
#[inline(always)]
pub(crate) const fn cast_mut<T: ?Sized>(ptr: *const T) -> *mut T {
    ptr as _
}

/// See `<*const T>::addr`.
#[must_use]
#[inline(always)]
pub(crate) fn addr<T>(ptr: *const T) -> usize {
    // A pointer-to-integer transmute currently has exactly the right semantics: it returns the
    // address without exposing the provenance. Note that this is *not* a stable guarantee about
    // transmute semantics, it relies on sysroot crates having special status.
    // SAFETY: Pointer-to-integer transmutes are valid (if you are okay with losing the
    // provenance).
    unsafe { mem::transmute(ptr.cast::<()>()) }
}
