use crate::{polyfill::pointer, SizedTypeProperties};
use core::{
    num::NonZeroUsize,
    ptr::{self, NonNull},
};

// Putting the expression in a function helps llvm to realize that it can initialize the value
// at this pointer instead of allocating it on the stack and then copying it over.
#[inline(always)]
pub(crate) unsafe fn write_with<T>(ptr: NonNull<T>, f: impl FnOnce() -> T) {
    ptr::write(ptr.as_ptr(), f());
}

/// See `pointer::add` for semantics and safety requirements.
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

/// See `pointer::sub` for semantics and safety requirements.
#[inline(always)]
pub(crate) unsafe fn sub<T>(ptr: NonNull<T>, delta: usize) -> NonNull<T>
where
    T: Sized,
{
    // SAFETY: We require that the delta stays in-bounds of the object, and
    // thus it cannot become null, as no legal objects can be allocated
    // in such as way that the null address is part of them.
    // And the caller promised the `delta` is sound to subtract.
    NonNull::new_unchecked(ptr.as_ptr().sub(delta))
}

/// Calculates the offset from a pointer in bytes (convenience for `.byte_offset(count as isize)`).
///
/// `count` is in units of bytes.
///
/// This is purely a convenience for casting to a `u8` pointer and
/// using [`add`] on it. See that method for documentation
/// and safety requirements.
///
/// For non-`Sized` pointees this operation changes only the data pointer,
/// leaving the metadata untouched.
#[must_use]
#[inline(always)]
#[allow(dead_code)]
#[cfg_attr(miri, track_caller)] // even without panics, this helps for Miri backtraces
pub(crate) unsafe fn byte_add<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    add(ptr.cast::<u8>(), count).cast()
}

/// Calculates the offset from a pointer in bytes (convenience for
/// `.byte_offset((count as isize).wrapping_neg())`).
///
/// `count` is in units of bytes.
///
/// This is purely a convenience for casting to a `u8` pointer and
/// using [`sub`] on it. See that method for documentation
/// and safety requirements.
///
/// For non-`Sized` pointees this operation changes only the data pointer,
/// leaving the metadata untouched.
#[must_use]
#[inline(always)]
#[allow(dead_code)]
#[cfg_attr(miri, track_caller)] // even without panics, this helps for Miri backtraces
pub(crate) unsafe fn byte_sub<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    sub(ptr.cast::<u8>(), count).cast()
}

// Must not overflow
#[inline(always)]
pub(crate) unsafe fn wrapping_byte_add<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    NonNull::new_unchecked(ptr.as_ptr().cast::<u8>().wrapping_add(count).cast())
}

// Must not overflow.
#[inline(always)]
pub(crate) unsafe fn wrapping_byte_sub<T>(ptr: NonNull<T>, count: usize) -> NonNull<T> {
    NonNull::new_unchecked(ptr.as_ptr().cast::<u8>().wrapping_sub(count).cast())
}

/// Gets the "address" portion of the pointer.
///
/// For more details see the equivalent method on a raw pointer, `pointer::addr`.
///
/// This API and its claimed semantics are part of the Strict Provenance experiment,
/// see the [`ptr` module documentation][core::ptr].
#[inline(always)]
pub(crate) fn addr<T>(ptr: NonNull<T>) -> NonZeroUsize {
    // SAFETY: The pointer is guaranteed by the type to be non-null,
    // meaning that the address will be non-zero.
    unsafe { NonZeroUsize::new_unchecked(sptr::Strict::addr(ptr.as_ptr())) }
}

/// Creates a new pointer with the given address.
///
/// For more details see the equivalent method on a raw pointer, `pointer::with_addr`.
///
/// This API and its claimed semantics are part of the Strict Provenance experiment,
/// see the [`ptr` module documentation][core::ptr].
#[must_use]
#[inline(always)]
pub(crate) fn with_addr<T>(ptr: NonNull<T>, addr: NonZeroUsize) -> NonNull<T> {
    // SAFETY: The result of `ptr::from::with_addr` is non-null because `addr` is guaranteed to be non-zero.
    unsafe { NonNull::new_unchecked(sptr::Strict::with_addr(ptr.as_ptr(), addr.get())) }
}

/// Calculates the distance between two pointers, *where it's known that
/// `self` is equal to or greater than `origin`*. The returned value is in
/// units of T: the distance in bytes is divided by `mem::size_of::<T>()`.
///
/// This computes the same value that [`offset_from`](#method.offset_from)
/// would compute, but with the added precondition that the offset is
/// guaranteed to be non-negative.  This method is equivalent to
/// `usize::try_from(self.offset_from(origin)).unwrap_unchecked()`,
/// but it provides slightly more information to the optimizer, which can
/// sometimes allow it to optimize slightly better with some backends.
///
/// This method can be though of as recovering the `count` that was passed
/// to [`add`](#method.add) (or, with the parameters in the other order,
/// to [`sub`](#method.sub)).  The following are all equivalent, assuming
/// that their safety preconditions are met:
/// ```
/// # #![feature(ptr_sub_ptr)]
/// # unsafe fn blah(ptr: *mut i32, origin: *mut i32, count: usize) -> bool {
/// ptr.sub_ptr(origin) == count
/// # &&
/// origin.add(count) == ptr
/// # &&
/// ptr.sub(count) == origin
/// # }
/// ```
///
/// # Safety
///
/// - The distance between the pointers must be non-negative (`self >= origin`)
///
/// - *All* the safety conditions of [`offset_from`](#method.offset_from)
///   apply to this method as well; see it for the full details.
///
/// Importantly, despite the return type of this method being able to represent
/// a larger offset, it's still *not permitted* to pass pointers which differ
/// by more than `isize::MAX` *bytes*.  As such, the result of this method will
/// always be less than or equal to `isize::MAX as usize`.
///
/// # Panics
///
/// This function panics if `T` is a Zero-Sized Type ("ZST").
///
/// # Examples
///
/// ```
/// #![feature(ptr_sub_ptr)]
///
/// let mut a = [0; 5];
/// let p: *mut i32 = a.as_mut_ptr();
/// unsafe {
///     let ptr1: *mut i32 = p.add(1);
///     let ptr2: *mut i32 = p.add(3);
///
///     assert_eq!(ptr2.sub_ptr(ptr1), 2);
///     assert_eq!(ptr1.add(2), ptr2);
///     assert_eq!(ptr2.sub(2), ptr1);
///     assert_eq!(ptr2.sub_ptr(ptr2), 0);
/// }
///
/// // This would be incorrect, as the pointers are not correctly ordered:
/// // ptr1.offset_from(ptr2)
#[must_use]
#[inline(always)]
pub(crate) unsafe fn sub_ptr<T>(lhs: NonNull<T>, rhs: NonNull<T>) -> usize {
    debug_assert!((addr(lhs).get() - addr(rhs).get()) % T::SIZE == 0);
    pointer::sub_ptr(lhs.as_ptr(), rhs.as_ptr())
}

#[must_use]
#[inline(always)]
pub(crate) unsafe fn byte_sub_ptr<T>(lhs: NonNull<T>, rhs: NonNull<T>) -> usize {
    sub_ptr::<u8>(lhs.cast(), rhs.cast())
}

/// Creates a non-null raw slice from a thin pointer and a length.
///
/// The `len` argument is the number of **elements**, not the number of bytes.
///
/// This function is safe, but dereferencing the return value is unsafe.
/// See the documentation of [`slice::from_raw_parts`](core::slice::from_raw_parts) for slice safety requirements.
#[must_use]
#[inline(always)]
pub(crate) const fn slice_from_raw_parts<T>(data: NonNull<T>, len: usize) -> NonNull<[T]> {
    // SAFETY: `data` is a `NonNull` pointer which is necessarily non-null
    unsafe { NonNull::new_unchecked(pointer::cast_mut(ptr::slice_from_raw_parts(data.as_ptr(), len))) }
}

#[must_use]
#[inline(always)]
pub(crate) const fn str_from_utf8(bytes: NonNull<[u8]>) -> NonNull<str> {
    unsafe { NonNull::new_unchecked(bytes.as_ptr() as *mut str) }
}

#[must_use]
#[inline(always)]
pub(crate) const fn str_bytes(str: NonNull<str>) -> NonNull<[u8]> {
    unsafe { NonNull::new_unchecked(str.as_ptr() as *mut [u8]) }
}

#[must_use]
#[inline(always)]
pub(crate) const fn str_len(str: NonNull<str>) -> usize {
    str_bytes(str).len()
}

/// See [`ptr::copy`] for semantics and safety requirements.
#[inline(always)]
pub(crate) unsafe fn copy<T>(src: NonNull<T>, dst: NonNull<T>, count: usize) {
    ptr::copy(src.as_ptr(), dst.as_ptr(), count);
}

/// See [`ptr::copy_nonoverlapping`] for semantics and safety requirements.
#[inline(always)]
pub(crate) unsafe fn copy_nonoverlapping<T>(src: NonNull<T>, dst: NonNull<T>, count: usize) {
    ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), count);
}

#[inline(always)]
pub(crate) unsafe fn result<T, E>(mut ptr: NonNull<Result<T, E>>) -> Result<NonNull<T>, NonNull<E>> {
    match ptr.as_mut() {
        Ok(ok) => Ok(ok.into()),
        Err(err) => Err(err.into()),
    }
}

#[inline(always)]
pub(crate) fn is_aligned_to(ptr: NonNull<u8>, align: usize) -> bool {
    debug_assert!(align.is_power_of_two());
    addr(ptr).get() & (align - 1) == 0
}

#[inline(always)]
pub(crate) fn as_non_null_ptr<T>(ptr: NonNull<[T]>) -> NonNull<T> {
    ptr.cast()
}

#[inline(always)]
pub(crate) fn set_ptr<T>(ptr: &mut NonNull<[T]>, new_ptr: NonNull<T>) {
    let len = ptr.len();
    *ptr = slice_from_raw_parts(new_ptr, len);
}

#[inline(always)]
pub(crate) fn set_len<T>(ptr: &mut NonNull<[T]>, new_len: usize) {
    let elem_ptr = as_non_null_ptr(*ptr);
    *ptr = slice_from_raw_parts(elem_ptr, new_len);
}

#[inline(always)]
pub(crate) unsafe fn drop_in_place<T: ?Sized>(ptr: NonNull<T>) {
    ptr.as_ptr().drop_in_place();
}

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

/// like `<NonNull<T> as From<&T>>::from` but `const`
pub(crate) const fn from_ref<T>(r: &T) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(r as *const T as *mut T) }
}
