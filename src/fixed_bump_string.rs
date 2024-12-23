use crate::{
    error_behavior_generic_methods_if, one_sided_range, owned_str,
    polyfill::{self, nonnull, transmute_mut},
    BumpAllocatorScope, BumpBox, BumpString, ErrorBehavior, FixedBumpVec, FromUtf8Error, NoDrop, OneSidedRange,
};
use core::{
    borrow::{Borrow, BorrowMut},
    fmt::{self, Debug, Display},
    hash::Hash,
    mem::{self, MaybeUninit},
    ops::{Deref, DerefMut, Range, RangeBounds},
    ptr, str,
};

/// A type like [`BumpString`](crate::BumpString) but with a fixed capacity.
///
/// It can be constructed with [`alloc_fixed_string`] or from a `BumpBox` via [`from_init`] or [`from_uninit`].
///
/// # Examples
/// ```
/// # use bump_scope::Bump;
/// # let mut bump: Bump = Bump::new();
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
/// [`from_uninit`]: Self::from_uninit
/// [`from_init`]: Self::from_init
// `FixedBumpString` and `FixedBumpVec<u8>` have the same repr.
#[repr(C)]
pub struct FixedBumpString<'a> {
    pub(crate) initialized: BumpBox<'a, str>,
    pub(crate) capacity: usize,
}

unsafe impl Send for FixedBumpString<'_> {}
unsafe impl Sync for FixedBumpString<'_> {}

impl<'a> FixedBumpString<'a> {
    /// Empty fixed string.
    pub const EMPTY: Self = Self {
        initialized: BumpBox::EMPTY_STR,
        capacity: 0,
    };

    /// Turns a `BumpBox<str>` into a full `FixedBumpString`.
    #[must_use]
    pub fn from_init(initialized: BumpBox<'a, str>) -> Self {
        let capacity = initialized.len();
        Self { initialized, capacity }
    }

    /// Turns a `BumpBox<[MaybeUninit<u8>]>` into a `FixedBumpString` with a length of `0`.
    #[must_use]
    pub fn from_uninit(uninitialized: BumpBox<'a, [MaybeUninit<u8>]>) -> Self {
        let uninitialized = uninitialized.into_raw();
        let capacity = uninitialized.len();

        let ptr = nonnull::as_non_null_ptr(uninitialized).cast::<u8>();
        let initialized = unsafe { BumpBox::from_raw(nonnull::str_from_utf8(nonnull::slice_from_raw_parts(ptr, 0))) };

        Self { initialized, capacity }
    }

    /// Converts a vector of bytes to a `FixedBumpString`.
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
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpString };
    /// # let bump: Bump = Bump::new();
    /// // some bytes, in a vector
    /// let mut sparkle_heart = bump.alloc_fixed_vec(4);
    /// sparkle_heart.extend_from_slice_copy(&[240, 159, 146, 150]);
    ///
    /// // We know these bytes are valid, so we'll use `unwrap()`.
    /// let sparkle_heart = FixedBumpString::from_utf8(sparkle_heart).unwrap();
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    ///
    /// Incorrect bytes:
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpString };
    /// # let bump: Bump = Bump::new();
    /// // some invalid bytes, in a vector
    /// let mut sparkle_heart = bump.alloc_fixed_vec(4);
    /// sparkle_heart.extend_from_slice_copy(&[0, 159, 146, 150]);
    ///
    /// assert!(FixedBumpString::from_utf8(sparkle_heart).is_err());
    /// ```
    ///
    /// [`from_utf8_unchecked`]: Self::from_utf8_unchecked
    /// [`FixedBumpVec<u8>`]: FixedBumpVec
    /// [`&str`]: prim@str "&str"
    /// [`into_bytes`]: Self::into_bytes
    pub fn from_utf8(vec: FixedBumpVec<'a, u8>) -> Result<Self, FromUtf8Error<FixedBumpVec<'a, u8>>> {
        match core::str::from_utf8(vec.as_slice()) {
            // SAFETY: `FixedBumpVec<u8>` and `FixedBumpString` have the same representation;
            // only the invariant that the bytes are utf8 is different.
            Ok(_) => Ok(unsafe { mem::transmute(vec) }),
            Err(error) => Err(FromUtf8Error { error, bytes: vec }),
        }
    }

    /// Converts a vector of bytes to a `FixedBumpString` without checking that the
    /// string contains valid UTF-8.
    ///
    /// See the safe version, [`from_utf8`](Self::from_utf8), for more details.
    ///
    /// # Safety
    ///
    /// The bytes passed in must be valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpString };
    /// # let bump: Bump = Bump::new();
    ///
    /// // some bytes, in a vector
    /// let mut sparkle_heart = bump.alloc_fixed_vec(4);
    /// sparkle_heart.extend_from_slice_copy(&[240, 159, 146, 150]);
    ///
    /// let sparkle_heart = unsafe {
    ///     FixedBumpString::from_utf8_unchecked(sparkle_heart)
    /// };
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    #[must_use]
    pub unsafe fn from_utf8_unchecked(vec: FixedBumpVec<'a, u8>) -> Self {
        debug_assert!(str::from_utf8(vec.as_slice()).is_ok());
        // SAFETY: `FixedBumpVec<u8>` and `FixedBumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        mem::transmute(vec)
    }

    /// Returns this string's capacity, in bytes.
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the length of this string, in bytes, not [`char`]s or
    /// graphemes. In other words, it might not be what a human considers the
    /// length of the string.
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.initialized.len()
    }

    /// Returns `true` if this string has a length of zero, and `false` otherwise.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.initialized.is_empty()
    }

    /// Converts this `FixedBumpString` into `&str` that is live for this bump scope.
    #[must_use]
    #[inline(always)]
    pub fn into_str(self) -> &'a mut str {
        self.initialized.into_mut()
    }

    /// Converts a `FixedBumpString` into a `BumpBox<str>`.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_str(self) -> BumpBox<'a, str> {
        self.initialized
    }

    /// Turns this `FixedBumpString<T>` into a `BumpVec<T>`.
    #[must_use]
    #[inline(always)]
    pub fn into_string<A: BumpAllocatorScope<'a>>(self, allocator: A) -> BumpString<A> {
        BumpString::from_parts(self, allocator)
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
        // SAFETY: `FixedBumpVec<u8>` and `FixedBumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        unsafe { mem::transmute(self) }
    }

    /// Splits the string into two at the given byte index.
    ///
    /// Returns a string containing the bytes in the provided range.
    /// After the call, the original string will be left containing the remaining bytes.
    /// The splitting point must be on the boundary of a UTF-8 code point.
    ///
    /// The string on the right will have the excess capacity if any.
    ///
    /// [String]: alloc::string::String
    /// [split_off]: alloc::string::String::split_off
    ///
    /// # Panics
    ///
    /// Panics if `at` is not on a `UTF-8` code point boundary, or if it is beyond the last
    /// code point of the string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut string = BumpString::with_capacity_in(10, &bump);
    /// string.push_str("foobarbaz");
    ///
    /// let foo = string.split_off(..3);
    /// assert_eq!(foo, "foo");
    /// assert_eq!(string, "barbaz");
    ///
    /// assert_eq!(foo.capacity(), 3);
    /// assert_eq!(string.capacity(), 7);
    ///
    /// let baz = string.split_off(3..);
    /// assert_eq!(baz, "baz");
    /// assert_eq!(string, "bar");
    ///
    /// assert_eq!(baz.capacity(), 4);
    /// assert_eq!(string.capacity(), 3);
    /// ```
    #[inline]
    #[allow(clippy::return_self_not_must_use)]
    pub fn split_off(&mut self, range: impl OneSidedRange<usize>) -> Self {
        let (direction, mid) = one_sided_range::direction(range, ..self.len());

        assert!(self.is_char_boundary(mid));

        let ptr = self.initialized.ptr.cast::<u8>();
        let len = self.len();
        let cap = self.capacity;

        let lhs = nonnull::slice_from_raw_parts(ptr, mid);
        let rhs = nonnull::slice_from_raw_parts(unsafe { nonnull::add(ptr, mid) }, len - mid);

        let lhs = nonnull::str_from_utf8(lhs);
        let rhs = nonnull::str_from_utf8(rhs);

        let lhs_capacity = mid;
        let rhs_capacity = cap - mid;

        match direction {
            one_sided_range::Direction::From => unsafe {
                self.initialized.ptr = lhs;
                self.capacity = lhs_capacity;

                FixedBumpString {
                    initialized: BumpBox::from_raw(rhs),
                    capacity: rhs_capacity,
                }
            },
            one_sided_range::Direction::To => unsafe {
                self.initialized.ptr = rhs;
                self.capacity = rhs_capacity;

                FixedBumpString {
                    initialized: BumpBox::from_raw(lhs),
                    capacity: lhs_capacity,
                }
            },
        }
    }

    /// Removes the last character from the string buffer and returns it.
    ///
    /// Returns [`None`] if this string is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_fixed_string(4);
    /// s.push_str("abč");
    ///
    /// assert_eq!(s.pop(), Some('č'));
    /// assert_eq!(s.pop(), Some('b'));
    /// assert_eq!(s.pop(), Some('a'));
    ///
    /// assert_eq!(s.pop(), None);
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<char> {
        self.initialized.pop()
    }

    /// Truncates this string, removing all contents.
    ///
    /// While this means the string will have a length of zero, it does not
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
        self.initialized.clear();
    }

    /// Shortens this string to the specified length.
    ///
    /// If `new_len` is greater than or equal to the string's current length, this has no
    /// effect.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the string
    ///
    /// # Panics
    ///
    /// Panics if `new_len` does not lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_fixed_string(5);
    /// s.push_str("hello");
    ///
    /// s.truncate(2);
    ///
    /// assert_eq!(s, "he");
    /// ```
    #[inline]
    pub fn truncate(&mut self, new_len: usize) {
        self.initialized.truncate(new_len);
    }

    /// Removes a [`char`] from this string at a byte position and returns it.
    ///
    /// This is an *O*(*n*) operation, as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than or equal to the string's length,
    /// or if it does not lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_fixed_string(4);
    /// s.push_str("abç");
    ///
    /// assert_eq!(s.remove(0), 'a');
    /// assert_eq!(s.remove(1), 'ç');
    /// assert_eq!(s.remove(0), 'b');
    /// ```
    #[inline]
    pub fn remove(&mut self, idx: usize) -> char {
        self.initialized.remove(idx)
    }

    /// Retains only the characters specified by the predicate.
    ///
    /// In other words, remove all characters `c` such that `f(c)` returns `false`.
    /// This method operates in place, visiting each character exactly once in the
    /// original order, and preserves the order of the retained characters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_fixed_string(50);
    /// s.push_str("f_o_ob_ar");
    ///
    /// s.retain(|c| c != '_');
    ///
    /// assert_eq!(s, "foobar");
    /// ```
    ///
    /// Because the elements are visited exactly once in the original order,
    /// external state may be used to decide which elements to keep.
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_fixed_string(50);
    /// s.push_str("abcde");
    ///
    /// let keep = [false, true, true, false, true];
    /// let mut iter = keep.iter();
    /// s.retain(|_| *iter.next().unwrap());
    /// assert_eq!(s, "bce");
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(char) -> bool,
    {
        self.initialized.retain(f);
    }

    /// Removes the specified range from the string in bulk, returning all
    /// removed characters as an iterator.
    ///
    /// The returned iterator keeps a mutable borrow on the string to optimize
    /// its implementation.
    ///
    /// # Panics
    ///
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Leaking
    ///
    /// If the returned iterator goes out of scope without being dropped (due to
    /// [`core::mem::forget`], for example), the string may still contain a copy
    /// of any drained characters, or may have lost characters arbitrarily,
    /// including characters outside the range.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_fixed_string(50);
    /// s.push_str("α is alpha, β is beta");
    ///
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Remove the range up until the β from the string
    /// let t: String = s.drain(..beta_offset).collect();
    /// assert_eq!(t, "α is alpha, ");
    /// assert_eq!(s, "β is beta");
    ///
    /// // A full range clears the string, like `clear()` does
    /// s.drain(..);
    /// assert_eq!(s, "");
    /// ```
    pub fn drain<R>(&mut self, range: R) -> owned_str::Drain<'_>
    where
        R: RangeBounds<usize>,
    {
        self.initialized.drain(range)
    }

    /// Extracts a string slice containing the entire `FixedBumpString`.
    #[must_use]
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.initialized
    }

    /// Converts a `FixedBumpString` into a mutable string slice.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_str(&mut self) -> &mut str {
        &mut self.initialized
    }

    /// Returns a byte slice of this `FixedBumpString`'s contents.
    #[must_use]
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.initialized.as_bytes()
    }

    /// Returns a mutable reference to the contents of this `FixedBumpString`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned `&mut FixedBumpVec<u8>` allows writing
    /// bytes which are not valid UTF-8. If this constraint is violated, using
    /// the original `FixedBumpString` after dropping the `&mut FixedBumpVec<u8>` may violate memory
    /// safety, as `FixedBumpString`s must be valid UTF-8.
    #[must_use]
    #[inline(always)]
    pub unsafe fn as_mut_vec(&mut self) -> &mut FixedBumpVec<'a, u8> {
        // SAFETY: `BumpVec<u8>` and `BumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        transmute_mut(self)
    }
}

impl FixedBumpString<'_> {
    error_behavior_generic_methods_if! {
        if "the string is full"

        /// Appends the given [`char`] to the end of this string.
        impl
        do examples
        /// ```
        /// # use bump_scope::Bump;
        /// # let bump: Bump = Bump::new();
        /// let mut s = bump.alloc_fixed_string(3);
        ///
        /// s.push('a');
        /// s.push('b');
        /// s.push('c');
        ///
        /// assert_eq!(s, "abc");
        /// ```
        for fn push
        do examples
        /// ```
        /// # use bump_scope::Bump;
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut s = bump.try_alloc_fixed_string(3)?;
        ///
        /// s.try_push('a')?;
        /// s.try_push('b')?;
        /// s.try_push('c')?;
        ///
        /// assert_eq!(s, "abc");
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_push
        #[inline]
        use fn generic_push(&mut self, ch: char) {
            let vec = unsafe { self.as_mut_vec() };

            match ch.len_utf8() {
                1 => vec.generic_push(ch as u8),
                _ => vec.generic_extend_from_slice_copy(ch.encode_utf8(&mut [0; 4]).as_bytes()),
            }
        }

        /// Appends a given string slice onto the end of this string.
        impl
        do examples
        /// ```
        /// # use bump_scope::Bump;
        /// # let bump: Bump = Bump::new();
        /// let mut s = bump.alloc_fixed_string(6);
        ///
        /// s.push_str("foo");
        /// s.push_str("bar");
        ///
        /// assert_eq!(s, "foobar");
        /// ```
        for fn push_str
        do examples
        /// ```
        /// # use bump_scope::Bump;
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut s = bump.try_alloc_fixed_string(6)?;
        ///
        /// s.try_push_str("foo")?;
        /// s.try_push_str("bar")?;
        ///
        /// assert_eq!(s, "foobar");
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_push_str
        #[inline]
        use fn generic_push_str(&mut self, string: &str) {
            let vec = unsafe { self.as_mut_vec() };
            vec.generic_extend_from_slice_copy(string.as_bytes())
        }

        /// Inserts a character into this string at a byte position.
        ///
        /// This is an *O*(*n*) operation as it requires copying every element in the
        /// buffer.
        do panics
        /// Panics if `idx` is larger than the string's length, or if it does not
        /// lie on a [`char`] boundary.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::new();
        /// let mut s = bump.alloc_fixed_string(3);
        ///
        /// s.insert(0, 'f');
        /// s.insert(1, 'o');
        /// s.insert(2, 'o');
        ///
        /// assert_eq!("foo", s);
        /// ```
        for fn insert
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut s = bump.try_alloc_fixed_string(3)?;
        ///
        /// s.try_insert(0, 'f')?;
        /// s.try_insert(1, 'o')?;
        /// s.try_insert(2, 'o')?;
        ///
        /// assert_eq!("foo", s);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_insert
        #[inline]
        use fn generic_insert(&mut self, idx: usize, ch: char) {
            assert!(self.is_char_boundary(idx));
            let mut bits = [0; 4];
            let bits = ch.encode_utf8(&mut bits).as_bytes();

            unsafe {
                self.insert_bytes(idx, bits)
            }
        }

        /// Inserts a string slice into this string at a byte position.
        ///
        /// This is an *O*(*n*) operation as it requires copying every element in the
        /// buffer.
        do panics
        /// Panics if `idx` is larger than the string's length, or if it does not
        /// lie on a [`char`] boundary.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::new();
        /// let mut s = bump.alloc_fixed_string(6);
        /// s.push_str("bar");
        ///
        /// s.insert_str(0, "foo");
        ///
        /// assert_eq!("foobar", s);
        /// ```
        for fn insert_str
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut s = bump.try_alloc_fixed_string(6)?;
        /// s.try_push_str("bar")?;
        ///
        /// s.try_insert_str(0, "foo")?;
        ///
        /// assert_eq!("foobar", s);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_insert_str
        #[inline]
        use fn generic_insert_str(&mut self, idx: usize, string: &str) {
            assert!(self.is_char_boundary(idx));

            unsafe {
                self.insert_bytes(idx, string.as_bytes())
            }
        }

        /// Copies elements from `src` range to the end of the string.
        do panics
        /// Panics if the starting point or end point do not lie on a [`char`]
        /// boundary, or if they're out of bounds.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::new();
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
        for fn extend_from_within
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut string = bump.try_alloc_fixed_string(14)?;
        /// string.try_push_str("abcde")?;
        ///
        /// string.try_extend_from_within(2..)?;
        /// assert_eq!(string, "abcdecde");
        ///
        /// string.try_extend_from_within(..2)?;
        /// assert_eq!(string, "abcdecdeab");
        ///
        /// string.try_extend_from_within(4..8)?;
        /// assert_eq!(string, "abcdecdeabecde");
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_extend_from_within
        #[inline]
        use fn generic_extend_from_within<{R}>(&mut self, src: R)
        where {
            R: RangeBounds<usize>,
        } in {
            let src @ Range { start, end } = polyfill::slice::range(src, ..self.len());

            assert!(self.is_char_boundary(start));
            assert!(self.is_char_boundary(end));

            let vec = unsafe { self.as_mut_vec() };
            vec.generic_extend_from_within_copy(src)
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
        for fn extend_zeroed
        do examples
        /// ```
        /// # use bump_scope::{ Bump, FixedBumpString };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut string = bump.try_alloc_fixed_string(8)?;
        /// string.try_push_str("What?")?;
        /// string.try_extend_zeroed(3)?;
        /// assert_eq!(string, "What?\0\0\0");
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_extend_zeroed
        #[inline]
        use fn generic_extend_zeroed(&mut self, additional: usize) {
            let vec = unsafe { self.as_mut_vec() };
            vec.generic_reserve(additional)?;

            unsafe {
                let ptr = vec.as_mut_ptr();
                let len = vec.len();

                ptr.add(len).write_bytes(0, additional);
                vec.set_len(len + additional);
            }

            Ok(())
        }

        /// Checks if at least `additional` more bytes can be inserted
        /// in the given `FixedBumpString` due to capacity.
        impl
        for fn reserve
        for fn try_reserve
        #[inline]
        use fn generic_reserve(&mut self, additional: usize) {
            unsafe { self.as_mut_vec() }.generic_reserve(additional)
        }
    }

    unsafe fn insert_bytes<B: ErrorBehavior>(&mut self, idx: usize, bytes: &[u8]) -> Result<(), B> {
        let vec = self.as_mut_vec();

        let len = vec.len();
        let amt = bytes.len();
        vec.generic_reserve(amt)?;

        ptr::copy(vec.as_ptr().add(idx), vec.as_mut_ptr().add(idx + amt), len - idx);
        ptr::copy_nonoverlapping(bytes.as_ptr(), vec.as_mut_ptr().add(idx), amt);
        vec.set_len(len + amt);

        Ok(())
    }
}

impl fmt::Write for FixedBumpString<'_> {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.try_push_str(s).map_err(|_| fmt::Error)
    }

    #[inline(always)]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.try_push(c).map_err(|_| fmt::Error)
    }
}

impl Debug for FixedBumpString<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Display for FixedBumpString<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Default for FixedBumpString<'_> {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Deref for FixedBumpString<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for FixedBumpString<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

#[cfg(feature = "panic-on-alloc")]
impl core::ops::AddAssign<&str> for FixedBumpString<'_> {
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}

impl AsRef<str> for FixedBumpString<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsMut<str> for FixedBumpString<'_> {
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl Borrow<str> for FixedBumpString<'_> {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl BorrowMut<str> for FixedBumpString<'_> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl PartialEq for FixedBumpString<'_> {
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

impl Eq for FixedBumpString<'_> {}

impl PartialOrd for FixedBumpString<'_> {
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

impl Ord for FixedBumpString<'_> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        <str as Ord>::cmp(self, other)
    }
}

impl Hash for FixedBumpString<'_> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<'s> Extend<&'s str> for FixedBumpString<'_> {
    #[inline]
    fn extend<T: IntoIterator<Item = &'s str>>(&mut self, iter: T) {
        for str in iter {
            self.push_str(str);
        }
    }
}

#[cfg(feature = "panic-on-alloc")]
impl Extend<char> for FixedBumpString<'_> {
    fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
        let iterator = iter.into_iter();
        let (lower_bound, _) = iterator.size_hint();
        self.reserve(lower_bound);
        iterator.for_each(move |c| self.push(c));
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<'s> Extend<&'s char> for FixedBumpString<'_> {
    fn extend<I: IntoIterator<Item = &'s char>>(&mut self, iter: I) {
        self.extend(iter.into_iter().copied());
    }
}

#[cfg(feature = "alloc")]
impl<'a> From<FixedBumpString<'a>> for alloc::string::String {
    #[inline]
    fn from(value: FixedBumpString<'a>) -> Self {
        value.as_str().into()
    }
}

impl NoDrop for FixedBumpString<'_> {}
