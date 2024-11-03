use core::{fmt, iter::FusedIterator, ptr::NonNull, str::Chars};

use crate::BumpBox;

/// A draining iterator for an owned string.
///
/// This struct is created by the `drain` method on
/// [`BumpBox<str>`](BumpBox<str>::drain),
/// [`FixedBumpString`](crate::FixedBumpString::drain),
/// [`BumpString`](crate::BumpString::drain) and
/// [`MutBumpString`](crate::MutBumpString::drain).
///
/// See their documentation for more.
pub struct Drain<'a> {
    /// Will be used as `&'a mut BumpBox<str>` in the destructor
    pub(crate) string: NonNull<BumpBox<'a, str>>,
    /// Start of part to remove
    pub(crate) start: usize,
    /// End of part to remove
    pub(crate) end: usize,
    /// Current remaining range to remove
    pub(crate) iter: Chars<'a>,
}

impl fmt::Debug for Drain<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_str()).finish()
    }
}

unsafe impl Sync for Drain<'_> {}
unsafe impl Send for Drain<'_> {}

impl Drop for Drain<'_> {
    fn drop(&mut self) {
        unsafe {
            // Use `BumpBox::<[u8]>::drain`. "Reaffirm" the bounds checks to avoid
            // panic code being inserted again.
            let self_vec = self.string.as_mut().as_mut_bytes();

            if self.start <= self.end && self.end <= self_vec.len() {
                self_vec.drain(self.start..self.end);
            }
        }
    }
}

impl Drain<'_> {
    /// Returns the remaining (sub)string of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump };
    /// # let bump: Bump = Bump::new();    ///
    /// let mut s = bump.alloc_str("abc");
    /// let mut drain = s.drain(..);
    /// assert_eq!(drain.as_str(), "abc");
    /// let _ = drain.next().unwrap();
    /// assert_eq!(drain.as_str(), "bc");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.iter.as_str()
    }
}

impl AsRef<str> for Drain<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for Drain<'_> {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl Iterator for Drain<'_> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn last(mut self) -> Option<char> {
        self.next_back()
    }
}

impl DoubleEndedIterator for Drain<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<char> {
        self.iter.next_back()
    }
}

impl FusedIterator for Drain<'_> {}
