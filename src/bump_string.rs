use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    ffi::CStr,
    fmt::{self, Debug, Display},
    hash::Hash,
    ops::{Deref, DerefMut, Range, RangeBounds},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr, str,
};

use crate::{
    alloc::AllocError,
    collection_method_allocator_stats,
    destructure::destructure,
    owned_str,
    polyfill::{self, transmute_mut, transmute_value},
    raw_fixed_bump_string::RawFixedBumpString,
    stats::Stats,
    BumpAllocator, BumpAllocatorScope, BumpBox, BumpVec, ErrorBehavior, FixedBumpString, FromUtf16Error, FromUtf8Error,
};

#[cfg(feature = "panic-on-alloc")]
use crate::{panic_on_error, polyfill::nonnull, PanicsOnAlloc};

/// This is like [`format!`](alloc_crate::format) but allocates inside a bump allocator, returning a [`BumpString`].
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
/// let mut string = bump_format!(in &bump, "{greeting}, world!");
/// string.push_str(" How are you?");
///
/// assert_eq!(string, "Hello, world! How are you?");
/// ```
#[macro_export]
macro_rules! bump_format {
    (in $bump:expr) => {{
        $crate::BumpString::new_in($bump.as_scope())
    }};
    (in $bump:expr, $($arg:tt)*) => {{
        let mut string = $crate::private::PanicsOnAlloc($crate::BumpString::new_in($bump.as_scope()));
        match $crate::private::core::fmt::Write::write_fmt(&mut string, $crate::private::core::format_args!($($arg)*)) {
            $crate::private::core::result::Result::Ok(_) => string.0,
            $crate::private::core::result::Result::Err(_) => $crate::private::format_trait_error(),
        }
    }};
    (try in $bump:expr) => {{
        Ok::<_, $crate::alloc::AllocError>($crate::BumpString::new_in($bump.as_scope()))
    }};
    (try in $bump:expr, $($arg:tt)*) => {{
        let mut string = $crate::BumpString::new_in($bump.as_scope());
        match $crate::private::core::fmt::Write::write_fmt(&mut string, $crate::private::core::format_args!($($arg)*)) {
            $crate::private::core::result::Result::Ok(_) => $crate::private::core::result::Result::Ok(string),
            $crate::private::core::result::Result::Err(_) => $crate::private::core::result::Result::Err($crate::alloc::AllocError),
        }
    }};
}

/// A bump allocated [`String`](alloc_crate::string::String).
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
/// You can append a [`char`] to a string with the [`push`] method, and
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
/// let sparkle_heart = bump_vec![in &bump; 240, 159, 146, 150];
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
pub struct BumpString<A: BumpAllocator> {
    fixed: RawFixedBumpString,
    allocator: A,
}

impl<A: BumpAllocator + UnwindSafe> UnwindSafe for BumpString<A> {}
impl<A: BumpAllocator + RefUnwindSafe> RefUnwindSafe for BumpString<A> {}

impl<A: BumpAllocator> BumpString<A> {
    /// Constructs a new empty `BumpString`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    #[inline]
    pub fn new_in(allocator: A) -> Self {
        Self {
            fixed: RawFixedBumpString::EMPTY,
            allocator,
        }
    }

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
    /// let sparkle_heart = bump_vec![in &bump; 240, 159, 146, 150];
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
    /// let sparkle_heart = bump_vec![in &bump; 0, 159, 146, 150];
    ///
    /// assert!(BumpString::from_utf8(sparkle_heart).is_err());
    /// ```
    ///
    /// [`from_utf8_unchecked`]: Self::from_utf8_unchecked
    /// [`BumpVec<u8>`]: BumpVec
    /// [`&str`]: prim@str "&str"
    /// [`into_bytes`]: Self::into_bytes
    pub fn from_utf8(vec: BumpVec<u8, A>) -> Result<Self, FromUtf8Error<BumpVec<u8, A>>> {
        #[allow(clippy::missing_transmute_annotations)]
        match str::from_utf8(vec.as_slice()) {
            // SAFETY: `BumpVec<u8>` and `BumpString` have the same representation;
            // only the invariant that the bytes are utf8 is different.
            Ok(_) => Ok(unsafe { transmute_value(vec) }),
            Err(error) => Err(FromUtf8Error::new(error, vec)),
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
    /// let sparkle_heart = bump_vec![in &bump; 240, 159, 146, 150];
    ///
    /// let sparkle_heart = unsafe {
    ///     BumpString::from_utf8_unchecked(sparkle_heart)
    /// };
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    #[must_use]
    pub unsafe fn from_utf8_unchecked(vec: BumpVec<u8, A>) -> Self {
        debug_assert!(str::from_utf8(vec.as_slice()).is_ok());

        // SAFETY: `BumpVec<u8>` and `BumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        transmute_value(vec)
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
        self.fixed.len() == 0
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
    pub fn into_bytes(self) -> BumpVec<u8, A> {
        // SAFETY: `BumpVec<u8>` and `BumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        unsafe { transmute_value(self) }
    }

    #[track_caller]
    #[inline(always)]
    pub(crate) fn assert_char_boundary(&self, index: usize) {
        unsafe { self.fixed.cook_ref() }.assert_char_boundary(index);
    }

    /// Splits the string into two by removing the specified range.
    ///
    /// This method does not allocate and does not change the order of the elements.
    ///
    /// The excess capacity may end up in either string.
    /// This behavior is different from <code>String::[split_off](alloc_crate::string::String::split_off)</code> which allocates a new string for the split-off bytes
    /// so the original string keeps its capacity.
    /// If you rather want that behavior then you can write this instead:
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// # let mut string = BumpString::from_str_in("abcde", &bump);
    /// # let start = 1;
    /// # let end = 4;
    /// let mut other = BumpString::new_in(*string.allocator());
    /// other.push_str(&string[start..end]);
    /// string.drain(start..end);
    /// # assert_eq!(string, "ae");
    /// # assert_eq!(other, "bcd");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the starting point or end point do not lie on a [`char`] boundary, or if they're out of bounds.
    ///
    /// # Complexity
    ///
    /// This operation takes `O(1)` time if either the range starts at 0, ends at `len`, or is empty.
    /// Otherwise it takes `O(min(end, len - start))` time.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut string = BumpString::with_capacity_in(10, &bump);
    /// string.push_str("foobarbazqux");
    ///
    /// let foo = string.split_off(..3);
    /// assert_eq!(foo, "foo");
    /// assert_eq!(string, "barbazqux");
    ///
    /// let qux = string.split_off(6..);
    /// assert_eq!(qux, "qux");
    /// assert_eq!(string, "barbaz");
    ///
    /// let rb = string.split_off(2..4);
    /// assert_eq!(rb, "rb");
    /// assert_eq!(string, "baaz");
    ///
    /// let rest = string.split_off(..);
    /// assert_eq!(rest, "baaz");
    /// assert_eq!(string, "");
    /// ```
    #[inline]
    #[allow(clippy::return_self_not_must_use)]
    pub fn split_off(&mut self, range: impl RangeBounds<usize>) -> Self
    where
        A: Clone,
    {
        let other = unsafe { self.fixed.cook_mut() }.split_off(range);

        Self {
            fixed: unsafe { RawFixedBumpString::from_cooked(other) },
            allocator: self.allocator.clone(),
        }
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
        unsafe { self.fixed.cook_mut() }.pop()
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
        unsafe { self.fixed.cook_mut() }.clear();
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
        unsafe { self.fixed.cook_mut() }.truncate(new_len);
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
        unsafe { self.fixed.cook_mut() }.remove(idx)
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
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::from_str_in("f_o_ob_ar", &bump);
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
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::from_str_in("abcde", &bump);    ///
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
        unsafe { self.fixed.cook_mut() }.retain(f);
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
        unsafe { self.fixed.cook_mut() }.drain(range)
    }

    /// Extracts a string slice containing the entire `BumpString`.
    #[must_use]
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        unsafe { self.fixed.cook_ref() }.as_str()
    }

    /// Converts a `BumpString` into a mutable string slice.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { self.fixed.cook_mut() }.as_mut_str()
    }

    /// Returns a byte slice of this `BumpString`'s contents.
    #[must_use]
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { self.fixed.cook_ref() }.as_bytes()
    }

    /// Returns a mutable reference to the contents of this `BumpString`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned `&mut BumpVec<u8>` allows writing
    /// bytes which are not valid UTF-8. If this constraint is violated, using
    /// the original `BumpString` after dropping the `&mut BumpVec<u8>` may violate memory
    /// safety, as `BumpString`s must be valid UTF-8.
    #[must_use]
    #[inline(always)]
    pub unsafe fn as_mut_vec(&mut self) -> &mut BumpVec<u8, A> {
        // SAFETY: `BumpVec<u8>` and `BumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        transmute_mut(self)
    }

    /// Returns a raw pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const u8 {
        self.fixed.as_ptr()
    }

    /// Returns an unsafe mutable pointer to slice, or a dangling
    /// raw pointer valid for zero sized reads.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.fixed.as_mut_ptr()
    }

    #[inline(always)]
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        self.fixed.set_len(new_len);
    }

    /// Constructs a new empty `BumpString` with the specified capacity
    /// in the provided bump allocator.
    ///
    /// The string will be able to hold `capacity` bytes without
    /// reallocating. If `capacity` is 0, the string will not allocate.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_capacity_in(capacity: usize, allocator: A) -> Self {
        panic_on_error(Self::generic_with_capacity_in(capacity, allocator))
    }

    /// Constructs a new empty `BumpString` with the specified capacity
    /// in the provided bump allocator.
    ///
    /// The string will be able to hold `capacity` bytes without
    /// reallocating. If `capacity` is 0, the string will not allocate.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn try_with_capacity_in(capacity: usize, allocator: A) -> Result<Self, AllocError> {
        Self::generic_with_capacity_in(capacity, allocator)
    }

    #[inline]
    pub(crate) fn generic_with_capacity_in<E: ErrorBehavior>(capacity: usize, allocator: A) -> Result<Self, E> {
        if capacity == 0 {
            return Ok(Self {
                fixed: RawFixedBumpString::EMPTY,
                allocator,
            });
        }

        Ok(Self {
            fixed: unsafe { RawFixedBumpString::allocate(&allocator, capacity)? },
            allocator,
        })
    }

    /// Constructs a new `BumpString` from a `&str`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let string = BumpString::from_str_in("Hello!", &bump);
    /// assert_eq!(string, "Hello!");
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_str_in(string: &str, allocator: A) -> Self {
        panic_on_error(Self::generic_from_str_in(string, allocator))
    }

    /// Constructs a new `BumpString` from a `&str`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let string = BumpString::try_from_str_in("Hello!", &bump)?;
    /// assert_eq!(string, "Hello!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_str_in(string: &str, allocator: A) -> Result<Self, AllocError> {
        Self::generic_from_str_in(string, allocator)
    }

    #[inline]
    pub(crate) fn generic_from_str_in<E: ErrorBehavior>(string: &str, allocator: A) -> Result<Self, E> {
        let mut this = Self::generic_with_capacity_in(string.len(), allocator)?;

        unsafe {
            ptr::copy_nonoverlapping(string.as_ptr(), this.fixed.as_mut_ptr(), string.len());
            this.as_mut_vec().set_len(string.len());
        }

        Ok(this)
    }

    /// Converts a slice of bytes to a string, including invalid characters.
    ///
    /// Strings are made of bytes ([`u8`]), and a slice of bytes
    /// ([`&[u8]`][byteslice]) is made of bytes, so this function converts
    /// between the two. Not all byte slices are valid strings, however: strings
    /// are required to be valid UTF-8. During this conversion,
    /// `from_utf8_lossy()` will replace any invalid UTF-8 sequences with
    /// [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD], which looks like this: �
    ///
    /// [byteslice]: prim@slice
    /// [U+FFFD]: core::char::REPLACEMENT_CHARACTER
    ///
    /// If you are sure that the byte slice is valid UTF-8, and you don't want
    /// to incur the overhead of the conversion, there is an unsafe version
    /// of this function, [`from_utf8_unchecked`], which has the same behavior
    /// but skips the checks.
    ///
    /// [`from_utf8_unchecked`]: Self::from_utf8_unchecked
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// // some bytes, in a vector
    /// let sparkle_heart = [240, 159, 146, 150];
    ///
    /// let sparkle_heart = BumpString::from_utf8_lossy_in(&sparkle_heart, &bump);
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    ///
    /// Incorrect bytes:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// // some invalid bytes
    /// let input = b"Hello \xF0\x90\x80World";
    /// let output = BumpString::from_utf8_lossy_in(input, &bump);
    ///
    /// assert_eq!("Hello �World", output);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_utf8_lossy_in(v: &[u8], allocator: A) -> Self {
        panic_on_error(Self::generic_from_utf8_lossy_in(v, allocator))
    }

    /// Converts a slice of bytes to a string, including invalid characters.
    ///
    /// Strings are made of bytes ([`u8`]), and a slice of bytes
    /// ([`&[u8]`][byteslice]) is made of bytes, so this function converts
    /// between the two. Not all byte slices are valid strings, however: strings
    /// are required to be valid UTF-8. During this conversion,
    /// `from_utf8_lossy()` will replace any invalid UTF-8 sequences with
    /// [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD], which looks like this: �
    ///
    /// [byteslice]: prim@slice
    /// [U+FFFD]: core::char::REPLACEMENT_CHARACTER
    ///
    /// If you are sure that the byte slice is valid UTF-8, and you don't want
    /// to incur the overhead of the conversion, there is an unsafe version
    /// of this function, [`from_utf8_unchecked`], which has the same behavior
    /// but skips the checks.
    ///
    /// [`from_utf8_unchecked`]: Self::from_utf8_unchecked
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// // some bytes, in a vector
    /// let sparkle_heart = [240, 159, 146, 150];
    ///
    /// let sparkle_heart = BumpString::try_from_utf8_lossy_in(&sparkle_heart, &bump)?;
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Incorrect bytes:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// // some invalid bytes
    /// let input = b"Hello \xF0\x90\x80World";
    /// let output = BumpString::try_from_utf8_lossy_in(input, &bump)?;
    ///
    /// assert_eq!("Hello �World", output);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_utf8_lossy_in(v: &[u8], allocator: A) -> Result<Self, AllocError> {
        Self::generic_from_utf8_lossy_in(v, allocator)
    }

    pub(crate) fn generic_from_utf8_lossy_in<E: ErrorBehavior>(v: &[u8], allocator: A) -> Result<Self, E> {
        let mut iter = crate::polyfill::str::lossy::utf8_chunks(v);

        let first_valid = if let Some(chunk) = iter.next() {
            let valid = chunk.valid();
            if chunk.invalid().is_empty() {
                debug_assert_eq!(valid.len(), v.len());
                return Self::generic_from_str_in(valid, allocator);
            }
            valid
        } else {
            return Ok(Self::new_in(allocator));
        };

        const REPLACEMENT: &str = "\u{FFFD}";

        let mut res = Self::generic_with_capacity_in(v.len(), allocator)?;
        res.generic_push_str(first_valid)?;
        res.generic_push_str(REPLACEMENT)?;

        for chunk in iter {
            res.generic_push_str(chunk.valid())?;
            if !chunk.invalid().is_empty() {
                res.generic_push_str(REPLACEMENT)?;
            }
        }

        Ok(res)
    }

    /// Decode a UTF-16–encoded vector `v` into a `BumpString`, returning [`Err`]
    /// if `v` contains any invalid data.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// // 𝄞music
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0x0073, 0x0069, 0x0063];
    /// assert_eq!(BumpString::from_str_in("𝄞music", &bump),
    ///            BumpString::from_utf16_in(v, &bump).unwrap());
    ///
    /// // 𝄞mu<invalid>ic
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0xD800, 0x0069, 0x0063];
    /// assert!(BumpString::from_utf16_in(v, &bump).is_err());
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_utf16_in(v: &[u16], allocator: A) -> Result<Self, FromUtf16Error> {
        panic_on_error(Self::generic_from_utf16_in(v, allocator))
    }

    /// Decode a UTF-16–encoded vector `v` into a `BumpString`, returning [`Err`]
    /// if `v` contains any invalid data.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// // 𝄞music
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0x0073, 0x0069, 0x0063];
    /// assert_eq!(BumpString::try_from_str_in("𝄞music", &bump)?,
    ///            BumpString::try_from_utf16_in(v, &bump)?.unwrap());
    ///
    /// // 𝄞mu<invalid>ic
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0xD800, 0x0069, 0x0063];
    /// assert!(BumpString::try_from_utf16_in(v, &bump)?.is_err());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_utf16_in(v: &[u16], allocator: A) -> Result<Result<Self, FromUtf16Error>, AllocError> {
        Self::generic_from_utf16_in(v, allocator)
    }

    pub(crate) fn generic_from_utf16_in<E: ErrorBehavior>(
        v: &[u16],
        allocator: A,
    ) -> Result<Result<Self, FromUtf16Error>, E> {
        // This isn't done via collect::<Result<_, _>>() for performance reasons.
        // STD-FIXME: the function can be simplified again when #48994 is closed.
        let mut ret = Self::generic_with_capacity_in(v.len(), allocator)?;

        for c in char::decode_utf16(v.iter().copied()) {
            if let Ok(c) = c {
                ret.generic_push(c)?;
            } else {
                return Ok(Err(FromUtf16Error(())));
            }
        }

        Ok(Ok(ret))
    }

    /// Decode a UTF-16–encoded slice `v` into a `BumpString`, replacing
    /// invalid data with [the replacement character (`U+FFFD`)][U+FFFD].
    ///
    /// [U+FFFD]: core::char::REPLACEMENT_CHARACTER
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// // 𝄞mus<invalid>ic<invalid>
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0x0073, 0xDD1E, 0x0069, 0x0063,
    ///           0xD834];
    ///
    /// assert_eq!(BumpString::from_str_in("𝄞mus\u{FFFD}ic\u{FFFD}", &bump),
    ///            BumpString::from_utf16_lossy_in(v, &bump));
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_utf16_lossy_in(v: &[u16], allocator: A) -> Self {
        panic_on_error(Self::generic_from_utf16_lossy_in(v, allocator))
    }

    /// Decode a UTF-16–encoded slice `v` into a `BumpString`, replacing
    /// invalid data with [the replacement character (`U+FFFD`)][U+FFFD].
    ///
    /// [U+FFFD]: core::char::REPLACEMENT_CHARACTER
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// // 𝄞mus<invalid>ic<invalid>
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0x0073, 0xDD1E, 0x0069, 0x0063,
    ///           0xD834];
    ///
    /// assert_eq!(BumpString::try_from_str_in("𝄞mus\u{FFFD}ic\u{FFFD}", &bump)?,
    ///            BumpString::try_from_utf16_lossy_in(v, &bump)?);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_utf16_lossy_in(v: &[u16], allocator: A) -> Result<Self, AllocError> {
        Self::generic_from_utf16_lossy_in(v, allocator)
    }

    pub(crate) fn generic_from_utf16_lossy_in<E: ErrorBehavior>(v: &[u16], allocator: A) -> Result<Self, E> {
        let iter = char::decode_utf16(v.iter().copied());
        let capacity = iter.size_hint().0;
        let mut string = Self::generic_with_capacity_in(capacity, allocator)?;

        for r in iter {
            string.generic_push(r.unwrap_or(char::REPLACEMENT_CHARACTER))?;
        }

        Ok(string)
    }

    /// Appends the given [`char`] to the end of this string.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::from_str_in("abc", &bump);
    ///
    /// s.push('1');
    /// s.push('2');
    /// s.push('3');
    ///
    /// assert_eq!(s, "abc123");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn push(&mut self, ch: char) {
        panic_on_error(self.generic_push(ch))
    }

    /// Appends the given [`char`] to the end of this string.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = BumpString::try_from_str_in("abc", &bump)?;
    ///
    /// s.try_push('1')?;
    /// s.try_push('2')?;
    /// s.try_push('3')?;
    ///
    /// assert_eq!(s, "abc123");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_push(&mut self, ch: char) -> Result<(), AllocError> {
        self.generic_push(ch)
    }

    #[inline]
    pub(crate) fn generic_push<E: ErrorBehavior>(&mut self, ch: char) -> Result<(), E> {
        let vec = unsafe { self.as_mut_vec() };

        match ch.len_utf8() {
            1 => vec.generic_push(ch as u8),
            _ => vec.generic_extend_from_slice_copy(ch.encode_utf8(&mut [0; 4]).as_bytes()),
        }
    }

    /// Appends a given string slice onto the end of this string.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::from_str_in("foo", &bump);
    ///
    /// s.push_str("bar");
    ///
    /// assert_eq!(s, "foobar");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn push_str(&mut self, string: &str) {
        panic_on_error(self.generic_push_str(string))
    }

    /// Appends a given string slice onto the end of this string.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = BumpString::try_from_str_in("foo", &bump)?;
    ///
    /// s.try_push_str("bar")?;
    ///
    /// assert_eq!(s, "foobar");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_push_str(&mut self, string: &str) -> Result<(), AllocError> {
        self.generic_push_str(string)
    }

    #[inline]
    pub(crate) fn generic_push_str<E: ErrorBehavior>(&mut self, string: &str) -> Result<(), E> {
        let vec = unsafe { self.as_mut_vec() };
        vec.generic_extend_from_slice_copy(string.as_bytes())
    }

    /// Inserts a character into this string at a byte position.
    ///
    /// This is an *O*(*n*) operation as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if `idx` is larger than the string's length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Examples
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
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn insert(&mut self, idx: usize, ch: char) {
        panic_on_error(self.generic_insert(idx, ch))
    }

    /// Inserts a character into this string at a byte position.
    ///
    /// This is an *O*(*n*) operation as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    /// Panics if `idx` is larger than the string's length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = BumpString::try_with_capacity_in(3, &bump)?;
    ///
    /// s.try_insert(0, 'f')?;
    /// s.try_insert(1, 'o')?;
    /// s.try_insert(2, 'o')?;
    ///
    /// assert_eq!("foo", s);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_insert(&mut self, idx: usize, ch: char) -> Result<(), AllocError> {
        self.generic_insert(idx, ch)
    }

    #[inline]
    pub(crate) fn generic_insert<E: ErrorBehavior>(&mut self, idx: usize, ch: char) -> Result<(), E> {
        assert!(self.is_char_boundary(idx));
        let mut bits = [0; 4];
        let bits = ch.encode_utf8(&mut bits).as_bytes();

        unsafe { self.insert_bytes(idx, bits) }
    }

    /// Inserts a string slice into this string at a byte position.
    ///
    /// This is an *O*(*n*) operation as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if `idx` is larger than the string's length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::from_str_in("bar", &bump);
    ///
    /// s.insert_str(0, "foo");
    ///
    /// assert_eq!("foobar", s);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        panic_on_error(self.generic_insert_str(idx, string))
    }

    /// Inserts a string slice into this string at a byte position.
    ///
    /// This is an *O*(*n*) operation as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    /// Panics if `idx` is larger than the string's length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = BumpString::try_from_str_in("bar", &bump)?;
    ///
    /// s.try_insert_str(0, "foo")?;
    ///
    /// assert_eq!("foobar", s);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_insert_str(&mut self, idx: usize, string: &str) -> Result<(), AllocError> {
        self.generic_insert_str(idx, string)
    }

    #[inline]
    pub(crate) fn generic_insert_str<E: ErrorBehavior>(&mut self, idx: usize, string: &str) -> Result<(), E> {
        assert!(self.is_char_boundary(idx));

        unsafe { self.insert_bytes(idx, string.as_bytes()) }
    }

    /// Copies elements from `src` range to the end of the string.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Examples
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
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, src: R) {
        panic_on_error(self.generic_extend_from_within(src))
    }

    /// Copies elements from `src` range to the end of the string.
    ///
    /// # Panics
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
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
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_extend_from_within<R: RangeBounds<usize>>(&mut self, src: R) -> Result<(), AllocError> {
        self.generic_extend_from_within(src)
    }

    #[inline]
    pub(crate) fn generic_extend_from_within<E: ErrorBehavior, R: RangeBounds<usize>>(&mut self, src: R) -> Result<(), E> {
        let src @ Range { start, end } = polyfill::slice::range(src, ..self.len());

        assert!(self.is_char_boundary(start));
        assert!(self.is_char_boundary(end));

        let vec = unsafe { self.as_mut_vec() };
        vec.generic_extend_from_within_copy(src)
    }

    /// Extends this string by pushing `additional` new zero bytes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut string = BumpString::from_str_in("What?", &bump);
    /// string.extend_zeroed(3);
    /// assert_eq!(string, "What?\0\0\0");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_zeroed(&mut self, additional: usize) {
        panic_on_error(self.generic_extend_zeroed(additional))
    }

    /// Extends this string by pushing `additional` new zero bytes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut string = BumpString::try_from_str_in("What?", &bump)?;
    /// string.try_extend_zeroed(3)?;
    /// assert_eq!(string, "What?\0\0\0");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError> {
        self.generic_extend_zeroed(additional)
    }

    #[inline]
    pub(crate) fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
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

    /// Removes the specified range in the string,
    /// and replaces it with the given string.
    /// The given string doesn't need to be the same length as the range.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::from_str_in("α is alpha, β is beta", &bump);
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Replace the range up until the β from the string
    /// s.replace_range(..beta_offset, "Α is capital alpha; ");
    /// assert_eq!(s, "Α is capital alpha; β is beta");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn replace_range<R: RangeBounds<usize>>(&mut self, range: R, replace_with: &str) {
        panic_on_error(self.generic_replace_range(range, replace_with))
    }

    /// Removes the specified range in the string,
    /// and replaces it with the given string.
    /// The given string doesn't need to be the same length as the range.
    ///
    /// # Panics
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = BumpString::try_from_str_in("α is alpha, β is beta", &bump)?;
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Replace the range up until the β from the string
    /// s.try_replace_range(..beta_offset, "Α is capital alpha; ")?;
    /// assert_eq!(s, "Α is capital alpha; β is beta");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_replace_range<R: RangeBounds<usize>>(&mut self, range: R, replace_with: &str) -> Result<(), AllocError> {
        self.generic_replace_range(range, replace_with)
    }

    #[inline]
    pub(crate) fn generic_replace_range<E: ErrorBehavior, R: RangeBounds<usize>>(
        &mut self,
        range: R,
        replace_with: &str,
    ) -> Result<(), E> {
        let Range { start, end } = polyfill::slice::range(range, ..self.len());

        self.assert_char_boundary(start);
        self.assert_char_boundary(end);

        let range_len = end - start;
        let given_len = replace_with.len();

        let additional_len = given_len.saturating_sub(range_len);
        self.generic_reserve(additional_len)?;

        // move the tail
        if range_len != given_len {
            unsafe {
                let src = self.as_ptr().add(end);
                let dst = self.as_mut_ptr().add(start + given_len);
                let len = self.len() - end;
                src.copy_to(dst, len);
            }
        }

        // fill with given string
        unsafe {
            let src = replace_with.as_ptr();
            let dst = self.as_mut_ptr().add(start);
            let len = replace_with.len();
            src.copy_to_nonoverlapping(dst, len);
        }

        // update len
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
        unsafe {
            // Casting to `isize` is fine because per `Layout`'s rules all the `*len`s must be
            // less than isize::MAX. Subtracting two positive `isize`s can't overflow.
            let len_diff = given_len as isize - range_len as isize;
            self.set_len((self.len() as isize + len_diff) as usize);
        }

        Ok(())
    }

    /// Reserves capacity for at least `additional` bytes more than the
    /// current length. The allocator may reserve more space to speculatively
    /// avoid frequent allocations. After calling `reserve`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Examples
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::new_in(&bump);
    ///
    /// s.reserve(10);
    ///
    /// assert!(s.capacity() >= 10);
    /// ```
    ///
    /// This might not actually increase the capacity:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::with_capacity_in(10, &bump);
    /// s.push('a');
    /// s.push('b');
    ///
    /// // s now has a length of 2 and a capacity of at least 10
    /// let capacity = s.capacity();
    /// assert_eq!(2, s.len());
    /// assert!(capacity >= 10);
    ///
    /// // Since we already have at least an extra 8 capacity, calling this...
    /// s.reserve(8);
    ///
    /// // ... doesn't actually increase.
    /// assert_eq!(capacity, s.capacity());
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve(&mut self, additional: usize) {
        panic_on_error(self.generic_reserve(additional))
    }

    /// Reserves capacity for at least `additional` bytes more than the
    /// current length. The allocator may reserve more space to speculatively
    /// avoid frequent allocations. After calling `reserve`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = BumpString::new_in(&bump);
    ///
    /// s.try_reserve(10)?;
    ///
    /// assert!(s.capacity() >= 10);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// This might not actually increase the capacity:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = BumpString::try_with_capacity_in(10, &bump)?;
    /// s.push('a');
    /// s.push('b');
    ///
    /// // s now has a length of 2 and a capacity of at least 10
    /// let capacity = s.capacity();
    /// assert_eq!(2, s.len());
    /// assert!(capacity >= 10);
    ///
    /// // Since we already have at least an extra 8 capacity, calling this...
    /// s.try_reserve(8)?;
    ///
    /// // ... doesn't actually increase.
    /// assert_eq!(capacity, s.capacity());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), AllocError> {
        self.generic_reserve(additional)
    }

    #[inline]
    pub(crate) fn generic_reserve<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        let vec = unsafe { self.as_mut_vec() };
        vec.generic_reserve(additional)
    }

    /// Reserves the minimum capacity for at least `additional` bytes more than
    /// the current length. Unlike [`reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `reserve_exact`, capacity will be greater than or equal to
    /// `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// [`reserve`]: Self::reserve
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Examples
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::new_in(&bump);
    ///
    /// s.reserve_exact(10);
    ///
    /// assert!(s.capacity() >= 10);
    /// ```
    ///
    /// This might not actually increase the capacity:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut s = BumpString::with_capacity_in(10, &bump);
    /// s.push('a');
    /// s.push('b');
    ///
    /// // s now has a length of 2 and a capacity of at least 10
    /// let capacity = s.capacity();
    /// assert_eq!(2, s.len());
    /// assert!(capacity >= 10);
    ///
    /// // Since we already have at least an extra 8 capacity, calling this...
    /// s.reserve_exact(8);
    ///
    /// // ... doesn't actually increase.
    /// assert_eq!(capacity, s.capacity());
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve_exact(&mut self, additional: usize) {
        panic_on_error(self.generic_reserve_exact(additional))
    }

    /// Reserves the minimum capacity for at least `additional` bytes more than
    /// the current length. Unlike [`reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `reserve_exact`, capacity will be greater than or equal to
    /// `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// [`reserve`]: Self::reserve
    ///
    /// # Panics
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = BumpString::new_in(&bump);
    ///
    /// s.try_reserve_exact(10)?;
    ///
    /// assert!(s.capacity() >= 10);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// This might not actually increase the capacity:
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = BumpString::try_with_capacity_in(10, &bump)?;
    /// s.push('a');
    /// s.push('b');
    ///
    /// // s now has a length of 2 and a capacity of at least 10
    /// let capacity = s.capacity();
    /// assert_eq!(2, s.len());
    /// assert!(capacity >= 10);
    ///
    /// // Since we already have at least an extra 8 capacity, calling this...
    /// s.try_reserve_exact(8)?;
    ///
    /// // ... doesn't actually increase.
    /// assert_eq!(capacity, s.capacity());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), AllocError> {
        self.generic_extend_zeroed(additional)
    }

    #[inline]
    pub(crate) fn generic_reserve_exact<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        let vec = unsafe { self.as_mut_vec() };
        vec.generic_reserve_exact(additional)
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

    /// Returns a reference to the allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        &self.allocator
    }

    collection_method_allocator_stats!();

    pub(crate) fn generic_write_fmt<B: ErrorBehavior>(&mut self, args: fmt::Arguments) -> Result<(), B> {
        #[cfg(feature = "panic-on-alloc")]
        if B::PANICS_ON_ALLOC {
            if fmt::Write::write_fmt(PanicsOnAlloc::from_mut(self), args).is_err() {
                // Our `PanicsOnAlloc` wrapped `Write` implementation panics on allocation failure.
                // So this can only be an error returned by a `fmt()` implementor.
                // Note that `fmt()` implementors *should* not return errors (see `std::fmt::Error`)
                return Err(B::format_trait_error());
            }

            return Ok(());
        }

        if fmt::Write::write_fmt(self, args).is_err() {
            // Either an allocation failed or the `fmt()` implementor returned an error.
            // Either way we return an `AllocError`.
            // Note that `fmt()` implementors *should* not return errors (see `std::fmt::Error`).
            // So it's fine not to have an extra error variant for that.
            return Err(B::format_trait_error());
        }

        Ok(())
    }
}

impl<'a, A: BumpAllocatorScope<'a>> BumpString<A> {
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

    /// Converts this `BumpString` into `&CStr` that is live for this bump scope.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop there.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let hello = BumpString::from_str_in("Hello, world!", &bump);
    /// assert_eq!(hello.into_cstr(), c"Hello, world!");
    ///
    /// let abc0def = BumpString::from_str_in("abc\0def", &bump);
    /// assert_eq!(abc0def.into_cstr(), c"abc");
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn into_cstr(self) -> &'a CStr {
        panic_on_error(self.generic_into_cstr())
    }

    /// Converts this `BumpString` into `&CStr` that is live for this bump scope.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop there.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::try_new()?;
    /// let hello = BumpString::try_from_str_in("Hello, world!", &bump)?;    ///
    /// assert_eq!(hello.try_into_cstr()?, c"Hello, world!");
    ///
    /// let abc0def = BumpString::try_from_str_in("abc\0def", &bump)?;
    /// assert_eq!(abc0def.try_into_cstr()?, c"abc");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_into_cstr(self) -> Result<&'a CStr, AllocError> {
        self.generic_into_cstr()
    }

    #[inline]
    pub(crate) fn generic_into_cstr<B: ErrorBehavior>(mut self) -> Result<&'a CStr, B> {
        match self.as_bytes().iter().position(|&c| c == b'\0') {
            Some(nul) => unsafe { self.fixed.cook_mut().as_mut_vec().truncate(nul + 1) },
            None => self.generic_push('\0')?,
        }

        let bytes_with_nul = self.into_boxed_str().into_ref().as_bytes();
        Ok(unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) })
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
    pub fn from_parts(string: FixedBumpString<'a>, allocator: A) -> Self {
        Self {
            fixed: unsafe { RawFixedBumpString::from_cooked(string) },
            allocator,
        }
    }

    /// Turns this `BumpString` into its parts.
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpString };
    /// # let bump: Bump = Bump::new();
    /// let mut string = BumpString::new_in(&bump);
    /// string.reserve(10);
    /// string.push('a');
    /// let mut fixed_string = string.into_parts().0;
    /// assert_eq!(fixed_string.capacity(), 10);
    /// assert_eq!(fixed_string, "a");
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn into_parts(self) -> (FixedBumpString<'a>, A) {
        destructure!(let Self { fixed, allocator } = self);
        (unsafe { fixed.cook() }, allocator)
    }
}

impl<A: BumpAllocator> fmt::Write for BumpString<A> {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.try_push_str(s).map_err(|_| fmt::Error)
    }

    #[inline(always)]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.try_push(c).map_err(|_| fmt::Error)
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<A: BumpAllocator> fmt::Write for PanicsOnAlloc<BumpString<A>> {
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

impl<A: BumpAllocator> Debug for BumpString<A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl<A: BumpAllocator> Display for BumpString<A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl<A: BumpAllocator> Deref for BumpString<A> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<A: BumpAllocator> DerefMut for BumpString<A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

impl<A: BumpAllocator> Drop for BumpString<A> {
    fn drop(&mut self) {
        // SAFETY:
        // The dangling pointer can not be a valid ptr into a chunk; because
        // of the minimum chunk alignment of 16 the smallest address the chunk
        // may be at is 16. The bump allocator handles deallocate requests
        // from pointers outside its bound just fine by ignoring them.
        unsafe {
            let ptr = self.fixed.as_non_null_ptr().cast();
            let layout = Layout::from_size_align_unchecked(self.fixed.capacity(), 1);
            self.allocator.deallocate(ptr, layout);
        }
    }
}

impl<A: BumpAllocator + Default> Default for BumpString<A> {
    fn default() -> Self {
        Self::new_in(A::default())
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<A: BumpAllocator + Clone> Clone for BumpString<A> {
    fn clone(&self) -> Self {
        let len = self.len();
        let allocator = self.allocator.clone();
        let ptr = allocator.allocate_slice::<u8>(len);

        unsafe {
            self.as_ptr().copy_to_nonoverlapping(ptr.as_ptr(), len);

            let slice = nonnull::str_from_utf8(nonnull::slice_from_raw_parts(ptr, len));
            let fixed = RawFixedBumpString::from_raw_parts(slice, len);

            Self { fixed, allocator }
        }
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<A: BumpAllocator> core::ops::AddAssign<&str> for BumpString<A> {
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}

impl<A: BumpAllocator> AsRef<str> for BumpString<A> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<A: BumpAllocator> AsMut<str> for BumpString<A> {
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<A: BumpAllocator> Borrow<str> for BumpString<A> {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<A: BumpAllocator> BorrowMut<str> for BumpString<A> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<A: BumpAllocator> Eq for BumpString<A> {}

impl<A: BumpAllocator> PartialOrd for BumpString<A> {
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

impl<A: BumpAllocator> Ord for BumpString<A> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        <str as Ord>::cmp(self, other)
    }
}

impl<A: BumpAllocator> Hash for BumpString<A> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<'s, A: BumpAllocator> Extend<&'s str> for BumpString<A> {
    #[inline]
    fn extend<T: IntoIterator<Item = &'s str>>(&mut self, iter: T) {
        for str in iter {
            self.push_str(str);
        }
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<A: BumpAllocator> Extend<char> for BumpString<A> {
    fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
        let iterator = iter.into_iter();
        let (lower_bound, _) = iterator.size_hint();
        self.reserve(lower_bound);
        iterator.for_each(move |c| self.push(c));
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<'s, A: BumpAllocator> Extend<&'s char> for BumpString<A> {
    fn extend<I: IntoIterator<Item = &'s char>>(&mut self, iter: I) {
        self.extend(iter.into_iter().copied());
    }
}

#[cfg(feature = "alloc")]
impl<A: BumpAllocator> From<BumpString<A>> for alloc_crate::string::String {
    #[inline]
    fn from(value: BumpString<A>) -> Self {
        value.as_str().into()
    }
}

/// Implements the `+` operator for concatenating two strings.
///
/// This consumes the `BumpString` on the left-hand side and re-uses its buffer (growing it if
/// necessary). This is done to avoid allocating a new `BumpString` and copying the entire contents on
/// every operation, which would lead to *O*(*n*^2) running time when building an *n*-byte string by
/// repeated concatenation.
///
/// The string on the right-hand side is only borrowed; its contents are copied into the returned
/// `BumpString`.
///
/// # Examples
///
/// Concatenating two `BumpString`s takes the first by value and borrows the second:
///
/// ```
/// # use bump_scope::{ Bump, BumpString };
/// # let bump: Bump = Bump::new();
/// let a = BumpString::from_str_in("hello", &bump);
/// let b = BumpString::from_str_in(" world", &bump);
/// let c = a + &b;
/// // `a` is moved and can no longer be used here.
/// ```
///
/// If you want to keep using the first `BumpString`, you can clone it and append to the clone instead:
///
/// ```
/// # use bump_scope::{ Bump, BumpString };
/// # let bump: Bump = Bump::new();
/// let a = BumpString::from_str_in("hello", &bump);
/// let b = BumpString::from_str_in(" world", &bump);
/// let c = a.clone() + &b;
/// // `a` is still valid here.
/// ```
///
/// Concatenating `&str` slices can be done by converting the first to a `BumpString`:
///
/// ```
/// # use bump_scope::{ Bump, BumpString };
/// # let bump: Bump = Bump::new();
/// let a = "hello";
/// let b = " world";
/// let c = BumpString::from_str_in(a, &bump) + b;
/// ```
#[cfg(feature = "panic-on-alloc")]
impl<A: BumpAllocator> core::ops::Add<&str> for BumpString<A> {
    type Output = Self;

    #[inline]
    fn add(mut self, other: &str) -> Self {
        self.push_str(other);
        self
    }
}
