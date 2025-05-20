use core::{mem, ptr};

use crate::assume_unchecked;

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

/// See [`pointer::as_mut_ptr`].
#[inline(always)]
#[allow(dead_code)]
pub(crate) const fn as_mut_ptr<T>(ptr: *mut [T]) -> *mut T {
    ptr.cast()
}

/// See [`pointer::len`].
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

/// See [`pointer::offset_from_unsigned`].
#[inline]
#[cfg_attr(miri, track_caller)] // even without panics, this helps for Miri backtraces
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::checked_conversions)]
pub(crate) unsafe fn offset_from_unsigned<T>(lhs: *const T, rhs: *const T) -> usize {
    assume_unchecked(lhs >= rhs);
    let pointee_size = mem::size_of::<T>();
    assert!(0 < pointee_size && pointee_size <= isize::MAX as usize);
    lhs.offset_from(rhs) as usize
}

/// Not part of std.
///
/// Putting the expression in a function helps llvm to realize that it can initialize the value
/// at this pointer instead of allocating it on the stack and then copying it over.
#[inline(always)]
pub(crate) unsafe fn write_with<T>(ptr: *mut T, f: impl FnOnce() -> T) {
    ptr::write(ptr, f());
}

/// See [`pointer::cast_mut`].
#[inline(always)]
pub(crate) const fn cast_mut<T: ?Sized>(ptr: *const T) -> *mut T {
    ptr as _
}
