use core::{
    alloc::Layout,
    fmt::Debug,
    iter::FusedIterator,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
    slice,
};

use crate::{
    polyfill::nonnull, raw_fixed_bump_vec::RawFixedBumpVec, BumpAllocator, BumpBox, FixedBumpVec, SizedTypeProperties,
};

use super::BumpVec;

/// An iterator that moves out of a vector.
///
/// This `struct` is created by the `into_iter` method on
/// [`BumpVec`](crate::BumpVec::into_iter),
/// (provided by the [`IntoIterator`] trait).
// This is modelled after rust's `alloc/src/vec/into_iter.rs`
pub struct IntoIter<T, A: BumpAllocator> {
    pub(super) buf: NonNull<T>,
    pub(super) cap: usize,

    pub(super) ptr: NonNull<T>,

    /// If T is a ZST this is ptr + len.
    pub(super) end: NonNull<T>,

    pub(super) allocator: A,

    /// Marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    pub(super) marker: PhantomData<T>,
}

impl<T: Debug, A: BumpAllocator> Debug for IntoIter<T, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

impl<T, A: BumpAllocator> IntoIter<T, A> {
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let vec = bump_vec![in &bump; 1, 2, 3];
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
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let vec = bump_vec![in &bump; 'a', 'b', 'c'];
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

    /// Returns a reference to the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        &self.allocator
    }

    fn as_raw_mut_slice(&mut self) -> *mut [T] {
        ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len())
    }
}

impl<T, A: BumpAllocator> AsRef<[T]> for IntoIter<T, A> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, A: BumpAllocator> Iterator for IntoIter<T, A> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr == self.end {
            None
        } else if T::IS_ZST {
            // `ptr` has to stay where it is to remain aligned, so we reduce the length by 1 by
            // reducing the `end`.
            self.end = unsafe { nonnull::wrapping_byte_sub(self.end, 1) };

            // Make up a value of this ZST.
            Some(unsafe { mem::zeroed() })
        } else {
            let old = self.ptr;
            self.ptr = unsafe { nonnull::add(self.ptr, 1) };

            Some(unsafe { old.as_ptr().read() })
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = if T::IS_ZST {
            nonnull::addr(self.end).get().wrapping_sub(nonnull::addr(self.ptr).get())
        } else {
            unsafe { nonnull::sub_ptr(self.end, self.ptr) }
        };
        (exact, Some(exact))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }
}

impl<T, A: BumpAllocator> DoubleEndedIterator for IntoIter<T, A> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end == self.ptr {
            None
        } else if T::IS_ZST {
            // See above for why 'ptr.offset' isn't used
            self.end = unsafe { nonnull::wrapping_byte_sub(self.end, 1) };

            // Make up a value of this ZST.
            Some(unsafe { mem::zeroed() })
        } else {
            self.end = unsafe { nonnull::sub(self.end, 1) };

            Some(unsafe { self.end.as_ptr().read() })
        }
    }
}

impl<T, A: BumpAllocator> ExactSizeIterator for IntoIter<T, A> {}
impl<T, A: BumpAllocator> FusedIterator for IntoIter<T, A> {}

#[cfg(feature = "nightly-trusted-len")]
unsafe impl<T, A: BumpAllocator> core::iter::TrustedLen for IntoIter<T, A> {}

#[cfg(not(no_global_oom_handling))]
impl<T: Clone, A: BumpAllocator + Clone> Clone for IntoIter<T, A> {
    fn clone(&self) -> Self {
        let allocator = self.allocator.clone();
        let ptr = self.allocator.allocate_slice::<MaybeUninit<T>>(self.len());
        let slice = nonnull::slice_from_raw_parts(ptr, self.len());
        let boxed = unsafe { BumpBox::from_raw(slice) };
        let boxed = boxed.init_clone(self.as_slice());
        let fixed = FixedBumpVec::from_init(boxed);
        let fixed = unsafe { RawFixedBumpVec::from_cooked(fixed) };
        let vec = BumpVec { fixed, allocator };
        vec.into_iter()
    }
}

impl<T, A: BumpAllocator> Drop for IntoIter<T, A> {
    fn drop(&mut self) {
        struct DropGuard<'a, T, A: BumpAllocator>(&'a mut IntoIter<T, A>);

        impl<T, A: BumpAllocator> Drop for DropGuard<'_, T, A> {
            fn drop(&mut self) {
                unsafe {
                    let ptr = self.0.buf.cast();
                    let layout = Layout::from_size_align_unchecked(self.0.cap * T::SIZE, T::ALIGN);
                    self.0.allocator.deallocate(ptr, layout);
                }
            }
        }

        let guard = DropGuard(self);
        // destroy the remaining elements
        unsafe {
            ptr::drop_in_place(guard.0.as_raw_mut_slice());
        }
        // now `guard` will be dropped and do the rest
    }
}
