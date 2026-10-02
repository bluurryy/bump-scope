use core::{
    alloc::Layout,
    fmt::Debug,
    iter::FusedIterator,
    marker::PhantomData,
    ptr::{self, NonNull},
    slice,
};

use crate::{SizedTypeProperties, polyfill::non_null, traits::BumpAllocatorTyped};

#[cfg(feature = "panic-on-alloc")]
use crate::bump_vec::slice_to_bump_vec_in;

macro_rules! non_null {
    (mut $place:expr, $t:ident) => {{ unsafe { &mut *((&raw mut $place).cast::<NonNull<$t>>()) } }};
    ($place:expr, $t:ident) => {{ unsafe { *((&raw const $place).cast::<NonNull<$t>>()) } }};
}

/// An iterator that moves out of a vector.
///
/// This `struct` is created by the `into_iter` method on
/// [`BumpVec`](crate::BumpVec::into_iter),
/// (provided by the [`IntoIterator`] trait).
// This is modelled after rust's `alloc/src/vec/into_iter.rs`
pub struct IntoIter<T, A: BumpAllocatorTyped> {
    pub(super) buf: NonNull<T>,
    pub(super) cap: usize,

    pub(super) alloc: A,
    pub(super) ptr: NonNull<T>,

    /// If T is a ZST, this is actually ptr+len. This encoding is picked so that
    /// ptr == end is a quick test for the Iterator being empty, that works
    /// for both ZST and non-ZST.
    /// For non-ZSTs the pointer is treated as `NonNull<T>`
    pub(super) end: *const T,

    /// Marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    pub(super) marker: PhantomData<T>,
}

impl<T: Debug, A: BumpAllocatorTyped> Debug for IntoIter<T, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

impl<T, A: BumpAllocatorTyped> IntoIter<T, A> {
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let vec = bump_vec![in &bump; 'a', 'b', 'c'];
    /// let mut into_iter = vec.into_iter();
    /// assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    /// let _ = into_iter.next().unwrap();
    /// assert_eq!(into_iter.as_slice(), &['b', 'c']);
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
    /// # use bump_scope::{Bump, bump_vec};
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
        &self.alloc
    }

    fn as_raw_mut_slice(&mut self) -> *mut [T] {
        ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len())
    }
}

impl<T, A: BumpAllocatorTyped> AsRef<[T]> for IntoIter<T, A> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, A: BumpAllocatorTyped> Iterator for IntoIter<T, A> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let ptr = if T::IS_ZST {
            if ptr::eq(self.ptr.as_ptr(), self.end) {
                return None;
            }
            // `ptr` has to stay where it is to remain aligned, so we reduce the length by 1 by
            // reducing the `end`.
            self.end = self.end.wrapping_byte_sub(1);
            self.ptr
        } else {
            if self.ptr == non_null!(self.end, T) {
                return None;
            }
            let old = self.ptr;
            self.ptr = unsafe { old.add(1) };
            old
        };
        Some(unsafe { ptr.read() })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = if T::IS_ZST {
            self.end.addr().wrapping_sub(self.ptr.addr().get())
        } else {
            #[allow(unused_unsafe)] // for the macro
            unsafe {
                non_null::offset_from_unsigned(non_null!(self.end, T), self.ptr)
            }
        };
        (exact, Some(exact))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }
}

impl<T, A: BumpAllocatorTyped> DoubleEndedIterator for IntoIter<T, A> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if T::IS_ZST {
            if ptr::eq(self.ptr.as_ptr(), self.end) {
                return None;
            }
            // See above for why 'ptr.offset' isn't used
            self.end = self.end.wrapping_byte_sub(1);
            // Note that even though this is next_back() we're reading from `self.ptr`, not
            // `self.end`. We track our length using the byte offset from `self.ptr` to `self.end`,
            // so the end pointer may not be suitably aligned for T.
            Some(unsafe { ptr::read(self.ptr.as_ptr()) })
        } else {
            if self.ptr == non_null!(self.end, T) {
                return None;
            }
            unsafe {
                self.end = self.end.sub(1);
                Some(ptr::read(self.end))
            }
        }
    }
}

impl<T, A: BumpAllocatorTyped> ExactSizeIterator for IntoIter<T, A> {
    #[cfg(feature = "nightly-exact-size-is-empty")]
    #[inline]
    fn is_empty(&self) -> bool {
        if T::IS_ZST {
            ptr::eq(self.ptr.as_ptr(), self.end)
        } else {
            self.ptr == non_null!(self.end, T)
        }
    }
}

impl<T, A: BumpAllocatorTyped> FusedIterator for IntoIter<T, A> {}

#[cfg(feature = "nightly-trusted-len")]
unsafe impl<T, A: BumpAllocatorTyped> core::iter::TrustedLen for IntoIter<T, A> {}

#[cfg(feature = "panic-on-alloc")]
impl<T: Clone, A: BumpAllocatorTyped + Clone> Clone for IntoIter<T, A> {
    fn clone(&self) -> Self {
        slice_to_bump_vec_in(self.as_slice(), self.alloc.clone()).into_iter()
    }
}

impl<T, A: BumpAllocatorTyped> Drop for IntoIter<T, A> {
    fn drop(&mut self) {
        struct DropGuard<'a, T, A: BumpAllocatorTyped>(&'a mut IntoIter<T, A>);

        impl<T, A: BumpAllocatorTyped> Drop for DropGuard<'_, T, A> {
            fn drop(&mut self) {
                if T::IS_ZST || self.0.cap == 0 {
                    return;
                }

                unsafe {
                    let ptr = self.0.buf.cast();
                    let layout = Layout::from_size_align_unchecked(self.0.cap * T::SIZE, T::ALIGN);
                    self.0.alloc.deallocate(ptr, layout);
                }
            }
        }

        let guard = DropGuard(self);
        // destroy the remaining elements
        unsafe {
            ptr::drop_in_place(guard.0.as_raw_mut_slice());
        }
        // now `guard` will be dropped and deallocate
    }
}
