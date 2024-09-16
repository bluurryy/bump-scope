use crate::{polyfill::nonnull, BumpBox};
use core::ptr::NonNull;

/// An iterator which uses a closure to determine if an element should be removed.
///
/// This struct is created by the `extract_if` method on
/// [`BumpBox`](BumpBox::extract_if),
/// [`FixedBumpVec`](crate::FixedBumpVec::extract_if),
/// [`BumpVec`](crate::BumpVec::extract_if) and
/// [`MutBumpVec`](crate::MutBumpVec::extract_if).
///
/// See their documentation for more.
///
/// # Example
///
/// ```
/// use bump_scope::{ Bump, ExtractIf };
/// let bump: Bump = Bump::new();
///
/// let mut v = bump.alloc_slice_copy(&[0, 1, 2]);
/// let iter: ExtractIf<'_, _, _> = v.extract_if(|x| *x % 2 == 0);
/// ```
#[derive(Debug)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ExtractIf<'a, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    ptr: &'a mut NonNull<[T]>,
    index: usize,
    drained_count: usize,
    original_len: usize,
    filter: F,
}

impl<'a, T, F> ExtractIf<'a, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    pub(crate) fn new<'a2>(boxed: &'a mut BumpBox<'a2, [T]>, filter: F) -> Self {
        // When the ExtractIf is first created, it shortens the length of
        // the source boxed slice to make sure no uninitialized or moved-from elements
        // are accessible at all if the ExtractIf's destructor never gets to run.
        //
        // The 'a2 lifetime is shortened to 'a even though &'b mut BumpBox<'a2> is invariant
        // over 'a2. We are careful not to expose any api where that could cause issues.

        let ptr = &mut boxed.ptr;
        let len = ptr.len();

        nonnull::set_len(ptr, 0);

        Self {
            ptr,
            index: 0,
            drained_count: 0,
            original_len: len,
            filter,
        }
    }
}

impl<T, F> Iterator for ExtractIf<'_, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        unsafe {
            while self.index < self.original_len {
                let start_ptr = nonnull::as_non_null_ptr(*self.ptr);
                let mut value_ptr = nonnull::add(start_ptr, self.index);

                let drained = (self.filter)(value_ptr.as_mut());

                // Update the index *after* the predicate is called. If the index
                // is updated prior and the predicate panics, the element at this
                // index would be leaked.
                self.index += 1;

                if drained {
                    self.drained_count += 1;
                    return Some(value_ptr.as_ptr().read());
                } else if self.drained_count > 0 {
                    let src = value_ptr;
                    let dst = nonnull::sub(value_ptr, self.drained_count);
                    nonnull::copy_nonoverlapping(src, dst, 1);
                }
            }
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.original_len - self.index))
    }
}

impl<T, F> Drop for ExtractIf<'_, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    fn drop(&mut self) {
        unsafe {
            if self.index < self.original_len && self.drained_count > 0 {
                // This is a pretty messed up state, and there isn't really an
                // obviously right thing to do. We don't want to keep trying
                // to execute `pred`, so we just backshift all the unprocessed
                // elements and tell the vec that they still exist. The backshift
                // is required to prevent a double-drop of the last successfully
                // drained item prior to a panic in the predicate.
                let ptr = nonnull::as_non_null_ptr(*self.ptr);
                let src = nonnull::add(ptr, self.index);
                let dst = nonnull::sub(src, self.drained_count);
                let tail_len = self.original_len - self.index;
                nonnull::copy(src, dst, tail_len);
            }

            nonnull::set_len(self.ptr, self.original_len - self.drained_count);
        }
    }
}
