use core::{
    fmt,
    iter::FusedIterator,
    mem::{self, ManuallyDrop},
    ops::RangeBounds,
    ptr::{self, NonNull},
};

use crate::{
    polyfill::{nonnull, slice},
    BumpBox, IntoIter, SizedTypeProperties,
};

/// A draining iterator owned slices.
///
/// This struct is created by the `drain` method on 
/// [`BumpBox`](BumpBox::drain), 
/// [`FixedBumpVec`](crate::FixedBumpVec::drain),
/// [`BumpVec`](crate::BumpVec::drain) and
/// [`MutBumpVec`](crate::MutBumpVec::drain).
/// 
/// See their documentation for more.
///
/// # Example
///
/// ```
/// use bump_scope::{ Bump, Drain };
/// let bump: Bump = Bump::new();
///
/// let mut v = bump.alloc_slice_copy(&[0, 1, 2]);
/// let iter: Drain<'_, _> = v.drain(..);
/// ```
pub struct Drain<'a, T: 'a> {
    /// Index of tail to preserve
    tail_start: usize,
    /// Length of tail
    tail_len: usize,
    /// Current remaining range to remove
    iter: IntoIter<'a, T>,
    slice: &'a mut NonNull<[T]>,
}

impl<T: fmt::Debug> fmt::Debug for Drain<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.iter.as_slice()).finish()
    }
}

impl<'a, T> Drain<'a, T> {
    pub(crate) fn new(boxed: &'a mut BumpBox<[T]>, range: impl RangeBounds<usize>) -> Drain<'a, T> {
        // Memory safety
        //
        // When the Drain is first created, it shortens the length of
        // the source slice to make sure no uninitialized or moved-from elements
        // are accessible at all if the Drain's destructor never gets to run.
        //
        // Drain will copy out the values to remove.
        // When finished, remaining tail of the vec is copied back to cover
        // the hole, and the vector length is restored to the new length.

        let len = boxed.len();
        let range = slice::range(range, ..len);

        unsafe {
            // set self.vec length's to start, to be safe in case Drain is leaked
            boxed.set_len(range.start);

            Drain {
                tail_start: range.end,
                tail_len: len - range.end,
                iter: IntoIter::new_ranged(boxed.ptr, range),
                slice: &mut boxed.ptr,
            }
        }
    }

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
    pub fn as_slice(&self) -> &[T] {
        self.iter.as_slice()
    }

    /// Keep unyielded elements in the source slice.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(drain_keep_rest)]
    ///
    /// let mut vec = vec!['a', 'b', 'c'];
    /// let mut drain = vec.drain(..);
    ///
    /// assert_eq!(drain.next().unwrap(), 'a');
    ///
    /// // This call keeps 'b' and 'c' in the vec.
    /// drain.keep_rest();
    ///
    /// // If we wouldn't call `keep_rest()`,
    /// // `vec` would be empty.
    /// assert_eq!(vec, ['b', 'c']);
    /// ```
    pub fn keep_rest(self) {
        // At this moment layout looks like this:
        //
        // [head] [yielded by next] [unyielded] [yielded by next_back] [tail]
        //        ^-- start         \_________/-- unyielded_len        \____/-- self.tail_len
        //                          ^-- unyielded_ptr                  ^-- tail
        //
        // Normally `Drop` impl would drop [unyielded] and then move [tail] to the `start`.
        // Here we want to
        // 1. Move [unyielded] to `start`
        // 2. Move [tail] to a new start at `start + len(unyielded)`
        // 3. Update length of the original vec to `len(head) + len(unyielded) + len(tail)`
        //    a. In case of ZST, this is the only thing we want to do
        // 4. Do *not* drop self, as everything is put in a consistent state already, there is nothing to do
        let mut this = ManuallyDrop::new(self);

        unsafe {
            let slice_ptr = nonnull::as_non_null_ptr(*this.slice).as_ptr();

            let start = this.slice.len();
            let tail = this.tail_start;

            let unyielded_len = this.iter.len();
            let unyielded_ptr = this.iter.as_slice().as_ptr();

            // ZSTs have no identity, so we don't need to move them around.
            if !T::IS_ZST {
                let start_ptr = slice_ptr.add(start);

                // memmove back unyielded elements
                if unyielded_ptr != start_ptr {
                    let src = unyielded_ptr;
                    let dst = start_ptr;

                    ptr::copy(src, dst, unyielded_len);
                }

                // memmove back untouched tail
                if tail != (start + unyielded_len) {
                    let src = slice_ptr.add(tail);
                    let dst = start_ptr.add(unyielded_len);
                    ptr::copy(src, dst, this.tail_len);
                }
            }

            let new_len = start + unyielded_len + this.tail_len;
            nonnull::set_len(this.slice, new_len);
        }
    }
}

impl<'a, T> AsRef<[T]> for Drain<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

unsafe impl<T: Sync> Sync for Drain<'_, T> {}
unsafe impl<T: Send> Send for Drain<'_, T> {}

impl<T> Iterator for Drain<'_, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for Drain<'_, T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

impl<T> Drop for Drain<'_, T> {
    fn drop(&mut self) {
        /// Moves back the un-`Drain`ed elements to restore the original `Vec`.
        struct DropGuard<'r, 'a, T>(&'r mut Drain<'a, T>);

        impl<'r, 'a, T> Drop for DropGuard<'r, 'a, T> {
            fn drop(&mut self) {
                if self.0.tail_len > 0 {
                    unsafe {
                        // memmove back untouched tail, update to new length
                        let slice_ptr = nonnull::as_non_null_ptr(*self.0.slice).as_ptr();

                        let start = self.0.slice.len();
                        let tail = self.0.tail_start;

                        if tail != start {
                            let src = slice_ptr.add(tail);
                            let dst = slice_ptr.add(start);
                            ptr::copy(src, dst, self.0.tail_len);
                        }

                        nonnull::set_len(self.0.slice, start + self.0.tail_len);
                    }
                }
            }
        }

        let iter = mem::take(&mut self.iter);

        if T::IS_ZST {
            // ZSTs have no identity, so we don't need to move them around, we only need to drop the correct amount.
            // this can be achieved by manipulating the slice length instead of moving values out from `iter`.
            unsafe {
                let old_len = self.slice.len();
                nonnull::set_len(self.slice, old_len + iter.len() + self.tail_len);
                nonnull::truncate(self.slice, old_len + self.tail_len);
            }

            return;
        }

        // Ensure elements are moved back into their appropriate places, even when dropping `iter` panics.
        let _guard = DropGuard(self);

        // Drops the remaining drained elements.
        drop(iter);
    }
}

impl<T> ExactSizeIterator for Drain<'_, T> {
    #[cfg(feature = "nightly-exact-size-is-empty")]
    fn is_empty(&self) -> bool {
        self.iter.is_empty()
    }
}

#[cfg(feature = "nightly-trusted-len")]
unsafe impl<T> core::iter::TrustedLen for Drain<'_, T> {}

impl<T> FusedIterator for Drain<'_, T> {}
