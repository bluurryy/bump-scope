use core::{
    borrow::{Borrow, BorrowMut},
    fmt::{self, Debug, Display},
    hash::Hash,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, Range, RangeBounds},
    ptr, str,
};

use crate::{error_behavior_generic_methods_if, polyfill, BumpBox, ErrorBehavior, FixedBumpVec, FromUtf8Error};

/// A [`BumpString`](crate::BumpString) but with a fixed capacity.
///
/// It can be constructed with [`alloc_fixed_string`] or from a `BumpBox` via [`from_init`] or [`from_uninit`].
///
/// # Examples
/// ```
/// use bump_scope::Bump;
/// let mut bump: Bump = Bump::new();
/// let mut string = bump.alloc_fixed_string(9);
///
/// string.push_str("foo");
/// string.push_str("bar");
/// string.push_str("baz");
///
/// let str = string.into_str();
///
/// assert_eq!(str, "foobarbaz");
/// ```
///
/// [`alloc_fixed_string`]: crate::Bump::alloc_fixed_string
/// [`from_uninit`]: FixedBumpString::from_uninit
/// [`from_init`]: FixedBumpString::from_init
pub struct FixedBumpString<'a> {
    vec: FixedBumpVec<'a, u8>,
}

unsafe impl<'a> Send for FixedBumpString<'a> {}
unsafe impl<'a> Sync for FixedBumpString<'a> {}

impl<'a> FixedBumpString<'a> {
    /// Empty fixed string.
    pub const EMPTY: Self = Self {
        vec: FixedBumpVec::EMPTY,
    };

    /// Turns a `BumpBox<str>` into a full `FixedBumpString`.
    #[must_use]
    pub fn from_init(initialized: BumpBox<'a, str>) -> Self {
        Self {
            vec: FixedBumpVec::from_init(initialized.into_boxed_bytes()),
        }
    }

    /// Turns a `BumpBox<[MaybeUninit<u8>]>` into a `FixedBumpString` with a length of `0`.
    #[must_use]
    pub fn from_uninit(uninitialized: BumpBox<'a, [MaybeUninit<u8>]>) -> Self {
        Self {
            vec: FixedBumpVec::from_uninit(uninitialized),
        }
    }

    /// Returns this `FixedBumpString`'s capacity, in bytes.
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// Returns the length of this `FixedBumpString`, in bytes, not [`char`]s or
    /// graphemes. In other words, it might not be what a human considers the
    /// length of the string.
    #[must_use]
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns `true` if this `FixedBumpString` has a length of zero, and `false` otherwise.
    #[must_use]
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Truncates this `FixedBumpString`, removing all contents.
    ///
    /// While this means the `FixedBumpString` will have a length of zero, it does not
    /// touch its capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpString };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut s = bump.alloc_fixed_string(3);
    /// s.push_str("foo");
    ///
    /// s.clear();
    ///
    /// assert!(s.is_empty());
    /// assert_eq!(s.len(), 0);
    /// assert!(s.capacity() == 3);
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear();
    }

    /// Converts a bump allocated vector of bytes to a `FixedBumpString`.
    ///
    /// A string ([`FixedBumpString`]) is made of bytes ([`u8`]), and a vector of bytes
    /// ([`FixedBumpVec<u8>`]) is made of bytes, so this function converts between the
    /// two. Not all byte slices are valid `FixedBumpString`s, however: `FixedBumpString`
    /// requires that it is valid UTF-8. `from_utf8()` checks to ensure that
    /// the bytes are valid UTF-8, and then does the conversion.
    ///
    /// If you are sure that the byte slice is valid UTF-8, and you don't want
    /// to incur the overhead of the validity check, there is an unsafe version
    /// of this function, [`from_utf8_unchecked`], which has the same behavior
    /// but skips the check.
    ///
    /// This method will take care to not copy the vector, for efficiency's
    /// sake.
    ///
    /// If you need a [`&str`] instead of a `FixedBumpString`, consider
    /// [`str::from_utf8`].
    ///
    /// The inverse of this method is [`into_bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the slice is not UTF-8 with a description as to why the
    /// provided bytes are not UTF-8. The vector you moved in is also included.
    ///
    /// [`from_utf8_unchecked`]: String::from_utf8_unchecked
    /// [`FixedBumpVec<u8>`]: FixedBumpVec
    /// [`&str`]: prim@str "&str"
    /// [`into_bytes`]: FixedBumpString::into_bytes
    pub fn from_utf8(vec: FixedBumpVec<'a, u8>) -> Result<Self, FromUtf8Error<FixedBumpVec<'a, u8>>> {
        match str::from_utf8(vec.as_slice()) {
            Ok(_) => Ok(Self { vec }),
            Err(error) => Err(FromUtf8Error { error, bytes: vec }),
        }
    }

    /// Converts a `FixedBumpString` into a `FixedBumpVec<u8>`.
    ///
    /// This consumes the `FixedBumpString`, so we do not need to copy its contents.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_fixed_string(5);
    /// s.push_str("hello");
    /// let bytes = s.into_bytes();
    ///
    /// assert_eq!(&[104, 101, 108, 108, 111][..], &bytes[..]);
    /// ```
    #[inline(always)]
    #[must_use = "`self` will be dropped if the result is not used"]
    pub fn into_bytes(self) -> FixedBumpVec<'a, u8> {
        self.vec
    }

    /// Returns a byte slice of this `FixedBumpString`'s contents.
    #[must_use]
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.vec.as_slice()
    }

    /// Extracts a string slice containing the entire `FixedBumpString`.
    #[must_use]
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.vec.as_slice()) }
    }

    /// Converts a `FixedBumpString` into a mutable string slice.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.vec.as_mut_slice()) }
    }

    /// Returns a mutable reference to the contents of this `FixedBumpString`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned `&mut Vec` allows writing
    /// bytes which are not valid UTF-8. If this constraint is violated, using
    /// the original `FixedBumpString` after dropping the `&mut Vec` may violate memory
    /// safety, as `FixedBumpString`s must be valid UTF-8.
    #[must_use]
    #[inline(always)]
    pub unsafe fn as_mut_vec(&mut self) -> &FixedBumpVec<'a, u8> {
        &mut self.vec
    }

    /// Removes a [`char`] from this `String` at a byte position and returns it.
    ///
    /// This is an *O*(*n*) operation, as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than or equal to the `String`'s length,
    /// or if it does not lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpString };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut s = bump.alloc_fixed_string(4);
    /// s.push_str("abç");
    ///
    /// assert_eq!(s.remove(0), 'a');
    /// assert_eq!(s.remove(1), 'ç');
    /// assert_eq!(s.remove(0), 'b');
    /// ```
    #[inline]
    pub fn remove(&mut self, idx: usize) -> char {
        let ch = match self[idx..].chars().next() {
            Some(ch) => ch,
            None => panic!("cannot remove a char from the end of a string"),
        };

        let next = idx + ch.len_utf8();
        let len = self.len();
        unsafe {
            ptr::copy(self.vec.as_ptr().add(next), self.vec.as_mut_ptr().add(idx), len - next);
            self.vec.set_len(len - (next - idx));
        }
        ch
    }
}

impl<'a> FixedBumpString<'a> {
    error_behavior_generic_methods_if! {
        if "the string is full"

        /// Appends the given [`char`] to the end of this `FixedBumpString`.
        impl
        for pub fn push
        for pub fn try_push
        fn generic_push(&mut self, ch: char) {
            match ch.len_utf8() {
                1 => self.vec.generic_push(ch as u8),
                _ => self.vec.generic_extend_from_slice_copy(ch.encode_utf8(&mut [0; 4]).as_bytes()),
            }
        }

        /// Appends a given string slice onto the end of this `FixedBumpString`.
        impl
        for pub fn push_str
        for pub fn try_push_str
        fn generic_push_str(&mut self, string: &str) {
            self.vec.generic_extend_from_slice_copy(string.as_bytes())
        }

        /// Inserts a character into this `FixedBumpString` at a byte position.
        ///
        /// This is an *O*(*n*) operation as it requires copying every element in the
        /// buffer.
        do panics
        /// Panics if `idx` is larger than the `String`'s length, or if it does not
        /// lie on a [`char`] boundary.
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::new();
        /// #
        /// let mut s = bump.alloc_fixed_string(3);
        ///
        /// s.insert(0, 'f');
        /// s.insert(1, 'o');
        /// s.insert(2, 'o');
        ///
        /// assert_eq!("foo", s);
        /// ```
        impl
        for pub fn insert
        for pub fn try_insert
        fn generic_insert(&mut self, idx: usize, ch: char) {
            assert!(self.is_char_boundary(idx));
            let mut bits = [0; 4];
            let bits = ch.encode_utf8(&mut bits).as_bytes();

            unsafe {
                self.insert_bytes(idx, bits)
            }
        }

        /// Inserts a string slice into this `FixedBumpString` at a byte position.
        ///
        /// This is an *O*(*n*) operation as it requires copying every element in the
        /// buffer.
        do panics
        /// Panics if `idx` is larger than the `FixedBumpString`'s length, or if it does not
        /// lie on a [`char`] boundary.
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::new();
        /// #
        /// let mut s = bump.alloc_fixed_string(6);
        /// s.push_str("bar");
        ///
        /// s.insert_str(0, "foo");
        ///
        /// assert_eq!("foobar", s);
        /// ```
        impl
        for pub fn insert_str
        for pub fn try_insert_str
        fn generic_insert_str(&mut self, idx: usize, string: &str) {
            assert!(self.is_char_boundary(idx));

            unsafe {
                self.insert_bytes(idx, string.as_bytes())
            }
        }

        /// Copies elements from `src` range to the end of the string.
        do panics
        /// Panics if the starting point or end point do not lie on a [`char`]
        /// boundary, or if they're out of bounds.
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::new();
        /// #
        /// let mut string = bump.alloc_fixed_string(14);
        /// string.push_str("abcde");
        ///
        /// string.extend_from_within(2..);
        /// assert_eq!(string, "abcdecde");
        ///
        /// string.extend_from_within(..2);
        /// assert_eq!(string, "abcdecdeab");
        ///
        /// string.extend_from_within(4..8);
        /// assert_eq!(string, "abcdecdeabecde");
        /// ```
        impl
        for pub fn extend_from_within
        for pub fn try_extend_from_within
        fn generic_extend_from_within<{R}>(&mut self, src: R)
        where {
            R: RangeBounds<usize>,
        } in {
            let src @ Range { start, end } = polyfill::slice::range(src, ..self.len());

            assert!(self.is_char_boundary(start));
            assert!(self.is_char_boundary(end));

            self.vec.generic_extend_from_within_copy(src)
        }

        /// Extends this string by pushing `additional` new zero bytes.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::new();
        /// let mut string = bump.alloc_fixed_string(8);
        /// string.push_str("What?");
        /// string.extend_zeroed(3);
        /// assert_eq!(string, "What?\0\0\0");
        /// ```
        for pub fn extend_zeroed
        for pub fn try_extend_zeroed
        fn generic_extend_zeroed(&mut self, additional: usize) {
            self.vec.generic_reserve(additional)?;

            unsafe {
                let ptr = self.vec.as_mut_ptr();
                let len = self.len();

                ptr.add(len).write_bytes(0, additional);
                self.vec.set_len(len + additional);
            }

            Ok(())
        }
    }

    unsafe fn insert_bytes<B: ErrorBehavior>(&mut self, idx: usize, bytes: &[u8]) -> Result<(), B> {
        let len = self.len();
        let amt = bytes.len();
        self.vec.generic_reserve(amt)?;

        ptr::copy(self.vec.as_ptr().add(idx), self.vec.as_mut_ptr().add(idx + amt), len - idx);
        ptr::copy_nonoverlapping(bytes.as_ptr(), self.vec.as_mut_ptr().add(idx), amt);
        self.vec.set_len(len + amt);

        Ok(())
    }

    /// Converts a `FixedBumpString` into a `BumpBox<str>`.
    ///
    /// You may want to call `shrink_to_fit` before this, so the unused capacity does not take up space.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_str(self) -> BumpBox<'a, str> {
        unsafe { self.vec.into_boxed_slice().into_boxed_str_unchecked() }
    }

    /// Converts this `BumpBox<str>` into `&str` that is live for the entire bump scope.
    ///
    /// You may want to call `shrink_to_fit` before this, so the unused capacity does not take up space.
    #[must_use]
    #[inline(always)]
    pub fn into_str(self) -> &'a mut str {
        self.into_boxed_str().into_mut()
    }
}

impl<'a> fmt::Write for FixedBumpString<'a> {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.try_push_str(s).map_err(|_| fmt::Error)
    }

    #[inline(always)]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.try_push(c).map_err(|_| fmt::Error)
    }
}

impl<'a> Debug for FixedBumpString<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl<'a> Display for FixedBumpString<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl<'a> Deref for FixedBumpString<'a> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<'a> DerefMut for FixedBumpString<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'a> core::ops::AddAssign<&str> for FixedBumpString<'a> {
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}

impl<'a> AsRef<str> for FixedBumpString<'a> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> AsMut<str> for FixedBumpString<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<'a> Borrow<str> for FixedBumpString<'a> {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<'a> BorrowMut<str> for FixedBumpString<'a> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<'a> PartialEq for FixedBumpString<'a> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        <str as PartialEq>::eq(self, other)
    }

    #[inline]
    fn ne(&self, other: &Self) -> bool {
        <str as PartialEq>::ne(self, other)
    }
}

macro_rules! impl_partial_eq {
    (
        $(
            $(#[$attr:meta])*
            $string_like:ty,
        )*
    ) => {
        $(
            $(#[$attr])*
            impl<'a> PartialEq<$string_like> for FixedBumpString<'a> {
                #[inline]
                fn eq(&self, other: &$string_like) -> bool {
                    <str as PartialEq>::eq(self, other)
                }

                #[inline]
                fn ne(&self, other: &$string_like) -> bool {
                    <str as PartialEq>::ne(self, other)
                }
            }

            $(#[$attr])*
            impl<'a> PartialEq<FixedBumpString<'a>> for $string_like {
                #[inline]
                fn eq(&self, other: &FixedBumpString<'a>) -> bool {
                    <str as PartialEq>::eq(self, other)
                }

                #[inline]
                fn ne(&self, other: &FixedBumpString<'a>) -> bool {
                    <str as PartialEq>::ne(self, other)
                }
            }
        )*
    };
}

impl_partial_eq! {
    str,

    &str,

    #[cfg(feature = "alloc")]
    alloc::string::String,

    #[cfg(feature = "alloc")]
    alloc::borrow::Cow<'_, str>,
}

impl<'a> Eq for FixedBumpString<'a> {}

impl<'a> PartialOrd for FixedBumpString<'a> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }

    #[inline]
    fn lt(&self, other: &Self) -> bool {
        <str as PartialOrd>::lt(self, other)
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        <str as PartialOrd>::le(self, other)
    }

    #[inline]
    fn gt(&self, other: &Self) -> bool {
        <str as PartialOrd>::gt(self, other)
    }

    #[inline]
    fn ge(&self, other: &Self) -> bool {
        <str as PartialOrd>::ge(self, other)
    }
}

impl<'a> Ord for FixedBumpString<'a> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        <str as Ord>::cmp(self, other)
    }
}

impl<'a> Hash for FixedBumpString<'a> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.vec.hash(state);
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'s, 'a> Extend<&'s str> for FixedBumpString<'a> {
    #[inline]
    fn extend<T: IntoIterator<Item = &'s str>>(&mut self, iter: T) {
        for str in iter {
            self.push_str(str);
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> From<FixedBumpString<'a>> for alloc::string::String {
    #[inline]
    fn from(value: FixedBumpString<'a>) -> Self {
        value.as_str().into()
    }
}
