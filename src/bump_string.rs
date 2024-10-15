use allocator_api2::alloc::Allocator;

#[cfg(not(no_global_oom_handling))]
use crate::Infallibly;
use crate::{
    error_behavior_generic_methods_allocation_failure, owned_str,
    polyfill::{self, transmute_mut},
    BaseAllocator, BumpBox, BumpScope, BumpVec, ErrorBehavior, FixedBumpString, FromUtf8Error, GuaranteedAllocatedStats,
    MinimumAlignment, Stats, SupportedMinimumAlignment,
};
use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    fmt::{self, Debug, Display},
    hash::Hash,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut, Range, RangeBounds},
    ptr, str,
};

/// This is like [`format!`] but allocates inside a `Bump` or `BumpScope`, returning a [`BumpString`].
///
/// If you don't need to push to the string after creation you can also use [`Bump::alloc_fmt`](crate::Bump::alloc_fmt).
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
/// # use bump_scope::{ Bump, bump_format };
/// # let bump: Bump = Bump::new();
/// #
/// let greeting = "Hello";
/// let mut string = bump_format!(in bump, "{greeting} world!");
/// string.push_str(" How are you?");
///
/// assert_eq!(string, "Hello world! How are you?");
/// ```
#[macro_export]
macro_rules! bump_format {
    (in $bump:expr) => {{
        $crate::BumpString::new_in($bump.as_scope())
    }};
    (in $bump:expr, $($arg:tt)*) => {{
        let mut string = $crate::private::Infallibly($crate::BumpString::new_in($bump.as_scope()));
        match $crate::private::core::fmt::Write::write_fmt(&mut string, $crate::private::core::format_args!($($arg)*)) {
            $crate::private::core::result::Result::Ok(_) => string.0,
            $crate::private::core::result::Result::Err(_) => $crate::private::format_trait_error(),
        }
    }};
    (try in $bump:expr) => {{
        Ok::<_, $crate::allocator_api2::alloc::AllocError>($crate::BumpString::new_in($bump.as_scope()))
    }};
    (try in $bump:expr, $($arg:tt)*) => {{
        let mut string = $crate::BumpString::new_in($bump.as_scope());
        match $crate::private::core::fmt::Write::write_fmt(&mut string, $crate::private::core::format_args!($($arg)*)) {
            $crate::private::core::result::Result::Ok(_) => $crate::private::core::result::Result::Ok(string),
            $crate::private::core::result::Result::Err(_) => $crate::private::core::result::Result::Err($crate::allocator_api2::alloc::AllocError),
        }
    }};
}

macro_rules! bump_string_declaration {
    ($($allocator_parameter:tt)*) => {
        /// A bump allocated [`String`].
        ///
        /// When you are done building the string, you can turn it into a `&str` with [`into_str`].
        ///
        /// # Examples
        ///
        /// You can create a `BumpString` from [a literal string][`&str`] with [`BumpString::from_str_in`]:
        ///
        /// [`into_str`]: BumpString::into_str
        ///
        /// ```
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// let hello = BumpString::from_str_in("Hello, world!", &bump);
        /// ```
        ///
        /// You can append a [`char`] to a `String` with the [`push`] method, and
        /// append a [`&str`] with the [`push_str`] method:
        ///
        /// ```
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// let mut hello = BumpString::from_str_in("Hello, ", &bump);
        ///
        /// hello.push('w');
        /// hello.push_str("orld!");
        ///
        /// assert_eq!(hello.as_str(), "Hello, world!");
        /// ```
        ///
        /// [`push`]: BumpString::push
        /// [`push_str`]: BumpString::push_str
        ///
        /// If you have a vector of UTF-8 bytes, you can create a `BumpString` from it with
        /// the [`from_utf8`] method:
        ///
        /// ```
        /// # use bump_scope::{ Bump, BumpString, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// // some bytes, in a vector
        /// let sparkle_heart = bump_vec![in bump; 240, 159, 146, 150];
        ///
        /// // We know these bytes are valid, so we'll use `unwrap()`.
        /// let sparkle_heart = BumpString::from_utf8(sparkle_heart).unwrap();
        ///
        /// assert_eq!("💖", sparkle_heart);
        /// ```
        ///
        /// [`&str`]: prim@str "&str"
        /// [`from_utf8`]: BumpString::from_utf8
        // `BumpString` and `BumpVec<u8>` have the same repr.
        #[repr(C)]
        pub struct BumpString<
            'b,
            'a: 'b,
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        >
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
            pub(crate) fixed: FixedBumpString<'a>,
            pub(crate) bump: &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
        }
    };
}

crate::maybe_default_allocator!(bump_string_declaration);

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Constructs a new empty `BumpString`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    #[inline]
    pub fn new_in(bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
        Self {
            fixed: FixedBumpString::EMPTY,
            bump: bump.into(),
        }
    }

    error_behavior_generic_methods_allocation_failure! {
        /// Constructs a new empty `BumpString` with the specified capacity
        /// in the provided `BumpScope`.
        ///
        /// The string will be able to hold `capacity` bytes without
        /// reallocating. If `capacity` is 0, the string will not allocate.
        impl
        for fn with_capacity_in
        for fn try_with_capacity_in
        use fn generic_with_capacity_in(capacity: usize, bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            let bump = bump.into();

            if capacity == 0 {
                return Ok(Self {
                    fixed: FixedBumpString::EMPTY,
                    bump,
                });
            }

            Ok(Self {
                fixed: bump.generic_alloc_fixed_string(capacity)?,
                bump,
            })
        }

        /// Constructs a new `BumpString` from a `&str`.
        impl
        for fn from_str_in
        for fn try_from_str_in
        use fn generic_from_str_in(string: &str, bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            let mut this = Self::new_in(bump);
            this.generic_push_str(string)?;
            Ok(this)
        }
    }
}

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Converts a vector of bytes to a `BumpString`.
    ///
    /// A string ([`BumpString`]) is made of bytes ([`u8`]), and a vector of bytes
    /// ([`BumpVec<u8>`]) is made of bytes, so this function converts between the
    /// two. Not all byte slices are valid `BumpString`s, however: `BumpString`
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
    /// If you need a [`&str`] instead of a `BumpString`, consider
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
    /// # use bump_scope::{ Bump, bump_vec, BumpString };
    /// # let bump: Bump = Bump::new();
    /// // some bytes, in a vector
    /// let sparkle_heart = bump_vec![in bump; 240, 159, 146, 150];
    ///
    /// // We know these bytes are valid, so we'll use `unwrap()`.
    /// let sparkle_heart = BumpString::from_utf8(sparkle_heart).unwrap();
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    ///
    /// Incorrect bytes:
    /// ```
    /// # use bump_scope::{ Bump, bump_vec, BumpString };
    /// # let bump: Bump = Bump::new();
    /// // some invalid bytes, in a vector
    /// let sparkle_heart = bump_vec![in bump; 0, 159, 146, 150];
    ///
    /// assert!(BumpString::from_utf8(sparkle_heart).is_err());
    /// ```
    ///
    /// [`from_utf8_unchecked`]: Self::from_utf8_unchecked
    /// [`BumpVec<u8>`]: BumpVec
    /// [`&str`]: prim@str "&str"
    /// [`into_bytes`]: Self::into_bytes
    pub fn from_utf8(
        vec: BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
    ) -> Result<Self, FromUtf8Error<BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>> {
        #[allow(clippy::missing_transmute_annotations)]
        match str::from_utf8(vec.as_slice()) {
            // SAFETY: `BumpVec<u8>` and `BumpString` have the same representation;
            // only the invariant that the bytes are utf8 is different.
            Ok(_) => Ok(unsafe { mem::transmute(vec) }),
            Err(error) => Err(FromUtf8Error { error, bytes: vec }),
        }
    }

    /// Converts a vector of bytes to a `BumpString` without checking that the
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
    /// # use bump_scope::{ Bump, bump_vec, BumpString };
    /// # let bump: Bump = Bump::new();
    /// // some bytes, in a vector
    /// let sparkle_heart = bump_vec![in bump; 240, 159, 146, 150];
    ///
    /// let sparkle_heart = unsafe {
    ///     BumpString::from_utf8_unchecked(sparkle_heart)
    /// };
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    #[must_use]
    pub unsafe fn from_utf8_unchecked(vec: BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        debug_assert!(str::from_utf8(vec.as_slice()).is_ok());

        // SAFETY: `BumpVec<u8>` and `BumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        mem::transmute(vec)
    }

    /// Returns this string's capacity, in bytes.
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.fixed.capacity()
    }

    /// Returns the length of this string, in bytes, not [`char`]s or
    /// graphemes. In other words, it might not be what a human considers the
    /// length of the string.
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.fixed.len()
    }

    /// Returns `true` if this string has a length of zero, and `false` otherwise.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.fixed.is_empty()
    }

    /// Converts this `BumpString` into `&str` that is live for this bump scope.
    #[must_use]
    #[inline(always)]
    pub fn into_str(self) -> &'a mut str {
        self.into_boxed_str().into_mut()
    }

    /// Converts a `BumpString` into a `BumpBox<str>`.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_str(mut self) -> BumpBox<'a, str> {
        self.shrink_to_fit();
        self.into_fixed_string().into_boxed_str()
    }

    /// Turns this `BumpString` into a `FixedBumpString`.
    ///
    /// This retains the unused capacity unlike <code>[into_](Self::into_str)([boxed_](Self::into_boxed_str))[str](Self::into_str)</code>.
    #[must_use]
    #[inline(always)]
    pub fn into_fixed_string(self) -> FixedBumpString<'a> {
        self.into_parts().0
    }

    /// Converts a `BumpString` into a `BumpVec<u8>`.
    ///
    /// This consumes the `BumpString`, so we do not need to copy its contents.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::new_in(&bump);
    /// s.push_str("hello");
    /// let bytes = s.into_bytes();
    ///
    /// assert_eq!(&[104, 101, 108, 108, 111][..], &bytes[..]);
    /// ```
    #[inline(always)]
    #[must_use = "`self` will be dropped if the result is not used"]
    pub fn into_bytes(self) -> BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `BumpVec<u8>` and `BumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        unsafe { mem::transmute(self) }
    }

    /// Removes the last character from the string buffer and returns it.
    ///
    /// Returns [`None`] if this string is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::from_str_in("abč", &bump);
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

    /// Truncates this string, removing all contents.
    ///
    /// While this means the string will have a length of zero, it does not
    /// touch its capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut s = BumpString::from_str_in("foo", &bump);
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
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::from_str_in("hello", &bump);
    ///
    /// s.truncate(2);
    ///
    /// assert_eq!(s, "he");
    /// ```
    #[inline]
    pub fn truncate(&mut self, new_len: usize) {
        self.fixed.truncate(new_len);
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
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut s = BumpString::from_str_in("abç", &bump);
    ///
    /// assert_eq!(s.remove(0), 'a');
    /// assert_eq!(s.remove(1), 'ç');
    /// assert_eq!(s.remove(0), 'b');
    /// ```
    #[inline]
    pub fn remove(&mut self, idx: usize) -> char {
        self.fixed.remove(idx)
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
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::from_str_in("α is alpha, β is beta", &bump);
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
        self.fixed.drain(range)
    }

    /// Extracts a string slice containing the entire `BumpString`.
    #[must_use]
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.fixed.as_str()
    }

    /// Converts a `BumpString` into a mutable string slice.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_str(&mut self) -> &mut str {
        self.fixed.as_mut_str()
    }

    /// Returns a byte slice of this `BumpString`'s contents.
    #[must_use]
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.fixed.as_bytes()
    }

    /// Returns a mutable reference to the contents of this `BumpString`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned `&mut Vec` allows writing
    /// bytes which are not valid UTF-8. If this constraint is violated, using
    /// the original `BumpString` after dropping the `&mut Vec` may violate memory
    /// safety, as `BumpString`s must be valid UTF-8.
    #[must_use]
    #[inline(always)]
    pub unsafe fn as_mut_vec(&mut self) -> &mut BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `BumpVec<u8>` and `BumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        transmute_mut(self)
    }
}

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    error_behavior_generic_methods_allocation_failure! {
        /// Appends the given [`char`] to the end of this string.
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

        /// Appends a given string slice onto the end of this string.
        impl
        for fn push_str
        for fn try_push_str
        use fn generic_push_str(&mut self, string: &str) {
            let vec = unsafe { self.as_mut_vec() };
            vec.generic_extend_from_slice_copy(string.as_bytes())
        }

        /// Inserts a character into this string at a byte position.
        ///
        /// This is an *O*(*n*) operation as it requires copying every element in the
        /// buffer.
        do panics
        /// Panics if `idx` is larger than the `String`'s length, or if it does not
        /// lie on a [`char`] boundary.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// let mut s = BumpString::with_capacity_in(3, &bump);
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
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut s = BumpString::try_with_capacity_in(3, &bump)?;
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
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// let mut s = BumpString::from_str_in("bar", &bump);
        ///
        /// s.insert_str(0, "foo");
        ///
        /// assert_eq!("foobar", s);
        /// ```
        for fn insert_str
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut s = BumpString::try_from_str_in("bar", &bump)?;
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
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// let mut string = BumpString::from_str_in("abcde", &bump);
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
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut string = BumpString::try_from_str_in("abcde", &bump)?;
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
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// let mut string = BumpString::from_str_in("What?", &bump);
        /// string.extend_zeroed(3);
        /// assert_eq!(string, "What?\0\0\0");
        /// ```
        for fn extend_zeroed
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut string = BumpString::try_from_str_in("What?", &bump)?;
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

    /// Shrinks the capacity of the string as much as possible.
    ///
    /// This will also free space for future bump allocations iff this is the most recent allocation.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut string = BumpString::with_capacity_in(10, &bump);
    /// string.push_str("123");
    /// assert!(string.capacity() == 10);
    /// assert_eq!(bump.stats().allocated(), 10);
    /// string.shrink_to_fit();
    /// assert!(string.capacity() == 3);
    /// assert_eq!(bump.stats().allocated(), 3);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        let vec = unsafe { self.as_mut_vec() };
        vec.shrink_to_fit();
    }

    /// Creates a `BumpString` from its parts.
    ///
    /// The provided `bump` does not have to be the one the `fixed_string` was allocated in.
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut fixed_string = bump.alloc_fixed_string(3);
    /// fixed_string.push('a');
    /// fixed_string.push('b');
    /// fixed_string.push('c');
    /// let mut string = BumpString::from_parts(fixed_string, &bump);
    /// string.push('d');
    /// string.push('e');
    /// assert_eq!(string, "abcde");
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn from_parts(
        fixed_string: FixedBumpString<'a>,
        bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>,
    ) -> Self {
        Self {
            fixed: fixed_string,
            bump: bump.into(),
        }
    }

    /// Turns this `BumpString` into its parts.
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut string = BumpString::new_in(&bump);
    /// string.reserve(3);
    /// string.push('a');
    /// let mut fixed_string = string.into_parts().0;
    /// assert_eq!(fixed_string.capacity(), 3);
    /// assert_eq!(fixed_string, "a");
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn into_parts(self) -> (FixedBumpString<'a>, &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) {
        let mut this = ManuallyDrop::new(self);
        let bump = this.bump;
        let fixed = mem::take(&mut this.fixed);
        (fixed, bump)
    }

    #[doc = include_str!("docs/allocator.md")]
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        self.bump.allocator()
    }

    #[doc = include_str!("docs/bump.md")]
    #[must_use]
    #[inline(always)]
    pub fn bump(&self) -> &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        self.bump
    }
}

impl<'b, 'a: 'b, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[doc = include_str!("docs/stats.md")]
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, UP> {
        self.bump.stats()
    }
}

impl<'b, 'a: 'b, A, const MIN_ALIGN: usize, const UP: bool> BumpString<'b, 'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    #[doc = include_str!("docs/stats.md")]
    #[must_use]
    #[inline(always)]
    pub fn guaranteed_allocated_stats(&self) -> GuaranteedAllocatedStats<'a, UP> {
        self.bump.guaranteed_allocated_stats()
    }
}

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> fmt::Write
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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
    for Infallibly<BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>
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
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Display
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Deref
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> DerefMut
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

impl<'b, 'a: 'b, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn drop(&mut self) {
        unsafe {
            let ptr = self.fixed.initialized.ptr.cast();
            let layout = Layout::from_size_align_unchecked(self.fixed.capacity, 1);
            self.bump.deallocate(ptr, layout);
        }
    }
}

#[cfg(not(no_global_oom_handling))]
impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> core::ops::AddAssign<&str>
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> AsMut<str>
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Borrow<str>
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> BorrowMut<str>
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
            impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq<$string_like> for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
            where
                MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
                A: BaseAllocator<GUARANTEED_ALLOCATED>,
            {
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
            impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq<BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for $string_like
            where
                MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
                A: BaseAllocator<GUARANTEED_ALLOCATED>,
            {
                #[inline]
                fn eq(&self, other: &BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
                    <str as PartialEq>::eq(self, other)
                }

                #[inline]
                fn ne(&self, other: &BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
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
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialOrd
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        <str as Ord>::cmp(self, other)
    }
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Hash
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'s, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Extend<&'s str>
    for BumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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
    From<BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for alloc::string::String
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn from(value: BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        value.as_str().into()
    }
}
