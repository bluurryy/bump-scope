use core::{
    num::NonZeroUsize,
    ptr::{self, NonNull},
};

use crate::polyfill::pointer;

// Putting the expression in a function helps llvm to realize that it can initialize the value
// at this pointer instead of allocating it on the stack and then copying it over.
#[inline(always)]
pub(crate) unsafe fn write_with<T>(ptr: NonNull<T>, f: impl FnOnce() -> T) {
    ptr::write(ptr.as_ptr(), f());
}

/// See [`std::ptr::NonNull::add`].
#[inline(always)]
pub(crate) unsafe fn add<T>(ptr: NonNull<T>, delta: usize) -> NonNull<T>
where
    T: Sized,
{
    // SAFETY: We require that the delta stays in-bounds of the object, and
    // thus it cannot become null, as that would require wrapping the
    // address space, which no legal objects are allowed to do.
    // And the caller promised the `delta` is sound to add.
    NonNull::new_unchecked(ptr.as_ptr().add(delta))
}

/// See [`std::ptr::NonNull::sub`].
#[inline(always)]
pub(crate) const unsafe fn sub<T>(ptr: NonNull<T>, delta: usize) -> NonNull<T>
where
    T: Sized,
{
    // SAFETY: We require that the delta stays in-bounds of the object, and
    // thus it cannot become null, as no legal objects can be allocated
    // in such as way that the null address is part of them.
    // And the caller promised the `delta` is sound to subtract.
    NonNull::new_unchecked(ptr.as_ptr().sub(delta))
}

/// See [`std::ptr::NonNull::byte_add`].
#[must_use]
#[inline(always)]
#[allow(dead_code)]
#[cfg_attr(miri, track_caller)] // even without panics, this helps for Miri backtraces
pub(crate) unsafe fn byte_add<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    add(ptr.cast::<u8>(), count).cast()
}

/// See [`std::ptr::NonNull::byte_sub`].
#[must_use]
#[inline(always)]
#[allow(dead_code)]
#[cfg_attr(miri, track_caller)] // even without panics, this helps for Miri backtraces
pub(crate) unsafe fn byte_sub<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    sub(ptr.cast::<u8>(), count).cast()
}

/// Not part of std, but for context see [`pointer::wrapping_add`].
#[inline(always)]
pub(crate) unsafe fn wrapping_byte_add<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    NonNull::new_unchecked(ptr.as_ptr().cast::<u8>().wrapping_add(count).cast())
}

/// Not part of std, but for context see [`pointer::wrapping_sub`].
#[inline(always)]
pub(crate) unsafe fn wrapping_byte_sub<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    NonNull::new_unchecked(ptr.as_ptr().cast::<u8>().wrapping_sub(count).cast())
}

/// See [`std::ptr::NonNull::addr`].
#[inline(always)]
pub(crate) fn addr<T>(ptr: NonNull<T>) -> NonZeroUsize {
    // SAFETY: The pointer is guaranteed by the type to be non-null,
    // meaning that the address will be non-zero.
    unsafe { NonZeroUsize::new_unchecked(sptr::Strict::addr(ptr.as_ptr())) }
}

/// See [`std::ptr::NonNull::with_addr`].
#[must_use]
#[inline(always)]
pub(crate) fn with_addr<T>(ptr: NonNull<T>, addr: NonZeroUsize) -> NonNull<T> {
    // SAFETY: The result of `ptr::from::with_addr` is non-null because `addr` is guaranteed to be non-zero.
    unsafe { NonNull::new_unchecked(sptr::Strict::with_addr(ptr.as_ptr(), addr.get())) }
}

/// See [`std::ptr::NonNull::offset_from_unsigned`].
#[must_use]
#[inline(always)]
pub(crate) unsafe fn offset_from_unsigned<T>(this: NonNull<T>, origin: NonNull<T>) -> usize {
    pointer::offset_from_unsigned(this.as_ptr(), origin.as_ptr())
}

/// See [`std::ptr::NonNull::byte_offset_from_unsigned`].
#[must_use]
#[inline(always)]
pub(crate) unsafe fn byte_offset_from_unsigned<T>(this: NonNull<T>, origin: NonNull<T>) -> usize {
    offset_from_unsigned::<u8>(this.cast(), origin.cast())
}

/// See [`std::ptr::NonNull::slice_from_raw_parts`].
#[must_use]
#[inline(always)]
pub(crate) const fn slice_from_raw_parts<T>(data: NonNull<T>, len: usize) -> NonNull<[T]> {
    // SAFETY: `data` is a `NonNull` pointer which is necessarily non-null
    unsafe { NonNull::new_unchecked(pointer::cast_mut(ptr::slice_from_raw_parts(data.as_ptr(), len))) }
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

/// See [`std::ptr::copy`].
#[inline(always)]
pub(crate) unsafe fn copy<T>(src: NonNull<T>, dst: NonNull<T>, count: usize) {
    ptr::copy(src.as_ptr(), dst.as_ptr(), count);
}

/// See [`std::ptr::copy_nonoverlapping`].
#[inline(always)]
pub(crate) unsafe fn copy_nonoverlapping<T>(src: NonNull<T>, dst: NonNull<T>, count: usize) {
    ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), count);
}

/// Not part of std.
#[inline(always)]
pub(crate) unsafe fn result<T, E>(mut ptr: NonNull<Result<T, E>>) -> Result<NonNull<T>, NonNull<E>> {
    match ptr.as_mut() {
        Ok(ok) => Ok(ok.into()),
        Err(err) => Err(err.into()),
    }
}

/// See [`std::ptr::NonNull::is_aligned_to`].
#[inline(always)]
pub(crate) fn is_aligned_to(ptr: NonNull<u8>, align: usize) -> bool {
    debug_assert!(align.is_power_of_two());
    addr(ptr).get() & (align - 1) == 0
}

/// See [`core::ptr::NonNull::as_non_null_ptr`].
#[inline(always)]
pub(crate) const fn as_non_null_ptr<T>(ptr: NonNull<[T]>) -> NonNull<T> {
    ptr.cast()
}

/// Not part of std.
#[inline(always)]
pub(crate) fn set_ptr<T>(ptr: &mut NonNull<[T]>, new_ptr: NonNull<T>) {
    let len = ptr.len();
    *ptr = slice_from_raw_parts(new_ptr, len);
}

/// Not part of std.
#[inline(always)]
pub(crate) fn set_len<T>(ptr: &mut NonNull<[T]>, new_len: usize) {
    let elem_ptr = as_non_null_ptr(*ptr);
    *ptr = slice_from_raw_parts(elem_ptr, new_len);
}

/// See [`std::ptr::NonNull::drop_in_place`].
#[inline(always)]
pub(crate) unsafe fn drop_in_place<T: ?Sized>(ptr: NonNull<T>) {
    ptr.as_ptr().drop_in_place();
}

/// Not part of std, but for context see [`std::vec::Vec::truncate`].
///
/// # Safety
///
/// `ptr` must point to a valid slice.
pub(crate) unsafe fn truncate<T>(slice: &mut NonNull<[T]>, len: usize) {
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

    let to_drop_start = add(as_non_null_ptr(*slice), len);
    let to_drop = slice_from_raw_parts(to_drop_start, remaining_len);

    set_len::<T>(slice, len);
    drop_in_place(to_drop);
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
pub const fn without_provenance<T>(addr: NonZeroUsize) -> NonNull<T> {
    let pointer = sptr::invalid_mut(addr.get());

    // SAFETY: we know `addr` is non-zero.
    unsafe { NonNull::new_unchecked(pointer) }
}
