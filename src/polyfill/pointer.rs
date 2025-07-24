use core::{hint, mem, ptr};

/// See `<*const T>::offset_from_unsigned`.
#[inline]
#[cfg_attr(miri, track_caller)] // even without panics, this helps for Miri backtraces
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::checked_conversions)]
pub(crate) unsafe fn offset_from_unsigned<T>(this: *const T, origin: *const T) -> usize {
    hint::assert_unchecked(this >= origin);
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
