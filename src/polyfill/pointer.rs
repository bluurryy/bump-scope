use core::ptr;

/// Not part of std.
///
/// Putting the expression in a function helps llvm to realize that it can initialize the value
/// at this pointer instead of allocating it on the stack and then copying it over.
#[inline(always)]
pub(crate) unsafe fn write_with<T>(ptr: *mut T, f: impl FnOnce() -> T) {
    unsafe { ptr::write(ptr, f()) };
}
