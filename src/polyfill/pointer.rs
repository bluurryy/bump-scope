use crate::assume_unchecked;
use core::{mem, ptr};

#[must_use]
#[inline(always)]
pub(crate) fn from_ref<T: ?Sized>(r: &T) -> *const T {
    r
}

#[must_use]
#[inline(always)]
pub(crate) fn from_mut<T: ?Sized>(r: &mut T) -> *mut T {
    r
}

/// Returns a raw pointer to the slice's buffer.
///
/// This is equivalent to casting `self` to `*mut T`, but more type-safe.
#[inline(always)]
#[allow(dead_code)]
pub(crate) const fn as_mut_ptr<T>(ptr: *mut [T]) -> *mut T {
    ptr.cast()
}

#[must_use]
#[inline(always)]
pub(crate) unsafe fn len<T>(ptr: *const [T]) -> usize {
    (*ptr).len()
}

/// Calculates the distance between two pointers, *where it's known that
/// `self` is equal to or greater than `origin`*. The returned value is in
/// units of T: the distance in bytes is divided by `mem::size_of::<T>()`.
///
/// This computes the same value that [`offset_from`](#method.offset_from)
/// would compute, but with the added precondition that the offset is
/// [guaranteed allocated]o be non-negative.  This method is equivalent to
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
/// # unsafe fn blah(ptr: *const i32, origin: *const i32, count: usize) -> bool {
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
/// let a = [0; 5];
/// let ptr1: *const i32 = &a[1];
/// let ptr2: *const i32 = &a[3];
/// unsafe {
///     assert_eq!(ptr2.sub_ptr(ptr1), 2);
///     assert_eq!(ptr1.add(2), ptr2);
///     assert_eq!(ptr2.sub(2), ptr1);
///     assert_eq!(ptr2.sub_ptr(ptr2), 0);
/// }
///
/// // This would be incorrect, as the pointers are not correctly ordered:
/// // ptr1.sub_ptr(ptr2)
/// ```
#[inline]
#[cfg_attr(miri, track_caller)] // even without panics, this helps for Miri backtraces
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::checked_conversions)]
pub(crate) unsafe fn sub_ptr<T>(lhs: *const T, rhs: *const T) -> usize {
    assume_unchecked(lhs >= rhs);
    let pointee_size = mem::size_of::<T>();
    assert!(0 < pointee_size && pointee_size <= isize::MAX as usize);
    lhs.offset_from(rhs) as usize
}

// Putting the expression in a function helps llvm to realize that it can initialize the value
// at this pointer instead of allocating it on the stack and then copying it over.
#[inline(always)]
pub(crate) unsafe fn write_with<T>(ptr: *mut T, f: impl FnOnce() -> T) {
    ptr::write(ptr, f());
}

/// Changes constness without changing the type.
///
/// This is a bit safer than `as` because it wouldn't silently change the type if the code is
/// refactored.
#[inline(always)]
pub(crate) const fn cast_mut<T: ?Sized>(ptr: *const T) -> *mut T {
    ptr as _
}
