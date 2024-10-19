#![cfg(not(no_global_oom_handling))]

use core::{ptr, slice};

use crate::{BaseAllocator, MinimumAlignment, SupportedMinimumAlignment};

use super::Drain;

macro_rules! splice_declaration {
    ($($allocator_parameter:tt)*) => {
        /// A splicing iterator for `Vec`.
        ///
        /// This struct is created by [`Vec::splice()`].
        /// See its documentation for more.
        ///
        /// # Example
        ///
        /// ```
        /// let mut v = vec![0, 1, 2];
        /// let new = [7, 8];
        /// let iter: std::vec::Splice<'_, _> = v.splice(1.., new);
        /// ```
        #[derive(Debug)]
        pub struct Splice<
            'a,
            I,
            A,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        > where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
            I: Iterator + 'a,
        {
            pub(super) drain: Drain<'a, I::Item, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
            pub(super) replace_with: I,
        }
    };
}

crate::maybe_default_allocator!(splice_declaration);

impl<I, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Iterator
    for Splice<'_, I, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    I: Iterator,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.drain.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.drain.size_hint()
    }
}

impl<I, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DoubleEndedIterator
    for Splice<'_, I, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    I: Iterator,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.drain.next_back()
    }
}

impl<I, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> ExactSizeIterator
    for Splice<'_, I, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    I: Iterator,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<I, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for Splice<'_, I, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    I: Iterator,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn drop(&mut self) {
        self.drain.by_ref().for_each(drop);
        // At this point draining is done and the only remaining tasks are splicing
        // and moving things into the final place.
        // Which means we can replace the slice::Iter with pointers that won't point to deallocated
        // memory, so that Drain::drop is still allowed to call iter.len(), otherwise it would break
        // the ptr.sub_ptr contract.
        self.drain.iter = <[I::Item]>::iter(&[]);

        unsafe {
            if self.drain.tail_len == 0 {
                self.drain.vec.as_mut().extend(self.replace_with.by_ref());
                return;
            }

            // First fill the range left by drain().
            if !self.drain.fill(&mut self.replace_with) {
                return;
            }

            // There may be more elements. Use the lower bound as an estimate.
            // FIXME: Is the upper bound a better guess? Or something else?
            let (lower_bound, _upper_bound) = self.replace_with.size_hint();
            if lower_bound > 0 {
                self.drain.move_tail(lower_bound);
                if !self.drain.fill(&mut self.replace_with) {
                    return;
                }
            }

            // Collect any remaining elements.
            // This is a zero-length vector which does not allocate if `lower_bound` was exact.
            let mut collected = self.drain.vec.as_ref().bump.alloc_iter(&mut self.replace_with).into_iter();

            // Now we have an exact count.
            #[allow(clippy::len_zero)]
            if collected.len() > 0 {
                self.drain.move_tail(collected.len());
                let filled = self.drain.fill(&mut collected);
                debug_assert!(filled);
                debug_assert_eq!(collected.len(), 0);
            }
        }
        // Let `Drain::drop` move the tail back if necessary and restore `vec.len`.
    }
}

/// Private helper methods for `Splice::drop`
impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Drain<'_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// The range from `self.vec.len` to `self.tail_start` contains elements
    /// that have been moved out.
    /// Fill that range as much as possible with new elements from the `replace_with` iterator.
    /// Returns `true` if we filled the entire range. (`replace_with.next()` didn’t return `None`.)
    unsafe fn fill<I: Iterator<Item = T>>(&mut self, replace_with: &mut I) -> bool {
        let vec = self.vec.as_mut();
        let range_start = vec.len();
        let range_end = self.tail_start;
        let range_slice = slice::from_raw_parts_mut(vec.as_mut_ptr().add(range_start), range_end - range_start);

        for place in range_slice {
            if let Some(new_item) = replace_with.next() {
                ptr::write(place, new_item);
                vec.inc_len(1);
            } else {
                return false;
            }
        }
        true
    }

    /// Makes room for inserting more elements before the tail.
    #[cfg(not(no_global_oom_handling))]
    unsafe fn move_tail(&mut self, additional: usize) {
        let vec = self.vec.as_mut();
        let len = self.tail_start + self.tail_len;
        vec.buf_reserve(len, additional);

        let new_tail_start = self.tail_start + additional;

        let src = vec.as_ptr().add(self.tail_start);
        let dst = vec.as_mut_ptr().add(new_tail_start);
        ptr::copy(src, dst, self.tail_len);

        self.tail_start = new_tail_start;
    }
}
