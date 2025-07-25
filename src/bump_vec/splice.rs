#![cfg(feature = "panic-on-alloc")]

use core::{ptr, slice};

use crate::{BumpAllocatorExt, BumpVec, destructure::destructure};

use super::Drain;

/// A splicing iterator for `BumpVec`.
///
/// This struct is created by [`BumpVec::splice()`].
/// See its documentation for more.
///
/// # Example
///
/// ```
/// # use bump_scope::{Bump, bump_vec};
/// # let bump: Bump = Bump::new();
/// let mut v = bump_vec![in &bump; 0, 1, 2];
/// let new = [7, 8];
/// let old = bump.alloc_iter_exact(v.splice(1.., new));
/// assert_eq!(old, [1, 2]);
/// assert_eq!(v, [0, 7, 8]);
/// ```
///
/// [`BumpVec::splice()`]: crate::BumpVec::splice
#[derive(Debug)]
pub struct Splice<'a, I: Iterator + 'a, A: BumpAllocatorExt> {
    pub(super) drain: Drain<'a, I::Item, A>,
    pub(super) replace_with: I,
}

impl<I: Iterator, A: BumpAllocatorExt> Iterator for Splice<'_, I, A> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.drain.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.drain.size_hint()
    }
}

impl<I: Iterator, A: BumpAllocatorExt> DoubleEndedIterator for Splice<'_, I, A> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.drain.next_back()
    }
}

impl<I: Iterator, A: BumpAllocatorExt> ExactSizeIterator for Splice<'_, I, A> {}

impl<I: Iterator, A: BumpAllocatorExt> Drop for Splice<'_, I, A> {
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
            // STD-FIXME: Is the upper bound a better guess? Or something else?
            let (lower_bound, _upper_bound) = self.replace_with.size_hint();
            if lower_bound > 0 {
                self.drain.move_tail(lower_bound);
                if !self.drain.fill(&mut self.replace_with) {
                    return;
                }
            }

            // Collect any remaining elements.
            // This is a zero-length vector which does not allocate if `lower_bound` was exact.
            let collected = BumpVec::from_iter_in(&mut self.replace_with, self.drain.vec.as_ref().allocator());

            // We can't use `into_fixed_vec` here because that would require a
            // `BumpAllocatorScope<'a>` instead of just a `BumpAllocator`.
            destructure!(let BumpVec::<I::Item, &A> { fixed: collected } = collected);
            let mut collected = collected.cook().into_iter();

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
impl<T, A: BumpAllocatorExt> Drain<'_, T, A> {
    /// The range from `self.vec.len` to `self.tail_start` contains elements
    /// that have been moved out.
    /// Fill that range as much as possible with new elements from the `replace_with` iterator.
    /// Returns `true` if we filled the entire range. (`replace_with.next()` didn’t return `None`.)
    unsafe fn fill<I: Iterator<Item = T>>(&mut self, replace_with: &mut I) -> bool {
        unsafe {
            let vec = self.vec.as_mut();
            let range_start = vec.len();
            let range_end = self.tail_start;
            let range_slice = slice::from_raw_parts_mut(vec.as_mut_ptr().add(range_start), range_end - range_start);

            for place in range_slice {
                match replace_with.next() {
                    Some(new_item) => {
                        ptr::write(place, new_item);
                        vec.inc_len(1);
                    }
                    _ => {
                        return false;
                    }
                }
            }
            true
        }
    }

    /// Makes room for inserting more elements before the tail.
    unsafe fn move_tail(&mut self, additional: usize) {
        unsafe {
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
}
