use core::{
    fmt::Debug,
    iter::FusedIterator,
    marker::PhantomData,
    mem::{self, ManuallyDrop},
    ops::Range,
    ptr::NonNull,
    slice,
};

use crate::{BumpBox, SizedTypeProperties, polyfill::non_null};

use super::TakeOwnedSlice;

/// An iterator that moves out of an owned slice.
///
/// This `struct` is created by the `into_iter` method on
/// [`BumpBox`](BumpBox::into_iter),
/// [`FixedBumpVec`](crate::FixedBumpVec::into_iter),
/// [`MutBumpVec`](crate::MutBumpVec::into_iter) and
/// [`MutBumpVecRev`](crate::MutBumpVecRev::into_iter)
/// (provided by the [`IntoIterator`] trait).
pub struct IntoIter<'a, T> {
    ptr: NonNull<T>,
    end: NonNull<T>, // if T is a ZST this is ptr + len

    /// First field marks the lifetime.
    /// Second field marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    marker: PhantomData<(&'a (), T)>,
}

unsafe impl<T: Send> Send for IntoIter<'_, T> {}
unsafe impl<T: Sync> Sync for IntoIter<'_, T> {}

impl<T: Debug> Debug for IntoIter<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

impl<T> Default for IntoIter<'_, T> {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl<'a, T> IntoIter<'a, T> {
    /// Iterator that yields nothing.
    pub const EMPTY: Self = IntoIter {
        ptr: NonNull::dangling(),
        end: NonNull::dangling(),
        marker: PhantomData,
    };

    #[inline(always)]
    pub(crate) unsafe fn new(slice: NonNull<[T]>) -> Self {
        unsafe {
            if T::IS_ZST {
                Self::new_zst(slice.len())
            } else {
                let start = slice.cast::<T>();
                let end = start.add(slice.len());
                Self::new_range(start..end)
            }
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn new_ranged(ptr: NonNull<[T]>, range: Range<usize>) -> Self {
        unsafe {
            if T::IS_ZST {
                Self::new_zst(range.end - range.start)
            } else {
                let ptr = non_null::as_non_null_ptr(ptr);
                let start = ptr.add(range.start);
                let end = ptr.add(range.end);
                Self::new_range(start..end)
            }
        }
    }

    #[inline(always)]
    fn new_zst(len: usize) -> Self {
        assert!(T::IS_ZST);

        Self {
            ptr: NonNull::dangling(),
            end: unsafe { non_null::wrapping_byte_add(NonNull::dangling(), len) },
            marker: PhantomData,
        }
    }

    #[inline(always)]
    unsafe fn new_range(range: Range<NonNull<T>>) -> Self {
        assert!(!T::IS_ZST);

        Self {
            ptr: range.start,
            end: range.end,
            marker: PhantomData,
        }
    }

    /// Returns the exact remaining length of the iterator.
    #[must_use]
    #[inline(always)]
    pub fn len(&self) -> usize {
        if T::IS_ZST {
            self.end.addr().get().wrapping_sub(self.ptr.addr().get())
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
    #[must_use]
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len()) }
    }

    /// Returns the remaining items of this iterator as a mutable slice.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len()) }
    }

    /// Converts this iterator into a `BumpBox<[T]>`.
    // NB: `IntoIter<T>` might come from a `BumpBox<[T]>` or `MutBumpVec<T>`.
    // For `BumpBox` of course we can turn it back to a `BumpBox`.
    // For `MutBumpVec`, `'a` is a mutable borrow of the bump allocator, so we can act as if we have a
    // BumpBox allocated, for we can only mess with the bump allocator once that `BumpBox` is gone.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_slice(self) -> BumpBox<'a, [T]> {
        let this = ManuallyDrop::new(self);

        unsafe {
            let slice = NonNull::slice_from_raw_parts(this.ptr, this.len());
            BumpBox::from_raw(slice)
        }
    }
}

impl<T> Iterator for IntoIter<'_, T> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr == self.end {
            None
        } else if T::IS_ZST {
            // `ptr` has to stay aligned, so we decrement the length

            // SAFETY: self.ptr < self.end; subtracting 1 won't overflow
            self.end = unsafe { non_null::wrapping_byte_sub(self.end, 1) };

            // SAFETY: its a ZST
            Some(unsafe { mem::zeroed() })
        } else {
            unsafe {
                let old = self.ptr;
                self.ptr = self.ptr.add(1);
                Some(old.as_ptr().read())
            }
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = self.len();
        (exact, Some(exact))
    }

    #[inline(always)]
    fn count(self) -> usize {
        self.len()
    }
}

impl<T> DoubleEndedIterator for IntoIter<'_, T> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end == self.ptr {
            None
        } else if T::IS_ZST {
            // `ptr` has to stay aligned, so we decrement the length

            // SAFETY: self.ptr < self.end; subtracting 1 won't overflow
            self.end = unsafe { non_null::wrapping_byte_sub(self.end, 1) };

            // SAFETY: its a ZST
            Some(unsafe { mem::zeroed() })
        } else {
            unsafe {
                self.end = self.end.sub(1);
                Some(self.end.as_ptr().read())
            }
        }
    }
}

impl<T> ExactSizeIterator for IntoIter<'_, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        IntoIter::len(self)
    }
}

impl<T> FusedIterator for IntoIter<'_, T> {}

impl<T> Drop for IntoIter<'_, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            NonNull::slice_from_raw_parts(self.ptr, self.len()).as_ptr().drop_in_place();
        }
    }
}

// this implementation is tested in `drain.rs`
unsafe impl<T> TakeOwnedSlice for IntoIter<'_, T> {
    type Item = T;

    #[inline]
    fn owned_slice_ref(&self) -> &[Self::Item] {
        self.as_slice()
    }

    #[inline]
    fn take_owned_slice(&mut self) {
        // advance the iterator to the end without calling drop
        self.ptr = self.end;
    }
}
