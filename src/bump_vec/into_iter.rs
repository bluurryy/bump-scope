use crate::{
    bump_allocator::{BumpAllocatorExt, LifetimeMarker},
    infallible,
    polyfill::{nonnull, pointer},
    BumpAllocator, BumpBox, FixedBumpVec, SizedTypeProperties,
};
use core::{
    fmt::Debug,
    iter::FusedIterator,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr::{self, addr_of_mut, NonNull},
    slice,
};

use super::BumpVec;

/// An iterator that moves out of a vector.
///
/// This `struct` is created by the `into_iter` method on
/// [`BumpVec`](crate::BumpVec::into_iter),
/// (provided by the [`IntoIterator`] trait).
// This is modelled after rust's `alloc/src/vec/into_iter.rs`
pub struct IntoIter<'a, T, A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
    pub(super) buf: NonNull<T>,
    pub(super) cap: usize,

    pub(super) ptr: NonNull<T>,

    /// If T is a ZST this is ptr + len.
    pub(super) end: NonNull<T>,

    pub(super) bump: A,

    /// First field marks the lifetime.
    /// Second field marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    pub(super) marker: PhantomData<(&'a (), T)>,
}

impl<'a, T, A> Debug for IntoIter<'a, T, A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

impl<'a, T, A> IntoIter<'a, T, A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let vec = bump_vec![in bump; 1, 2, 3];
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
    /// let vec = bump_vec![in bump; 'a', 'b', 'c'];
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

    #[doc = include_str!("../docs/bump.md")]
    #[must_use]
    #[inline(always)]
    pub fn bump(&self) -> &A {
        &self.bump
    }

    fn as_raw_mut_slice(&mut self) -> *mut [T] {
        ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len())
    }
}

impl<'a, T, A> AsRef<[T]> for IntoIter<'a, T, A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<'a, T, A> Iterator for IntoIter<'a, T, A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
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

impl<'a, T, A> DoubleEndedIterator for IntoIter<'a, T, A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
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

impl<'a, T, A> ExactSizeIterator for IntoIter<'a, T, A> where A: BumpAllocator<Lifetime = LifetimeMarker<'a>> {}

impl<'a, T, A> FusedIterator for IntoIter<'a, T, A> where A: BumpAllocator<Lifetime = LifetimeMarker<'a>> {}

#[cfg(feature = "nightly-trusted-len")]
unsafe impl<'a, T, A> core::iter::TrustedLen for IntoIter<'a, T, A> where A: BumpAllocator<Lifetime = LifetimeMarker<'a>> {}

#[cfg(not(no_global_oom_handling))]
impl<'a, T, A> Clone for IntoIter<'a, T, A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>> + Clone,
    T: Clone,
{
    fn clone(&self) -> Self {
        let slice = self.as_slice();
        let boxed = infallible(self.bump.clone_slice(slice));
        let fixed = FixedBumpVec::from_init(boxed);
        let vec = BumpVec::from_parts(fixed, self.bump.clone());
        vec.into_iter()
    }
}

impl<'a, T, A> Drop for IntoIter<'a, T, A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
    fn drop(&mut self) {
        struct DropGuard<'i, 'a, T, A>(&'i mut IntoIter<'a, T, A>)
        where
            A: BumpAllocator<Lifetime = LifetimeMarker<'a>>;

        impl<'i, 'a, T, A> Drop for DropGuard<'i, 'a, T, A>
        where
            A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
        {
            fn drop(&mut self) {
                unsafe {
                    // BumpVec handles deallocation
                    let mut this = MaybeUninit::new(pointer::from_mut(self.0).read());

                    let this_buf = addr_of_mut!((*this.as_mut_ptr()).buf);
                    let this_cap = addr_of_mut!((*this.as_mut_ptr()).cap);
                    let this_bump = addr_of_mut!((*this.as_mut_ptr()).bump);

                    let buf = this_buf.read();
                    let capacity = this_cap.read();
                    let bump = this_bump.read();

                    let _ = BumpVec {
                        fixed: FixedBumpVec {
                            initialized: BumpBox::from_raw(nonnull::slice_from_raw_parts(buf, 0)),
                            capacity,
                        },
                        bump,
                    };
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
