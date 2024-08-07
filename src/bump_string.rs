use core::{
    borrow::{Borrow, BorrowMut},
    fmt::{self, Debug, Display},
    hash::Hash,
    ops::{Deref, DerefMut, Range, RangeBounds},
    ptr, str,
};

use crate::{
    error_behavior_generic_methods_allocation_failure, polyfill, BaseAllocator, BumpBox, BumpScope, BumpVec, ErrorBehavior,
    FromUtf8Error, GuaranteedAllocatedStats, MinimumAlignment, Stats, SupportedMinimumAlignment,
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
        let mut string = $crate::BumpString::new_in($bump.as_scope());
        match $crate::private::core::fmt::Write::write_fmt(&mut string, $crate::private::core::format_args!($($arg)*)) {
            $crate::private::core::result::Result::Ok(_) => string,
            $crate::private::core::result::Result::Err(_) => $crate::private::capacity_overflow(),
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
        pub struct BumpString<
            'b,
            'a: 'b,
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        > {
            pub(crate) vec: BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
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
            vec: BumpVec::new_in(bump),
        }
    }

    error_behavior_generic_methods_allocation_failure! {
        /// Constructs a new empty `BumpString` with the specified capacity
        /// in the provided `BumpScope`.
        ///
        /// The string will be able to hold `capacity` bytes without
        /// reallocating. If `capacity` is 0, the string will not allocate.
        impl
        for pub fn with_capacity_in
        for pub fn try_with_capacity_in
        fn generic_with_capacity_in(capacity: usize, bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            Ok(Self { vec: BumpVec::generic_with_capacity_in(capacity, bump.into())? } )
        }

        /// Constructs a new `BumpString` from a `&str`.
        impl
        for pub fn from_str_in
        for pub fn try_from_str_in
        fn generic_from_str_in(string: &str, bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            let mut this = Self::new_in(bump);
            this.generic_push_str(string)?;
            Ok(this)
        }
    }
}

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    /// Returns this `BumpString`'s capacity, in bytes.
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// Returns the length of this `BumpString`, in bytes, not [`char`]s or
    /// graphemes. In other words, it might not be what a human considers the
    /// length of the string.
    #[must_use]
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns `true` if this `BumpString` has a length of zero, and `false` otherwise.
    #[must_use]
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Truncates this `BumpString`, removing all contents.
    ///
    /// While this means the `BumpString` will have a length of zero, it does not
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
        self.vec.clear();
    }

    /// Converts a bump allocated vector of bytes to a `BumpString`.
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
    /// [`from_utf8_unchecked`]: String::from_utf8_unchecked
    /// [`BumpVec<u8>`]: BumpVec
    /// [`&str`]: prim@str "&str"
    /// [`into_bytes`]: BumpString::into_bytes
    pub fn from_utf8(
        vec: BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
    ) -> Result<Self, FromUtf8Error<BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>> {
        match str::from_utf8(vec.as_slice()) {
            Ok(_) => Ok(Self { vec }),
            Err(error) => Err(FromUtf8Error { error, bytes: vec }),
        }
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
        self.vec
    }

    /// Returns a byte slice of this `BumpString`'s contents.
    #[must_use]
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.vec.as_slice()
    }

    /// Extracts a string slice containing the entire `BumpString`.
    #[must_use]
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.vec.as_slice()) }
    }

    /// Converts a `BumpString` into a mutable string slice.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.vec.as_mut_slice()) }
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
    pub unsafe fn as_mut_vec(&mut self) -> &BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
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

impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    error_behavior_generic_methods_allocation_failure! {
        /// Appends the given [`char`] to the end of this `BumpString`.
        impl
        for pub fn push
        for pub fn try_push
        fn generic_push(&mut self, ch: char) {
            match ch.len_utf8() {
                1 => self.vec.generic_push(ch as u8),
                _ => self.vec.generic_extend_from_slice_copy(ch.encode_utf8(&mut [0; 4]).as_bytes()),
            }
        }

        /// Appends a given string slice onto the end of this `BumpString`.
        impl
        for pub fn push_str
        for pub fn try_push_str
        fn generic_push_str(&mut self, string: &str) {
            self.vec.generic_extend_from_slice_copy(string.as_bytes())
        }

        /// Inserts a character into this `BumpString` at a byte position.
        ///
        /// This is an *O*(*n*) operation as it requires copying every element in the
        /// buffer.
        do panics
        /// Panics if `idx` is larger than the `String`'s length, or if it does not
        /// lie on a [`char`] boundary.
        do examples
        /// ```
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// #
        /// let mut s = BumpString::with_capacity_in(3, &bump);
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

        /// Inserts a string slice into this `BumpString` at a byte position.
        ///
        /// This is an *O*(*n*) operation as it requires copying every element in the
        /// buffer.
        do panics
        /// Panics if `idx` is larger than the `BumpString`'s length, or if it does not
        /// lie on a [`char`] boundary.
        do examples
        /// ```
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// #
        /// let mut s = BumpString::from_str_in("bar", &bump);
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
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// #
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
        /// # use bump_scope::{ Bump, BumpString };
        /// # let bump: Bump = Bump::new();
        /// let mut string = BumpString::from_str_in("What?", &bump);
        /// string.extend_zeroed(3);
        /// assert_eq!(string, "What?\0\0\0");
        /// ```
        for pub fn extend_zeroed
        for pub fn try_extend_zeroed
        fn generic_extend_zeroed(&mut self, additional: usize) {
            self.generic_reserve(additional)?;

            unsafe {
                let ptr = self.vec.as_mut_ptr();
                let len = self.len();

                ptr.add(len).write_bytes(0, additional);
                self.vec.set_len(len + additional);
            }

            Ok(())
        }

        /// Reserves capacity for at least `additional` bytes more than the
        /// current length. The allocator may reserve more space to speculatively
        /// avoid frequent allocations. After calling `reserve`,
        /// capacity will be greater than or equal to `self.len() + additional`.
        /// Does nothing if capacity is already sufficient.
        impl
        for pub fn reserve
        for pub fn try_reserve
        fn generic_reserve(&mut self, additional: usize) {
            self.vec.generic_reserve(additional)
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

    /// Shrinks the capacity of the string as much as possible.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut string = BumpString::with_capacity_in(10, &bump);
    /// string.push_str("123");
    /// assert!(string.capacity() >= 10);
    /// string.shrink_to_fit();
    /// assert!(string.capacity() == 3);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit();
    }

    /// Converts a `BumpString` into a `BumpBox<str>`.
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

    #[doc = include_str!("docs/allocator.md")]
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        self.vec.allocator()
    }

    #[doc = include_str!("docs/bump.md")]
    #[must_use]
    #[inline(always)]
    pub fn bump(&self) -> &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        self.vec.bump()
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
        self.vec.stats()
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
        self.vec.guaranteed_allocated_stats()
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

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Debug
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Display
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Deref
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> DerefMut
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> core::ops::AddAssign<&str>
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> AsRef<str>
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> AsMut<str>
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Borrow<str>
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> BorrowMut<str>
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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
            impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq<$string_like> for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
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
            impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq<BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for $string_like {
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

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Eq
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialOrd
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Ord
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        <str as Ord>::cmp(self, other)
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Hash
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.vec.hash(state);
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'s, 'b, 'a, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Extend<&'s str>
    for BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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
{
    #[inline]
    fn from(value: BumpString<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        value.as_str().into()
    }
}
