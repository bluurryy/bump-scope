#![cfg(feature = "panic-on-alloc")]
//! This is not part of public api.
//!
//! This exists solely for the implementation of [`Splice`](crate::bump_vec::Splice).

use core::{
    fmt::{self, Debug},
    iter::FusedIterator,
    mem,
    ptr::{self, NonNull},
    slice::{self},
};

use crate::{BumpAllocatorExt, BumpVec, SizedTypeProperties};

/// A draining iterator for `BumpVec<T>`.
///
/// This `struct` is not directly created by any method.
/// It is an implementation detail of `Splice`.
///
/// The implementation of `drain` does not need a pointer to the whole `BumpVec`
/// (and all the generic parameters that come with it).
/// That's why we return `owned_slice::Drain` from `BumpVec::drain`.
///
/// However just like the standard library we use a `Drain` implementation to implement
/// `Splice`. For this particular case we *do* need a pointer to `BumpVec`.
pub struct Drain<'a, T: 'a, A: BumpAllocatorExt> {
    /// Index of tail to preserve
    pub(super) tail_start: usize,
    /// Length of tail
    pub(super) tail_len: usize,
    /// Current remaining range to remove
    pub(super) iter: slice::Iter<'a, T>,
    pub(super) vec: NonNull<BumpVec<T, A>>,
}

impl<T: Debug, A: BumpAllocatorExt> Debug for Drain<'_, T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.iter.as_slice()).finish()
    }
}

impl<T, A: BumpAllocatorExt> Drain<'_, T, A> {
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vec = vec!['a', 'b', 'c'];
    /// let mut drain = vec.drain(..);
    /// assert_eq!(drain.as_slice(), &['a', 'b', 'c']);
    /// let _ = drain.next().unwrap();
    /// assert_eq!(drain.as_slice(), &['b', 'c']);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        self.iter.as_slice()
    }
}

impl<T, A: BumpAllocatorExt> AsRef<[T]> for Drain<'_, T, A> {
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, A: BumpAllocatorExt> Iterator for Drain<'_, T, A> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<T> {
        self.iter.next().map(|elt| unsafe { ptr::read(elt) })
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T, A: BumpAllocatorExt> DoubleEndedIterator for Drain<'_, T, A> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back().map(|elt| unsafe { ptr::read(elt) })
    }
}

impl<T, A: BumpAllocatorExt> Drop for Drain<'_, T, A> {
    #[inline]
    fn drop(&mut self) {
        /// Moves back the un-`Drain`ed elements to restore the original vector.
        struct DropGuard<'r, 'a, T, A: BumpAllocatorExt>(&'r mut Drain<'a, T, A>);

        impl<T, A: BumpAllocatorExt> Drop for DropGuard<'_, '_, T, A> {
            fn drop(&mut self) {
                if self.0.tail_len > 0 {
                    unsafe {
                        let source_vec = self.0.vec.as_mut();
                        // memmove back untouched tail, update to new length
                        let start = source_vec.len();
                        let tail = self.0.tail_start;
                        if tail != start {
                            let src = source_vec.as_ptr().add(tail);
                            let dst = source_vec.as_mut_ptr().add(start);
                            ptr::copy(src, dst, self.0.tail_len);
                        }
                        source_vec.set_len(start + self.0.tail_len);
                    }
                }
            }
        }

        let iter = mem::replace(&mut self.iter, [].iter());
        let drop_len = iter.len();

        let mut vec = self.vec;

        if T::IS_ZST {
            // ZSTs have no identity, so we don't need to move them around, we only need to drop the correct amount.
            // this can be achieved by manipulating the vector length instead of moving values out from `iter`.
            unsafe {
                let vec = vec.as_mut();
                let old_len = vec.len();
                vec.set_len(old_len + drop_len + self.tail_len);
                vec.truncate(old_len + self.tail_len);
            }

            return;
        }

        // ensure elements are moved back into their appropriate places, even when drop_in_place panics
        let _guard = DropGuard(self);

        if drop_len == 0 {
            return;
        }

        // as_slice() must only be called when iter.len() is > 0 because
        // vec::Splice modifies vec::Drain fields and may grow the vec which would invalidate
        // the iterator's internal pointers. Creating a reference to deallocated memory
        // is invalid even when it is zero-length
        let drop_ptr = iter.as_slice().as_ptr();

        #[allow(clippy::cast_sign_loss)]
        unsafe {
            // drop_ptr comes from a slice::Iter which only gives us a &[T] but for drop_in_place
            // a pointer with mutable provenance is necessary. Therefore we must reconstruct
            // it from the original vec but also avoid creating a &mut to the front since that could
            // invalidate raw pointers to it which some unsafe code might rely on.
            let vec_ptr = vec.as_mut().as_mut_ptr();
            let drop_offset = drop_ptr.offset_from(vec_ptr) as usize;
            let to_drop = ptr::slice_from_raw_parts_mut(vec_ptr.add(drop_offset), drop_len);
            ptr::drop_in_place(to_drop);
        }
    }
}

impl<T, A: BumpAllocatorExt> ExactSizeIterator for Drain<'_, T, A> {}

impl<T, A: BumpAllocatorExt> FusedIterator for Drain<'_, T, A> {}
