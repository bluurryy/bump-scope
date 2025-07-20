use core::{
    fmt::Debug,
    iter::FusedIterator,
    marker::PhantomData,
    mem,
    ptr::{self, NonNull},
    slice,
};

use crate::{polyfill::non_null, BumpAllocatorExt, SizedTypeProperties};

/// An iterator that moves out of a vector.
///
/// This `struct` is created by the [`into_iter`] method on [`MutBumpVec`],
/// (provided by the [`IntoIterator`] trait).
///
/// [`into_iter`]: crate::MutBumpVec::into_iter
/// [`MutBumpVec`]: crate::MutBumpVec
pub struct IntoIter<T, A> {
    ptr: NonNull<T>,
    end: NonNull<T>, // if T is a ZST this is ptr + len

    // Just holding on to it so it doesn't drop.
    #[allow(dead_code)]
    allocator: A,

    /// First field marks the lifetime in the form of the allocator.
    /// Second field marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    marker: PhantomData<(A, T)>,
}

unsafe impl<T: Send, A: Send> Send for IntoIter<T, A> {}
unsafe impl<T: Sync, A: Sync> Sync for IntoIter<T, A> {}

impl<T: Debug, A: BumpAllocatorExt> Debug for IntoIter<T, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

impl<T, A> IntoIter<T, A> {
    pub(crate) unsafe fn new(slice: NonNull<[T]>, allocator: A) -> Self {
        if T::IS_ZST {
            IntoIter {
                ptr: NonNull::dangling(),
                end: unsafe { non_null::wrapping_byte_add(NonNull::dangling(), slice.len()) },
                allocator,
                marker: PhantomData,
            }
        } else {
            let start = non_null::as_non_null_ptr(slice);
            let end = non_null::add(start, slice.len());

            IntoIter {
                ptr: start,
                end,
                allocator,
                marker: PhantomData,
            }
        }
    }

    /// Returns the exact remaining length of the iterator.
    #[must_use]
    #[inline(always)]
    pub fn len(&self) -> usize {
        #![allow(clippy::cast_sign_loss)]

        if T::IS_ZST {
            non_null::addr(self.end).get().wrapping_sub(non_null::addr(self.ptr).get())
        } else {
            unsafe { non_null::offset_from_unsigned(self.end, self.ptr) }
        }
    }

    /// Returns true if the iterator is empty.
    #[must_use]
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.ptr == self.end
    }

    /// Returns the remaining items of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec};
    /// # let mut bump: Bump = Bump::new();
    /// let vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    /// let mut into_iter = vec.into_iter();
    /// assert_eq!(into_iter.as_slice(), &[1, 2, 3]);
    /// assert_eq!(into_iter.next(), Some(1));
    /// assert_eq!(into_iter.as_slice(), &[2, 3]);
    /// assert_eq!(into_iter.next_back(), Some(3));
    /// assert_eq!(into_iter.as_slice(), &[2]);
    /// ```
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len()) }
    }

    /// Returns the remaining items of this iterator as a mutable slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec};
    /// # let mut bump: Bump = Bump::new();
    /// let vec = mut_bump_vec![in &mut bump; 'a', 'b', 'c'];
    /// let mut into_iter = vec.into_iter();
    /// assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    /// into_iter.as_mut_slice()[2] = 'z';
    /// assert_eq!(into_iter.next().unwrap(), 'a');
    /// assert_eq!(into_iter.next().unwrap(), 'b');
    /// assert_eq!(into_iter.next().unwrap(), 'z');
    /// ```
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { &mut *self.as_raw_mut_slice() }
    }

    fn as_raw_mut_slice(&mut self) -> *mut [T] {
        ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len())
    }
}

impl<T, A> AsRef<[T]> for IntoIter<T, A> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, A> Iterator for IntoIter<T, A> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr == self.end {
            None
        } else if T::IS_ZST {
            // `ptr` has to stay where it is to remain aligned, so we reduce the length by 1 by
            // reducing the `end`.
            self.end = unsafe { non_null::wrapping_byte_sub(self.end, 1) };

            // Make up a value of this ZST.
            Some(unsafe { mem::zeroed() })
        } else {
            let old = self.ptr;
            self.ptr = unsafe { non_null::add(self.ptr, 1) };

            Some(unsafe { old.as_ptr().read() })
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = self.len();
        (exact, Some(exact))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }
}

impl<T, A> DoubleEndedIterator for IntoIter<T, A> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end == self.ptr {
            None
        } else if T::IS_ZST {
            // See above for why 'ptr.offset' isn't used
            self.end = unsafe { non_null::wrapping_byte_sub(self.end, 1) };

            // Make up a value of this ZST.
            Some(unsafe { mem::zeroed() })
        } else {
            self.end = unsafe { non_null::sub(self.end, 1) };

            Some(unsafe { self.end.as_ptr().read() })
        }
    }
}

impl<T, A> ExactSizeIterator for IntoIter<T, A> {}
impl<T, A> FusedIterator for IntoIter<T, A> {}

#[cfg(feature = "nightly-trusted-len")]
unsafe impl<T, A> core::iter::TrustedLen for IntoIter<T, A> {}

impl<T, A> Drop for IntoIter<T, A> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            non_null::slice_from_raw_parts(self.ptr, self.len()).as_ptr().drop_in_place();
        }
    }
}
