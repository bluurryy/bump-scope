use core::{
    ops::Range,
    ptr::{self, NonNull},
};

#[cfg(feature = "alloc")]
use core::num::NonZeroUsize;

use crate::polyfill::pointer;

/// See [`std::ptr::NonNull::offset_from_unsigned`].
#[must_use]
#[inline(always)]
pub(crate) unsafe fn offset_from_unsigned<T>(this: NonNull<T>, origin: NonNull<T>) -> usize {
    unsafe { pointer::offset_from_unsigned(this.as_ptr(), origin.as_ptr()) }
}

/// See [`std::ptr::NonNull::byte_offset_from_unsigned`].
#[must_use]
#[inline(always)]
pub(crate) unsafe fn byte_offset_from_unsigned<T>(this: NonNull<T>, origin: NonNull<T>) -> usize {
    unsafe { offset_from_unsigned::<u8>(this.cast(), origin.cast()) }
}

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

/// See [`std::ptr::NonNull::from_ref`].
pub(crate) const fn from_ref<T>(r: &T) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(r as *const T as *mut T) }
}

/// See [`std::ptr::NonNull::as_mut_ptr`].
#[inline]
#[must_use]
pub const fn as_mut_ptr<T>(p: NonNull<[T]>) -> *mut T {
    as_non_null_ptr(p).as_ptr()
}

/// See [`std::ptr::NonNull::without_provenance`].
#[must_use]
#[inline]
#[cfg(feature = "alloc")]
pub(crate) const fn without_provenance<T>(addr: NonZeroUsize) -> NonNull<T> {
    let pointer = ptr::without_provenance_mut(addr.get());
    // SAFETY: we know `addr` is non-zero.
    unsafe { NonNull::new_unchecked(pointer) }
}

/// Not part of std, but for context see [`std::vec::Vec::truncate`].
///
/// # Safety
///
/// `ptr` must point to a valid slice.
pub(crate) unsafe fn truncate<T>(slice: &mut NonNull<[T]>, len: usize) {
    unsafe {
        // This is safe because:
        //
        // * the slice passed to `drop_in_place` is valid; the `len > self.len`
        //   case avoids creating an invalid slice, and
        // * the `len` of the slice is shrunk before calling `drop_in_place`,
        //   such that no value will be dropped twice in case `drop_in_place`
        //   were to panic once (if it panics twice, the program aborts).

        // Unlike std this is `>=`. Std uses `>` because when a call is inlined with `len` of `0` that optimizes better.
        // But this was likely only motivated because `clear` used to be implemented as `truncate(0)`.
        // See <https://github.com/rust-lang/rust/issues/76089#issuecomment-1889416842>.
        if len >= slice.len() {
            return;
        }

        let remaining_len = slice.len() - len;

        let to_drop_start = as_non_null_ptr(*slice).add(len);
        let to_drop = NonNull::slice_from_raw_parts(to_drop_start, remaining_len);

        set_len::<T>(slice, len);
        to_drop.drop_in_place();
    }
}

/// Not part of std, but for context see `<*mut T>::wrapping_add`.
#[inline(always)]
pub(crate) unsafe fn wrapping_byte_add<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(ptr.as_ptr().cast::<u8>().wrapping_add(count).cast()) }
}

/// Not part of std, but for context see `<*mut T>::wrapping_sub`.
#[inline(always)]
pub(crate) unsafe fn wrapping_byte_sub<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(ptr.as_ptr().cast::<u8>().wrapping_sub(count).cast()) }
}

/// Not part of std.
#[inline(always)]
pub(crate) fn set_ptr<T>(ptr: &mut NonNull<[T]>, new_ptr: NonNull<T>) {
    let len = ptr.len();
    *ptr = NonNull::slice_from_raw_parts(new_ptr, len);
}

/// Not part of std.
#[inline(always)]
pub(crate) fn set_len<T>(ptr: &mut NonNull<[T]>, new_len: usize) {
    let elem_ptr = as_non_null_ptr(*ptr);
    *ptr = NonNull::slice_from_raw_parts(elem_ptr, new_len);
}

/// Not part of std.
#[inline(always)]
pub(crate) unsafe fn result<T, E>(mut ptr: NonNull<Result<T, E>>) -> Result<NonNull<T>, NonNull<E>> {
    unsafe {
        match ptr.as_mut() {
            Ok(ok) => Ok(ok.into()),
            Err(err) => Err(err.into()),
        }
    }
}

#[inline(always)]
pub(crate) unsafe fn cast_range<T, U>(ptr: Range<NonNull<T>>) -> Range<NonNull<U>> {
    ptr.start.cast()..ptr.end.cast()
}

/// Not part of std.
#[must_use]
#[inline(always)]
pub(crate) const fn str_from_utf8(bytes: NonNull<[u8]>) -> NonNull<str> {
    unsafe { NonNull::new_unchecked(bytes.as_ptr() as *mut str) }
}

/// Not part of std.
#[must_use]
#[inline(always)]
pub(crate) const fn str_bytes(str: NonNull<str>) -> NonNull<[u8]> {
    unsafe { NonNull::new_unchecked(str.as_ptr() as *mut [u8]) }
}

/// Not part of std.
#[must_use]
#[inline(always)]
pub(crate) const fn str_len(str: NonNull<str>) -> usize {
    str_bytes(str).len()
}

/// Not part of std.
///
/// Putting the expression in a function helps llvm to realize that it can initialize the value
/// at this pointer instead of allocating it on the stack and then copying it over.
#[inline(always)]
pub(crate) unsafe fn write_with<T>(ptr: NonNull<T>, f: impl FnOnce() -> T) {
    unsafe { ptr::write(ptr.as_ptr(), f()) };
}
