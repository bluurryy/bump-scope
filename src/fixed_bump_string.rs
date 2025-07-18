use core::{
    borrow::{Borrow, BorrowMut},
    fmt::{self, Debug, Display},
    hash::Hash,
    mem::{self, MaybeUninit},
    ops::{self, Deref, DerefMut, Range, RangeBounds},
    ptr::{self, NonNull},
    str,
};

use crate::{
    alloc::AllocError,
    owned_str,
    polyfill::{self, non_null, transmute_mut},
    BumpAllocatorScope, BumpBox, BumpString, ErrorBehavior, FixedBumpVec, FromUtf8Error, NoDrop,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// A type like [`BumpString`](crate::BumpString) but with a fixed capacity.
///
/// It can be constructed with [`alloc_fixed_string`] or from a `BumpBox` via [`from_init`] or [`from_uninit`].
///
/// # Examples
/// ```
/// # use bump_scope::{Bump, FixedBumpString};
/// # let mut bump: Bump = Bump::new();
/// let mut string = FixedBumpString::with_capacity_in(9, &bump);
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
    initialized: BumpBox<'a, str>,
    capacity: usize,
}

unsafe impl Send for FixedBumpString<'_> {}
unsafe impl Sync for FixedBumpString<'_> {}

impl<'a> FixedBumpString<'a> {
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpString::new()` instead"]
    /// Empty fixed string.
    pub const EMPTY: Self = Self {
        initialized: BumpBox::EMPTY_STR,
        capacity: 0,
    };

    /// Constructs a new empty `FixedBumpString`.
    ///
    /// This will not allocate.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::FixedBumpString;
    /// let string = FixedBumpString::new();
    /// assert_eq!(string.len(), 0);
    /// assert_eq!(string.capacity(), 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            initialized: BumpBox::EMPTY_STR,
            capacity: 0,
        }
    }

    /// Constructs a new empty `FixedBumpString` with the specified capacity
    /// in the provided bump allocator.
    ///
    /// The string will be able to hold `capacity` bytes.
    /// If `capacity` is 0, the string will not allocate.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(10, &bump);
    ///
    /// // The String contains no chars, even though it has capacity for more
    /// assert_eq!(s.len(), 0);
    ///
    /// // The string has capacity for 10 bytes...
    /// for _ in 0..10 {
    ///     s.push('a');
    /// }
    ///
    /// // ...but another byte will not fit
    /// assert!(s.try_push('a').is_err());
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_capacity_in(capacity: usize, allocator: impl BumpAllocatorScope<'a>) -> Self {
        panic_on_error(Self::generic_with_capacity_in(capacity, allocator))
    }

    /// Constructs a new empty `FixedBumpString` with the specified capacity
    /// in the provided bump allocator.
    ///
    /// The string will be able to hold `capacity` bytes.
    /// If `capacity` is 0, the string will not allocate.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::try_with_capacity_in(10, &bump)?;
    ///
    /// // The String contains no chars, even though it has capacity for more
    /// assert_eq!(s.len(), 0);
    ///
    /// // The string has capacity for 10 bytes...
    /// for _ in 0..10 {
    ///     s.push('a');
    /// }
    ///
    /// // ...but another byte will not fit
    /// assert!(s.try_push('a').is_err());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_capacity_in(capacity: usize, allocator: impl BumpAllocatorScope<'a>) -> Result<Self, AllocError> {
        Self::generic_with_capacity_in(capacity, allocator)
    }

    #[inline]
    pub(crate) fn generic_with_capacity_in<E: ErrorBehavior>(
        capacity: usize,
        allocator: impl BumpAllocatorScope<'a>,
    ) -> Result<Self, E> {
        Ok(BumpString::generic_with_capacity_in(capacity, allocator)?.into_fixed_string())
    }

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

        let ptr = non_null::as_non_null_ptr(uninitialized).cast::<u8>();
        let initialized = unsafe { BumpBox::from_raw(non_null::str_from_utf8(non_null::slice_from_raw_parts(ptr, 0))) };

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
    /// # use bump_scope::{Bump, FixedBumpVec, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// // some bytes, in a vector
    /// let mut sparkle_heart = FixedBumpVec::with_capacity_in(4, &bump);
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
    /// # use bump_scope::{Bump, FixedBumpVec, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// // some invalid bytes, in a vector
    /// let mut sparkle_heart = FixedBumpVec::with_capacity_in(4, &bump);
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
            Err(error) => Err(FromUtf8Error::new(error, vec)),
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
    /// # use bump_scope::{Bump, FixedBumpVec, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    ///
    /// // some bytes, in a vector
    /// let mut sparkle_heart = FixedBumpVec::with_capacity_in(4, &bump);
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

    /// Turns this `FixedBumpString` into a `BumpString`.
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
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(5, &bump);
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

    #[track_caller]
    #[inline(always)]
    pub(crate) fn assert_char_boundary(&self, index: usize) {
        self.initialized.assert_char_boundary(index);
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
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// # let mut string = FixedBumpString::with_capacity_in(5, &bump);
    /// # string.push_str("abcde");
    /// # let start = 1;
    /// # let end = 4;
    /// let mut other = FixedBumpString::with_capacity_in(end - start, &bump);
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
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut string = FixedBumpString::with_capacity_in(20, &bump);
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
    pub fn split_off(&mut self, range: impl RangeBounds<usize>) -> Self {
        let len = self.len();
        let ops::Range { start, end } = polyfill::slice::range(range, ..len);
        let ptr = non_null::as_non_null_ptr(non_null::str_bytes(self.initialized.ptr()));

        unsafe {
            if end == len {
                self.assert_char_boundary(start);

                let lhs = ptr;
                let rhs = non_null::add(ptr, start);

                let lhs_len = start;
                let rhs_len = len - start;

                let lhs_cap = start;
                let rhs_cap = self.capacity - lhs_cap;

                self.set_ptr(lhs);
                self.set_len(lhs_len);
                self.set_cap(lhs_cap);

                return FixedBumpString {
                    initialized: BumpBox::from_raw(non_null::str_from_utf8(non_null::slice_from_raw_parts(rhs, rhs_len))),
                    capacity: rhs_cap,
                };
            }

            if start == 0 {
                self.assert_char_boundary(end);

                let lhs = ptr;
                let rhs = non_null::add(ptr, end);

                let lhs_len = end;
                let rhs_len = len - end;

                let lhs_cap = end;
                let rhs_cap = self.capacity - lhs_cap;

                self.set_ptr(rhs);
                self.set_len(rhs_len);
                self.set_cap(rhs_cap);

                return FixedBumpString {
                    initialized: BumpBox::from_raw(non_null::str_from_utf8(non_null::slice_from_raw_parts(lhs, lhs_len))),
                    capacity: lhs_cap,
                };
            }

            if start == end {
                return FixedBumpString::new();
            }

            self.assert_char_boundary(start);
            self.assert_char_boundary(end);

            let head_len = start;
            let tail_len = len - end;

            let range_len = end - start;
            let remaining_len = len - range_len;

            if head_len < tail_len {
                // move the range of elements to split off to the start
                self.as_mut_vec().get_unchecked_mut(..end).rotate_right(range_len);

                let lhs = ptr;
                let rhs = non_null::add(ptr, range_len);

                let lhs_len = range_len;
                let rhs_len = remaining_len;

                let lhs_cap = range_len;
                let rhs_cap = self.capacity - lhs_cap;

                self.set_ptr(rhs);
                self.set_len(rhs_len);
                self.set_cap(rhs_cap);

                FixedBumpString {
                    initialized: BumpBox::from_raw(non_null::str_from_utf8(non_null::slice_from_raw_parts(lhs, lhs_len))),
                    capacity: lhs_cap,
                }
            } else {
                // move the range of elements to split off to the end
                self.as_mut_vec().get_unchecked_mut(start..).rotate_left(range_len);

                let lhs = ptr;
                let rhs = non_null::add(ptr, remaining_len);

                let lhs_len = remaining_len;
                let rhs_len = range_len;

                let lhs_cap = remaining_len;
                let rhs_cap = self.capacity - lhs_cap;

                self.set_ptr(lhs);
                self.set_len(lhs_len);
                self.set_cap(lhs_cap);

                FixedBumpString {
                    initialized: BumpBox::from_raw(non_null::str_from_utf8(non_null::slice_from_raw_parts(rhs, rhs_len))),
                    capacity: rhs_cap,
                }
            }
        }
    }

    /// Removes the last character from the string buffer and returns it.
    ///
    /// Returns [`None`] if this string is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(4, &bump);
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
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut s = FixedBumpString::with_capacity_in(3, &bump);
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
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(5, &bump);
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
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(4, &bump);
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
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(50, &bump);
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
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(50, &bump);
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
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(50, &bump);
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

    /// Returns a `NonNull` pointer to the string's buffer, or a dangling
    /// `NonNull` pointer valid for zero sized reads if the string didn't allocate.
    ///
    /// The caller must ensure that the string outlives the pointer this
    /// function returns, or else it will end up dangling.
    /// Modifying the string may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    ///
    /// This method guarantees that for the purpose of the aliasing model, this method
    /// does not materialize a reference to the underlying slice, and thus the returned pointer
    /// will remain valid when mixed with other calls to [`as_ptr`], [`as_mut_ptr`],
    /// and [`as_non_null`].
    /// Note that calling other methods that materialize references to the slice,
    /// or references to specific elements you are planning on accessing through this pointer,
    /// may still invalidate this pointer.
    /// See the second example below for how this guarantee can be used.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// // Allocate vector big enough for 4 elements.
    /// let size = 4;
    /// let mut x = FixedBumpString::with_capacity_in(size, &bump);
    /// let x_ptr = x.as_non_null();
    ///
    /// // Initialize elements via raw pointer writes, then set length.
    /// unsafe {
    ///     for i in 0..size {
    ///         x_ptr.add(i).write(i as u8 + b'a');
    ///     }
    ///     x.as_mut_vec().set_len(size);
    /// }
    /// assert_eq!(&*x, "abcd");
    /// ```
    ///
    /// Due to the aliasing guarantee, the following code is legal:
    ///
    /// ```
    /// # use bump_scope::{Bump, bump_format};
    /// # let bump: Bump = Bump::new();
    /// unsafe {
    ///     let v = bump_format!(in &bump, "a").into_fixed_string();
    ///     let ptr1 = v.as_non_null();
    ///     ptr1.write(b'b');
    ///     let ptr2 = v.as_non_null();
    ///     ptr2.write(b'c');
    ///     // Notably, the write to `ptr2` did *not* invalidate `ptr1`:
    ///     ptr1.write(b'd');
    /// }
    /// ```
    ///
    /// [`as_mut_ptr`]: Self::as_mut_ptr
    /// [`as_ptr`]: Self::as_ptr
    /// [`as_non_null`]: Self::as_non_null
    #[must_use]
    #[inline(always)]
    pub const fn as_non_null(&self) -> NonNull<u8> {
        self.initialized.as_non_null()
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
        // SAFETY: `FixedBumpVec<u8>` and `FixedBumpString` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        transmute_mut(self)
    }

    /// Returns a raw pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const u8 {
        self.initialized.as_ptr()
    }

    /// Returns an unsafe mutable pointer to slice, or a dangling
    /// raw pointer valid for zero sized reads.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.initialized.as_mut_ptr()
    }

    #[inline(always)]
    pub(crate) unsafe fn set_ptr(&mut self, new_ptr: NonNull<u8>) {
        self.initialized.set_ptr(new_ptr);
    }

    #[inline(always)]
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        self.initialized.set_len(new_len);
    }

    #[inline(always)]
    pub(crate) unsafe fn set_cap(&mut self, new_cap: usize) {
        self.capacity = new_cap;
    }
}

impl FixedBumpString<'_> {
    /// Appends the given [`char`] to the end of this string.
    ///
    /// # Panics
    /// Panics if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(3, &bump);
    ///
    /// s.push('a');
    /// s.push('b');
    /// s.push('c');
    ///
    /// assert_eq!(s, "abc");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn push(&mut self, ch: char) {
        panic_on_error(self.generic_push(ch));
    }

    /// Appends the given [`char`] to the end of this string.
    ///
    /// # Errors
    /// Errors if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = FixedBumpString::try_with_capacity_in(3, &bump)?;
    ///
    /// s.try_push('a')?;
    /// s.try_push('b')?;
    /// s.try_push('c')?;
    ///
    /// assert_eq!(s, "abc");
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
    /// Panics if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(6, &bump);
    ///
    /// s.push_str("foo");
    /// s.push_str("bar");
    ///
    /// assert_eq!(s, "foobar");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn push_str(&mut self, string: &str) {
        panic_on_error(self.generic_push_str(string));
    }

    /// Appends a given string slice onto the end of this string.
    ///
    /// # Errors
    /// Errors if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = FixedBumpString::try_with_capacity_in(6, &bump)?;
    ///
    /// s.try_push_str("foo")?;
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
    /// Panics if the string does not have enough capacity.
    ///
    /// Panics if `idx` is larger than the string's length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(3, &bump);
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
        panic_on_error(self.generic_insert(idx, ch));
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
    /// Errors if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = FixedBumpString::try_with_capacity_in(3, &bump)?;
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
    /// Panics if the string does not have enough capacity.
    ///
    /// Panics if `idx` is larger than the string's length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(6, &bump);
    /// s.push_str("bar");
    ///
    /// s.insert_str(0, "foo");
    ///
    /// assert_eq!("foobar", s);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        panic_on_error(self.generic_insert_str(idx, string));
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
    /// Errors if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = FixedBumpString::try_with_capacity_in(6, &bump)?;
    /// s.try_push_str("bar")?;
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
    /// Panics if the string does not have enough capacity.
    ///
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut string = FixedBumpString::with_capacity_in(14, &bump);
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
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_from_within<R>(&mut self, src: R)
    where
        R: RangeBounds<usize>,
    {
        panic_on_error(self.generic_extend_from_within(src));
    }

    /// Copies elements from `src` range to the end of the string.
    ///
    /// # Panics
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Errors
    /// Errors if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut string = FixedBumpString::try_with_capacity_in(14, &bump)?;
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
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_extend_from_within<R>(&mut self, src: R) -> Result<(), AllocError>
    where
        R: RangeBounds<usize>,
    {
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
    /// Panics if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut string = FixedBumpString::with_capacity_in(8, &bump);
    /// string.push_str("What?");
    /// string.extend_zeroed(3);
    /// assert_eq!(string, "What?\0\0\0");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_zeroed(&mut self, additional: usize) {
        panic_on_error(self.generic_extend_zeroed(additional));
    }

    /// Extends this string by pushing `additional` new zero bytes.
    ///
    /// # Errors
    /// Errors if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut string = FixedBumpString::try_with_capacity_in(8, &bump)?;
    /// string.try_push_str("What?")?;
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
    /// Panics if the string does not have enough capacity.
    ///
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::new();
    /// let mut s = FixedBumpString::with_capacity_in(50, &bump);
    /// s.push_str("α is alpha, β is beta");
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Replace the range up until the β from the string
    /// s.replace_range(..beta_offset, "Α is capital alpha; ");
    /// assert_eq!(s, "Α is capital alpha; β is beta");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: RangeBounds<usize>,
    {
        panic_on_error(self.generic_replace_range(range, replace_with));
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
    /// Errors if the string does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpString};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut s = FixedBumpString::try_with_capacity_in(50, &bump)?;
    /// s.push_str("α is alpha, β is beta");
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Replace the range up until the β from the string
    /// s.try_replace_range(..beta_offset, "Α is capital alpha; ")?;
    /// assert_eq!(s, "Α is capital alpha; β is beta");
    ///
    /// // An error will be returned when the capacity does not suffice
    /// let mut s = FixedBumpString::try_with_capacity_in(5, &bump)?;
    /// s.push_str("hello");
    /// assert!(s.try_replace_range(4..=4, " n").is_err());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_replace_range<R>(&mut self, range: R, replace_with: &str) -> Result<(), AllocError>
    where
        R: RangeBounds<usize>,
    {
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

    /// Checks if at least `additional` more bytes can be inserted
    /// in the given `FixedBumpString` due to capacity.
    ///
    /// # Panics
    /// Panics if the string does not have enough capacity.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve(&mut self, additional: usize) {
        panic_on_error(self.generic_reserve(additional));
    }

    /// Checks if at least `additional` more bytes can be inserted
    /// in the given `FixedBumpString` due to capacity.
    ///
    /// # Errors
    /// Errors if the string does not have enough capacity.
    #[inline(always)]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), AllocError> {
        self.generic_reserve(additional)
    }

    #[inline]
    pub(crate) fn generic_reserve<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        let vec = unsafe { self.as_mut_vec() };
        vec.generic_reserve(additional)
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
        Self::new()
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
    alloc_crate::string::String,

    #[cfg(feature = "alloc")]
    alloc_crate::borrow::Cow<'_, str>,
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
impl<'a> From<FixedBumpString<'a>> for alloc_crate::string::String {
    #[inline]
    fn from(value: FixedBumpString<'a>) -> Self {
        value.as_str().into()
    }
}

impl NoDrop for FixedBumpString<'_> {}
