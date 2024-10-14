use crate::{
    error_behavior_generic_methods_allocation_failure,
    polyfill::{self, nonnull, transmute_mut},
    BaseAllocator, BumpBox, BumpScope, ErrorBehavior, FixedBumpString, FromUtf8Error, GuaranteedAllocatedStats,
    MinimumAlignment, MutBumpVec, Stats, SupportedMinimumAlignment,
};
use core::{
    borrow::{Borrow, BorrowMut},
    fmt::{self, Debug, Display},
    hash::Hash,
    mem,
    ops::{Deref, DerefMut, Range, RangeBounds},
    ptr, str,
};

#[cfg(not(no_global_oom_handling))]
use crate::Infallibly;

/// This is like [`format!`] but allocates inside a *mutable* `Bump` or `BumpScope`, returning a [`MutBumpString`].
///
/// If you don't need to push to the string after creation you can also use [`Bump::alloc_fmt_mut`](crate::Bump::alloc_fmt_mut).
///
/// # Panics
/// If used without `try`, panics on allocation failure or if a formatting trait implementation returns an error.
///
/// # Errors
/// If used with `try`, errors on allocation failure or if a formatting trait implementation returns an error.
///
/// # Examples
///
/// ```
/// # use bump_scope::{ Bump, mut_bump_format };
/// # let mut bump: Bump = Bump::new();
/// #
/// let greeting = "Hello";
/// let mut string = mut_bump_format!(in bump, "{greeting} world!");
/// string.push_str(" How are you?");
///
/// assert_eq!(string, "Hello world! How are you?");
/// ```
#[macro_export]
macro_rules! mut_bump_format {
    (in $bump:expr) => {{
        $crate::MutBumpString::new_in($bump.as_mut_scope())
    }};
    (in $bump:expr, $($arg:tt)*) => {{
        let mut string = $crate::private::Infallibly($crate::MutBumpString::new_in($bump.as_mut_scope()));
        match $crate::private::core::fmt::Write::write_fmt(&mut string, $crate::private::core::format_args!($($arg)*)) {
            $crate::private::core::result::Result::Ok(_) => string.0,
            $crate::private::core::result::Result::Err(_) => $crate::private::format_trait_error(),
        }
    }};
    (try in $bump:expr) => {{
        Ok::<_, $crate::allocator_api2::alloc::AllocError>($crate::MutBumpString::new_in($bump.as_mut_scope()))
    }};
    (try in $bump:expr, $($arg:tt)*) => {{
        let mut string = $crate::MutBumpString::new_in($bump.as_mut_scope());
        match $crate::private::core::fmt::Write::write_fmt(&mut string, $crate::private::core::format_args!($($arg)*)) {
            $crate::private::core::result::Result::Ok(_) => $crate::private::core::result::Result::Ok(string),
            $crate::private::core::result::Result::Err(_) => $crate::private::core::result::Result::Err($crate::allocator_api2::alloc::AllocError),
        }
    }};
}

macro_rules! mut_bump_string_declaration {
    ($($allocator_parameter:tt)*) => {
        /// A type like [`BumpString`](crate::BumpString), optimized for a `&mut Bump(Scope)`.
        ///
        /// It has the advantage that it can assume the entire remaining chunk space as its capacity.
        /// It also only needs to update the bump pointer when calling <code>[into_](Self::into_str)([boxed_](Self::into_boxed_str))[str](Self::into_str)</code>.
        ///
        /// When you are done building the string, you can turn it into a `&str` with [`into_str`].
        ///
        /// # Examples
        ///
        /// You can create a `MutBumpString` from [a literal string][`&str`] with [`MutBumpString::from_str_in`]:
        ///
        /// [`into_str`]: MutBumpString::into_str
        ///
        /// ```
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::new();
        /// let hello = MutBumpString::from_str_in("Hello, world!", &mut bump);
        /// # let _ = hello;
        /// ```
        ///
        /// You can append a [`char`] to a `String` with the [`push`] method, and
        /// append a [`&str`] with the [`push_str`] method:
        ///
        /// ```
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::new();
        /// let mut hello = MutBumpString::from_str_in("Hello, ", &mut bump);
        ///
        /// hello.push('w');
        /// hello.push_str("orld!");
        ///
        /// assert_eq!(hello.as_str(), "Hello, world!");
        /// ```
        ///
        /// [`push`]: MutBumpString::push
        /// [`push_str`]: MutBumpString::push_str
        ///
        /// If you have a vector of UTF-8 bytes, you can create a `MutBumpString` from it with
        /// the [`from_utf8`] method:
        ///
        /// ```
        /// # use bump_scope::{ Bump, MutBumpString, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// // some bytes, in a vector
        /// let sparkle_heart = mut_bump_vec![in bump; 240, 159, 146, 150];
        ///
        /// // We know these bytes are valid, so we'll use `unwrap()`.
        /// let sparkle_heart = MutBumpString::from_utf8(sparkle_heart).unwrap();
        ///
        /// assert_eq!("💖", sparkle_heart);
        /// ```
        ///
        /// [`&str`]: prim@str "&str"
        /// [`from_utf8`]: MutBumpString::from_utf8
        #[repr(C)]
        pub struct MutBumpString<
            'b,
            'a: 'b,
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        > {
            fixed: FixedBumpString<'b>,
            pub(crate) bump: &'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
        }
    };
}

crate::maybe_default_allocator!(mut_bump_string_declaration);

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Constructs a new empty `MutBumpString`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    pub fn new_in(bump: impl Into<&'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
        Self {
            fixed: FixedBumpString::EMPTY,
            bump: bump.into(),
        }
    }

    error_behavior_generic_methods_allocation_failure! {
        /// Constructs a new empty `MutBumpString` with at least the specified capacity
        /// with the provided `BumpScope`.
        ///
        /// The string will be able to hold at least `capacity` bytes without
        /// reallocating. This method allocates for as much elements as the< current chunk can hold.
        /// If `capacity` is 0, the string will not allocate.
        impl
        for fn with_capacity_in
        for fn try_with_capacity_in
        use fn generic_with_capacity_in(capacity: usize, bump: impl Into<&'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            let bump = bump.into();

            if capacity == 0 {
                return Ok(Self {
                    fixed: FixedBumpString::EMPTY,
                    bump,
                });
            }

            let (ptr, capacity) = bump.alloc_greedy(capacity)?;

            Ok(Self {
                fixed: FixedBumpString {
                    initialized: unsafe{ BumpBox::from_raw(nonnull::str_from_utf8(nonnull::slice_from_raw_parts(ptr, 0))) },
                    capacity,
                },
                bump,
            })
        }

        /// Constructs a new `MutBumpString` from a `&str`.
        impl
        for fn from_str_in
        for fn try_from_str_in
        use fn generic_from_str_in(string: &str, bump: impl Into<&'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            let mut this = Self::new_in(bump);
            this.generic_push_str(string)?;
            Ok(this)
        }
    }
}

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    /// Returns this `MutBumpString`'s capacity, in bytes.
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.fixed.capacity()
    }

    /// Returns the length of this `MutBumpString`, in bytes, not [`char`]s or
    /// graphemes. In other words, it might not be what a human considers the
    /// length of the string.
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.fixed.len()
    }

    /// Returns `true` if this `MutBumpString` has a length of zero, and `false` otherwise.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.fixed.is_empty()
    }

    /// Removes the last character from the string buffer and returns it.
    ///
    /// Returns [`None`] if this string is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, MutBumpString };
    /// # let mut bump: Bump = Bump::new();
    /// let mut s = MutBumpString::from_str_in("abč", &mut bump);
    ///
    /// assert_eq!(s.pop(), Some('č'));
    /// assert_eq!(s.pop(), Some('b'));
    /// assert_eq!(s.pop(), Some('a'));
    ///
    /// assert_eq!(s.pop(), None);
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<char> {
        self.fixed.pop()
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
    /// # use bump_scope::{ Bump, MutBumpString };
    /// # let mut bump: Bump = Bump::new();
    /// let mut s = MutBumpString::from_str_in("hello", &mut bump);
    ///
    /// s.truncate(2);
    ///
    /// assert_eq!("he", s);
    /// ```
    #[inline]
    pub fn truncate(&mut self, new_len: usize) {
        self.fixed.truncate(new_len);
    }

    /// Truncates this `MutBumpString`, removing all contents.
    ///
    /// While this means the `MutBumpString` will have a length of zero, it does not
    /// touch its capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, MutBumpString };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut s = MutBumpString::from_str_in("foo", &mut bump);
    ///
    /// s.clear();
    ///
    /// assert!(s.is_empty());
    /// assert_eq!(s.len(), 0);
    /// assert!(s.capacity() >= 3);
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.fixed.clear();
    }

    /// Converts a vector of bytes to a `MutBumpString`.
    ///
    /// A string ([`MutBumpString`]) is made of bytes ([`u8`]), and a vector of bytes
    /// ([`MutBumpVec<u8>`]) is made of bytes, so this function converts between the
    /// two. Not all byte slices are valid `MutBumpString`s, however: `MutBumpString`
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
    /// If you need a [`&str`] instead of a `MutBumpString`, consider
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
    /// # use bump_scope::{ Bump, mut_bump_vec, MutBumpString };
    /// # let mut bump: Bump = Bump::new();
    /// // some bytes, in a vector
    /// let sparkle_heart = mut_bump_vec![in bump; 240, 159, 146, 150];
    ///
    /// // We know these bytes are valid, so we'll use `unwrap()`.
    /// let sparkle_heart = MutBumpString::from_utf8(sparkle_heart).unwrap();
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    ///
    /// Incorrect bytes:
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec, MutBumpString };
    /// # let mut bump: Bump = Bump::new();
    /// // some invalid bytes, in a vector
    /// let sparkle_heart = mut_bump_vec![in bump; 0, 159, 146, 150];
    ///
    /// assert!(MutBumpString::from_utf8(sparkle_heart).is_err());
    /// ```
    ///
    /// [`from_utf8_unchecked`]: Self::from_utf8_unchecked
    /// [`MutBumpVec<u8>`]: MutBumpVec
    /// [`&str`]: prim@str "&str"
    /// [`into_bytes`]: Self::into_bytes
    pub fn from_utf8(
        vec: MutBumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
    ) -> Result<Self, FromUtf8Error<MutBumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>> {
        #[allow(clippy::missing_transmute_annotations)]
        match str::from_utf8(vec.as_slice()) {
            // SAFETY: `MutBumpVec<u8>` and `MutBumpString` have the same representation;
            // only the invariant that the bytes are utf8 is different.
            Ok(_) => Ok(unsafe { mem::transmute(vec) }),
            Err(error) => Err(FromUtf8Error { error, bytes: vec }),
        }
    }

    /// Converts a vector of bytes to a `MutBumpString` without checking that the
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
    /// # use bump_scope::{ Bump, mut_bump_vec, MutBumpString };
    /// # let mut bump: Bump = Bump::new();
    /// // some bytes, in a vector
    /// let sparkle_heart = mut_bump_vec![in bump; 240, 159, 146, 150];
    ///
    /// let sparkle_heart = unsafe {
    ///     MutBumpString::from_utf8_unchecked(sparkle_heart)
    /// };
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    #[must_use]
    pub unsafe fn from_utf8_unchecked(vec: MutBumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        debug_assert!(str::from_utf8(vec.as_slice()).is_ok());

        // SAFETY: `MutBumpVec<u8>` and `MutBumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        mem::transmute(vec)
    }

    /// Converts a `MutBumpString` into a `MutBumpVec<u8>`.
    ///
    /// This consumes the `MutBumpString`, so we do not need to copy its contents.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, MutBumpString };
    /// # let mut bump: Bump = Bump::new();
    /// let mut s = MutBumpString::new_in(&mut bump);
    /// s.push_str("hello");
    /// let bytes = s.into_bytes();
    ///
    /// assert_eq!(&[104, 101, 108, 108, 111][..], &bytes[..]);
    /// ```
    #[inline(always)]
    #[must_use = "`self` will be dropped if the result is not used"]
    pub fn into_bytes(self) -> MutBumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `MutBumpVec<u8>` and `MutBumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        unsafe { mem::transmute(self) }
    }

    /// Returns a byte slice of this `MutBumpString`'s contents.
    #[must_use]
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.fixed.as_bytes()
    }

    /// Extracts a string slice containing the entire `MutBumpString`.
    #[must_use]
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.fixed.as_str()
    }

    /// Converts a `MutBumpString` into a mutable string slice.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_str(&mut self) -> &mut str {
        self.fixed.as_mut_str()
    }

    /// Returns a mutable reference to the contents of this `MutBumpString`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned `&mut Vec` allows writing
    /// bytes which are not valid UTF-8. If this constraint is violated, using
    /// the original `MutBumpString` after dropping the `&mut Vec` may violate memory
    /// safety, as `MutBumpString`s must be valid UTF-8.
    #[must_use]
    #[inline(always)]
    pub unsafe fn as_mut_vec(&mut self) -> &mut MutBumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `MutBumpVec<u8>` and `MutBumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        transmute_mut(self)
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
    /// # use bump_scope::{ Bump, MutBumpString };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut s = MutBumpString::from_str_in("abç", &mut bump);
    ///
    /// assert_eq!(s.remove(0), 'a');
    /// assert_eq!(s.remove(1), 'ç');
    /// assert_eq!(s.remove(0), 'b');
    /// ```
    #[inline]
    pub fn remove(&mut self, idx: usize) -> char {
        self.fixed.remove(idx)
    }
}

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    error_behavior_generic_methods_allocation_failure! {
        /// Appends the given [`char`] to the end of this `MutBumpString`.
        impl
        for fn push
        for fn try_push
        use fn generic_push(&mut self, ch: char) {
            let vec = unsafe { self.as_mut_vec() };

            match ch.len_utf8() {
                1 => vec.generic_push(ch as u8),
                _ => vec.generic_extend_from_slice_copy(ch.encode_utf8(&mut [0; 4]).as_bytes()),
            }
        }

        /// Appends a given string slice onto the end of this `MutBumpString`.
        impl
        for fn push_str
        for fn try_push_str
        use fn generic_push_str(&mut self, string: &str) {
            let vec = unsafe { self.as_mut_vec() };
            vec.generic_extend_from_slice_copy(string.as_bytes())
        }

        /// Inserts a character into this `MutBumpString` at a byte position.
        ///
        /// This is an *O*(*n*) operation as it requires copying every element in the
        /// buffer.
        do panics
        /// Panics if `idx` is larger than the `String`'s length, or if it does not
        /// lie on a [`char`] boundary.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::new();
        /// let mut s = MutBumpString::with_capacity_in(3, &mut bump);
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
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut s = MutBumpString::try_with_capacity_in(3, &mut bump)?;
        ///
        /// s.try_insert(0, 'f')?;
        /// s.try_insert(1, 'o')?;
        /// s.try_insert(2, 'o')?;
        ///
        /// assert_eq!("foo", s);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_insert
        use fn generic_insert(&mut self, idx: usize, ch: char) {
            assert!(self.is_char_boundary(idx));
            let mut bits = [0; 4];
            let bits = ch.encode_utf8(&mut bits).as_bytes();

            unsafe {
                self.insert_bytes(idx, bits)
            }
        }

        /// Inserts a string slice into this `MutBumpString` at a byte position.
        ///
        /// This is an *O*(*n*) operation as it requires copying every element in the
        /// buffer.
        do panics
        /// Panics if `idx` is larger than the `MutBumpString`'s length, or if it does not
        /// lie on a [`char`] boundary.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::new();
        /// let mut s = MutBumpString::from_str_in("bar", &mut bump);
        ///
        /// s.insert_str(0, "foo");
        ///
        /// assert_eq!("foobar", s);
        /// ```
        for fn insert_str
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut s = MutBumpString::try_from_str_in("bar", &mut bump)?;
        ///
        /// s.try_insert_str(0, "foo")?;
        ///
        /// assert_eq!("foobar", s);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_insert_str
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
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::new();
        /// let mut string = MutBumpString::from_str_in("abcde", &mut bump);
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
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut string = MutBumpString::try_from_str_in("abcde", &mut bump)?;
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
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::new();
        /// let mut string = MutBumpString::from_str_in("What?", &mut bump);
        /// string.extend_zeroed(3);
        /// assert_eq!(string, "What?\0\0\0");
        /// ```
        for fn extend_zeroed
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, MutBumpString };
        /// # let mut bump: Bump = Bump::new();
        /// let mut string = MutBumpString::try_from_str_in("What?", &mut bump)?;
        /// string.try_extend_zeroed(3)?;
        /// assert_eq!(string, "What?\0\0\0");
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_extend_zeroed
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

        /// Reserves capacity for at least `additional` bytes more than the
        /// current length. The allocator may reserve more space to speculatively
        /// avoid frequent allocations. After calling `reserve`,
        /// capacity will be greater than or equal to `self.len() + additional`.
        /// Does nothing if capacity is already sufficient.
        impl
        for fn reserve
        for fn try_reserve
        use fn generic_reserve(&mut self, additional: usize) {
            let vec = unsafe { self.as_mut_vec() };

            vec.generic_reserve(additional)
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

    /// Converts a `MutBumpString` into a `BumpBox<str>`.
    ///
    /// Unused capacity does not take up space.<br/>
    /// When [bumping downwards](crate#bumping-upwards-or-downwards) this needs to shift all elements to the other end of the chunk.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_str(self) -> BumpBox<'a, str> {
        let bytes = self.into_bytes().into_boxed_slice();
        unsafe { bytes.into_boxed_str_unchecked() }
    }

    /// Converts this `MutBumpString` into `&str` that is live for this bump scope.
    ///
    /// Unused capacity does not take up space.<br/>
    /// When [bumping downwards](crate#bumping-upwards-or-downwards) this needs to shift all elements to the other end of the chunk.
    #[must_use]
    #[inline(always)]
    pub fn into_str(self) -> &'a mut str {
        self.into_boxed_str().into_mut()
    }

    #[doc = include_str!("docs/allocator.md")]
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        self.bump.allocator()
    }
}

impl<'b, 'a: 'b, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[doc = include_str!("docs/stats.md")]
    #[doc = include_str!("docs/stats_mut_collection_addendum.md")]
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, UP> {
        self.bump.stats()
    }
}

impl<'b, 'a: 'b, A, const MIN_ALIGN: usize, const UP: bool> MutBumpString<'b, 'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    #[doc = include_str!("docs/stats.md")]
    #[doc = include_str!("docs/stats_mut_collection_addendum.md")]
    #[must_use]
    #[inline(always)]
    pub fn guaranteed_allocated_stats(&self) -> GuaranteedAllocatedStats<'a, UP> {
        self.bump.guaranteed_allocated_stats()
    }
}

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> fmt::Write
    for MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.try_push_str(s).map_err(|_| fmt::Error)
    }

    #[inline(always)]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.try_push(c).map_err(|_| fmt::Error)
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> fmt::Write
    for Infallibly<MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(s);
        Ok(())
    }

    #[inline(always)]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.push(c);
        Ok(())
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Debug
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Display
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Deref
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> DerefMut
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

#[cfg(not(no_global_oom_handling))]
impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> core::ops::AddAssign<&str>
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> AsRef<str>
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> AsMut<str>
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Borrow<str>
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> BorrowMut<str>
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
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
            impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq<$string_like> for MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
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
            impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq<MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for $string_like {
                #[inline]
                fn eq(&self, other: &MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
                    <str as PartialEq>::eq(self, other)
                }

                #[inline]
                fn ne(&self, other: &MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
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

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Eq
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialOrd
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
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

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Ord
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        <str as Ord>::cmp(self, other)
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Hash
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'s, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Extend<&'s str>
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn extend<T: IntoIterator<Item = &'s str>>(&mut self, iter: T) {
        for str in iter {
            self.push_str(str);
        }
    }
}

#[cfg(feature = "alloc")]
impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    From<MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for alloc::string::String
{
    #[inline]
    fn from(value: MutBumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        value.as_str().into()
    }
}
