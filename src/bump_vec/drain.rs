//! This is not part of public api.
//!
//! This exists solely for the implementation of [`Splice`](crate::bump_vec::Splice).

use core::{
    fmt::{self, Debug},
    iter::FusedIterator,
    mem::{self, ManuallyDrop},
    ptr::{self, NonNull},
    slice::{self},
};

use crate::{BaseAllocator, BumpVec, MinimumAlignment, SizedTypeProperties, SupportedMinimumAlignment};

macro_rules! drain_declaration {
    ($($allocator_parameter:tt)*) => {
        /// A draining iterator for `BumpVec<T>`.
        ///
        /// This `struct` is created by [`Vec::drain`].
        /// See its documentation for more.
        ///
        /// # Example
        ///
        /// ```
        /// let mut v = vec![0, 1, 2];
        /// let iter: std::vec::Drain<_> = v.drain(..);
        /// ```
        pub struct Drain<
            'a,
            T: 'a,
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        >
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
            /// Index of tail to preserve
            pub(super) tail_start: usize,
            /// Length of tail
            pub(super) tail_len: usize,
            /// Current remaining range to remove
            pub(super) iter: slice::Iter<'a, T>,
            pub(super) vec: NonNull<BumpVec<'a, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>,
        }
    }
}

crate::maybe_default_allocator!(drain_declaration);

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Debug
    for Drain<'_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.iter.as_slice()).finish()
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Drain<'_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
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

    /// Returns a reference to the underlying allocator.
    #[must_use]
    #[inline(always)]
    #[allow(dead_code)]
    pub fn allocator(&self) -> &A {
        unsafe { self.vec.as_ref().allocator() }
    }

    /// Keep unyielded elements in the source `Vec`.
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
    #[allow(dead_code)]
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
            let source_vec = this.vec.as_mut();

            let start = source_vec.len();
            let tail = this.tail_start;

            let unyielded_len = this.iter.len();
            let unyielded_ptr = this.iter.as_slice().as_ptr();

            // ZSTs have no identity, so we don't need to move them around.
            if !T::IS_ZST {
                let start_ptr = source_vec.as_mut_ptr().add(start);

                // memmove back unyielded elements
                if unyielded_ptr != start_ptr {
                    let src = unyielded_ptr;
                    let dst = start_ptr;

                    ptr::copy(src, dst, unyielded_len);
                }

                // memmove back untouched tail
                if tail != (start + unyielded_len) {
                    let src = source_vec.as_ptr().add(tail);
                    let dst = start_ptr.add(unyielded_len);
                    ptr::copy(src, dst, this.tail_len);
                }
            }

            source_vec.set_len(start + unyielded_len + this.tail_len);
        }
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AsRef<[T]>
    for Drain<'_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Iterator
    for Drain<'_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<T> {
        self.iter.next().map(|elt| unsafe { ptr::read(elt as *const _) })
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DoubleEndedIterator
    for Drain<'_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back().map(|elt| unsafe { ptr::read(elt as *const _) })
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for Drain<'_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn drop(&mut self) {
        /// Moves back the un-`Drain`ed elements to restore the original `Vec`.
        struct DropGuard<'r, 'a, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>(
            &'r mut Drain<'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
        )
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>;

        impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
            for DropGuard<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
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
            // this can be achieved by manipulating the Vec length instead of moving values out from `iter`.
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

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> ExactSizeIterator
    for Drain<'_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> FusedIterator
    for Drain<'_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}