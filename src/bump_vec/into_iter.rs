use core::{
    fmt::Debug,
    iter::FusedIterator,
    marker::PhantomData,
    mem,
    ptr::{self, NonNull},
    slice,
};

use crate::{
    polyfill::nonnull, BaseAllocator, BumpBox, BumpScope, FixedBumpVec, MinimumAlignment, SizedTypeProperties,
    SupportedMinimumAlignment,
};

use super::BumpVec;

macro_rules! into_iter_declaration {
    ($($allocator_parameter:tt)*) => {
        /// An iterator that moves out of a vector.
        ///
        /// This `struct` is created by the `into_iter` method on
        /// [`BumpVec`](crate::BumpVec::into_iter),
        /// (provided by the [`IntoIterator`] trait).
        // This is modelled after rust's `alloc/src/vec/into_iter.rs`
        pub struct IntoIter<
            'b,
            'a: 'b,
            T,
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        > where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
            pub(super) buf: NonNull<T>,
            pub(super) cap: usize,

            pub(super) ptr: NonNull<T>,

            /// If T is a ZST this is ptr + len.
            pub(super) end: NonNull<T>,

            pub(super) bump: &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,

            /// Marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
            pub(super) marker: PhantomData<T>,
        }
    }
}

crate::maybe_default_allocator!(into_iter_declaration);

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Debug
    for IntoIter<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

impl<'b, 'a, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    IntoIter<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
    pub fn bump(&self) -> &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        self.bump
    }

    fn as_raw_mut_slice(&mut self) -> *mut [T] {
        ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len())
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AsRef<[T]>
    for IntoIter<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Iterator
    for IntoIter<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DoubleEndedIterator
    for IntoIter<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> ExactSizeIterator
    for IntoIter<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> FusedIterator
    for IntoIter<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

#[cfg(feature = "nightly-trusted-len")]
unsafe impl<'b, 'a, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> core::iter::TrustedLen
    for IntoIter<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

#[cfg(not(no_global_oom_handling))]
impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Clone
    for IntoIter<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
    T: Clone,
{
    fn clone(&self) -> Self {
        let slice = self.as_slice();
        let boxed = self.bump.alloc_slice_clone(slice);
        let fixed = FixedBumpVec::from_init(boxed);
        let vec = BumpVec::from_parts(fixed, self.bump);
        vec.into_iter()
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for IntoIter<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn drop(&mut self) {
        struct DropGuard<'i, 'b, 'a, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>(
            &'i mut IntoIter<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
        )
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>;

        impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
            for DropGuard<'_, '_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
            fn drop(&mut self) {
                unsafe {
                    // BumpVec handles deallocation
                    let _ = BumpVec {
                        fixed: FixedBumpVec {
                            initialized: BumpBox::from_raw(nonnull::slice_from_raw_parts(self.0.buf, 0)),
                            capacity: self.0.cap,
                        },
                        bump: self.0.bump,
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
