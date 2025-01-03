use core::ptr::NonNull;

use crate::polyfill::nonnull;

// Set the length of the vec when the `SetLenOnDropByPtr` value goes out of scope.
//
// The idea is: The length field in SetLenOnDropByPtr is a local variable
// that the optimizer will see does not alias with any stores through the MutBumpVec's data
// pointer. This is a workaround for alias analysis issue #32155
pub(super) struct SetLenOnDropByPtr<'a, T> {
    slice: &'a mut NonNull<[T]>,
    local_len: usize,
}

impl<'a, T> SetLenOnDropByPtr<'a, T> {
    #[inline]
    pub(super) fn new(slice: &'a mut NonNull<[T]>) -> Self {
        SetLenOnDropByPtr {
            local_len: slice.len(),
            slice,
        }
    }

    #[inline]
    pub(super) fn increment_len(&mut self, increment: usize) {
        self.local_len += increment;
    }

    #[inline]
    pub(super) fn current_len(&self) -> usize {
        self.local_len
    }
}

impl<T> Drop for SetLenOnDropByPtr<'_, T> {
    #[inline]
    fn drop(&mut self) {
        nonnull::set_len(self.slice, self.local_len);
    }
}
