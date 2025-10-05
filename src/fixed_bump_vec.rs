use core::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    hash::Hash,
    iter,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{self, Deref, DerefMut, Index, IndexMut, RangeBounds},
    ptr::{self, NonNull},
    slice::SliceIndex,
};

use crate::{
    BumpAllocatorScopeExt, BumpBox, BumpVec, ErrorBehavior, NoDrop, SizedTypeProperties,
    alloc::AllocError,
    owned_slice::{self, OwnedSlice, TakeOwnedSlice},
    polyfill::{self, hint::likely, non_null, pointer, slice},
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// A type like [`BumpVec`] but with a fixed capacity.
///
/// It can be constructed using [`with_capacity_in`] or from a `BumpBox` via [`from_init`] or [`from_uninit`].
///
/// ## Using it like `BumpVec`
///
/// This type is also useful when you want a growing `BumpVec` but don't want to carry around a reference to
/// the `Bump(Scope)`. You can use it this way by first converting it to a `BumpVec` using [`BumpVec::from_parts`],
/// making your changes and then turning it back into a `FixedBumpVec` with [`BumpVec::into_fixed_vec`].
///
/// Not storing the `Bump(Scope)` allows you to call bump allocator methods that require `&mut`,
/// like [`scoped`](crate::Bump::scoped).
///
/// # Examples
/// ```
/// # use bump_scope::{Bump, FixedBumpVec};
/// # let bump: Bump = Bump::new();
/// let mut vec = FixedBumpVec::with_capacity_in(3, &bump);
///
/// vec.push(1);
/// vec.push(2);
/// vec.push(3);
///
/// let slice: &[i32] = vec.into_slice();
///
/// assert_eq!(slice, [1, 2, 3]);
/// ```
///
/// Growing it via `BumpVec`:
///
/// ```
/// # use bump_scope::{Bump, BumpScope, BumpVec, FixedBumpVec};
/// # type T = i32;
/// struct MyBuilder<'a, 'b> {
///     bump: &'b mut BumpScope<'a>,
///     vec: FixedBumpVec<'a, T>,
/// }
///
/// impl<'a, 'b> MyBuilder<'a, 'b> {
///     fn push(&mut self, value: T) {
///         let fixed_vec = core::mem::take(&mut self.vec);
///         let mut vec = BumpVec::from_parts(fixed_vec, &mut *self.bump);
///         vec.push(value);
///         self.vec = vec.into_fixed_vec();
///     }
/// }
///
/// let mut bump: Bump = Bump::new();
///
/// let mut builder = MyBuilder {
///     bump: bump.as_mut_scope(),
///     vec: FixedBumpVec::new(),
/// };
///
/// builder.push(1);
/// assert_eq!(builder.vec, [1]);
/// ```
///
/// [`with_capacity_in`]: Self::with_capacity_in
/// [`from_uninit`]: Self::from_uninit
/// [`from_init`]: Self::from_init
// `FixedBumpString` and `FixedBumpVec<u8>` have the same repr.
#[repr(C)]
pub struct FixedBumpVec<'a, T> {
    initialized: BumpBox<'a, [T]>,
    capacity: usize,
}

unsafe impl<T: Send> Send for FixedBumpVec<'_, T> {}
unsafe impl<T: Sync> Sync for FixedBumpVec<'_, T> {}

impl<'a, T> FixedBumpVec<'a, T> {
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpVec::new()` instead"]
    /// Empty fixed vector.
    pub const EMPTY: Self = Self {
        initialized: BumpBox::EMPTY,
        capacity: if T::IS_ZST { usize::MAX } else { 0 },
    };

    /// Constructs a new empty `FixedBumpVec<T>`.
    ///
    /// This will not allocate.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::FixedBumpVec;
    /// let vec = FixedBumpVec::<i32>::new();
    /// assert_eq!(vec.len(), 0);
    /// assert_eq!(vec.capacity(), 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            initialized: BumpBox::EMPTY,
            capacity: if T::IS_ZST { usize::MAX } else { 0 },
        }
    }

    /// Constructs a new empty vector with the specified capacity
    /// in the provided bump allocator.
    ///
    /// The vector will be able to hold `capacity` elements.
    /// If `capacity` is 0, the vector will not allocate.
    ///
    /// When `T` is a zero-sized type, there will be no allocation
    /// and the capacity will always be `usize::MAX`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::<i32>::with_capacity_in(10, &bump);
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    /// assert!(vec.capacity() == 10);
    ///
    /// // The vector has space for 10 items...
    /// for i in 0..10 {
    ///     vec.push(i);
    /// }
    ///
    /// assert_eq!(vec.len(), 10);
    /// assert!(vec.capacity() == 10);
    ///
    /// // ...but one more will not fit
    /// assert!(vec.try_push(11).is_err());
    ///
    /// // A vector of a zero-sized type will always over-allocate, since no
    /// // allocation is necessary
    /// let vec_units = FixedBumpVec::<()>::with_capacity_in(10, &bump);
    /// assert_eq!(vec_units.capacity(), usize::MAX);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_capacity_in(capacity: usize, allocator: impl BumpAllocatorScopeExt<'a>) -> Self {
        panic_on_error(Self::generic_with_capacity_in(capacity, allocator))
    }

    /// Constructs a new empty vector with the specified capacity
    /// in the provided bump allocator.
    ///
    /// The vector will be able to hold `capacity` elements.
    /// If `capacity` is 0, the vector will not allocate.
    ///
    /// When `T` is a zero-sized type, there will be no allocation
    /// and the capacity will always be `usize::MAX`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::<i32>::try_with_capacity_in(10, &bump)?;
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    /// assert!(vec.capacity() == 10);
    ///
    /// // The vector has space for 10 items...
    /// for i in 0..10 {
    ///     vec.push(i);
    /// }
    /// assert_eq!(vec.len(), 10);
    /// assert!(vec.capacity() == 10);
    ///
    /// // ...but one more will not fit
    /// assert!(vec.try_push(11).is_err());
    ///
    /// // A vector of a zero-sized type will always over-allocate, since no
    /// // allocation is necessary
    /// let vec_units = FixedBumpVec::<()>::try_with_capacity_in(10, &bump)?;
    /// assert_eq!(vec_units.capacity(), usize::MAX);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_capacity_in(capacity: usize, allocator: impl BumpAllocatorScopeExt<'a>) -> Result<Self, AllocError> {
        Self::generic_with_capacity_in(capacity, allocator)
    }

    #[inline]
    pub(crate) fn generic_with_capacity_in<E: ErrorBehavior>(
        capacity: usize,
        allocator: impl BumpAllocatorScopeExt<'a>,
    ) -> Result<Self, E> {
        Ok(BumpVec::generic_with_capacity_in(capacity, allocator)?.into_fixed_vec())
    }

    /// Create a new [`FixedBumpVec`] whose elements are taken from an iterator and allocated in the given `bump`.
    ///
    /// This is behaviorally identical to [`FromIterator::from_iter`].
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`from_iter_exact_in`](Self::from_iter_exact_in) instead for better performance.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec = FixedBumpVec::from_iter_in([1, 2, 3], &bump);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_iter_in<I, A>(iter: I, allocator: A) -> Self
    where
        I: IntoIterator<Item = T>,
        A: BumpAllocatorScopeExt<'a>,
    {
        panic_on_error(Self::generic_from_iter_in(iter, allocator))
    }

    /// Create a new [`FixedBumpVec`] whose elements are taken from an iterator and allocated in the given `bump`.
    ///
    /// This is behaviorally identical to [`FromIterator::from_iter`].
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`from_iter_exact_in`](Self::from_iter_exact_in) instead for better performance.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let vec = FixedBumpVec::try_from_iter_in([1, 2, 3], &bump)?;
    /// assert_eq!(vec, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_iter_in<I, A>(iter: I, allocator: A) -> Result<Self, AllocError>
    where
        I: IntoIterator<Item = T>,
        A: BumpAllocatorScopeExt<'a>,
    {
        Self::generic_from_iter_in(iter, allocator)
    }

    #[inline]
    pub(crate) fn generic_from_iter_in<E: ErrorBehavior, I, A>(iter: I, allocator: A) -> Result<Self, E>
    where
        I: IntoIterator<Item = T>,
        A: BumpAllocatorScopeExt<'a>,
    {
        Ok(BumpVec::generic_from_iter_in(iter, allocator)?.into_fixed_vec())
    }

    /// Create a new [`FixedBumpVec`] whose elements are taken from an iterator and allocated in the given `bump`.
    ///
    /// This is just like [`from_iter_in`](Self::from_iter_in) but optimized for an [`ExactSizeIterator`].
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec = FixedBumpVec::from_iter_exact_in([1, 2, 3], &bump);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_iter_exact_in<I, A>(iter: I, allocator: A) -> Self
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
        A: BumpAllocatorScopeExt<'a>,
    {
        panic_on_error(Self::generic_from_iter_exact_in(iter, allocator))
    }

    /// Create a new [`FixedBumpVec`] whose elements are taken from an iterator and allocated in the given `bump`.
    ///
    /// This is just like [`from_iter_in`](Self::from_iter_in) but optimized for an [`ExactSizeIterator`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let vec = FixedBumpVec::try_from_iter_exact_in([1, 2, 3], &bump)?;
    /// assert_eq!(vec, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_iter_exact_in<I, A>(iter: I, allocator: A) -> Result<Self, AllocError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
        A: BumpAllocatorScopeExt<'a>,
    {
        Self::generic_from_iter_exact_in(iter, allocator)
    }

    #[inline]
    pub(crate) fn generic_from_iter_exact_in<E: ErrorBehavior, I, A>(iter: I, allocator: A) -> Result<Self, E>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
        A: BumpAllocatorScopeExt<'a>,
    {
        Ok(BumpVec::generic_from_iter_exact_in(iter, allocator)?.into_fixed_vec())
    }

    /// Turns a `BumpBox<[T]>` into a full `FixedBumpVec<T>`.
    #[must_use]
    pub fn from_init(initialized: BumpBox<'a, [T]>) -> Self {
        let capacity = if T::IS_ZST { usize::MAX } else { initialized.len() };
        Self { initialized, capacity }
    }

    /// Turns a `BumpBox<[MaybeUninit<T>]>` into a `FixedBumpVec<T>` with a length of `0`.
    #[must_use]
    pub fn from_uninit(uninitialized: BumpBox<'a, [MaybeUninit<T>]>) -> Self {
        let uninitialized = uninitialized.into_raw();
        let capacity = if T::IS_ZST { usize::MAX } else { uninitialized.len() };

        let ptr = non_null::as_non_null_ptr(uninitialized).cast::<T>();
        let initialized = unsafe { BumpBox::from_raw(NonNull::slice_from_raw_parts(ptr, 0)) };

        Self { initialized, capacity }
    }

    /// Returns the total number of elements the vector can hold.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec = FixedBumpVec::<i32>::with_capacity_in(2048, &bump);
    /// assert_eq!(vec.capacity(), 2048);
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of elements in the vector, also referred to
    /// as its 'length'.
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.initialized.len()
    }

    /// Returns `true` if the vector contains no elements.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.initialized.is_empty()
    }

    /// Returns `true` if the vector has reached its capacity.
    #[must_use]
    #[inline(always)]
    pub const fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// Turns this `FixedBumpVec<T>` into a `&[T]` that is live for this bump scope.
    ///
    /// This is only available for [`NoDrop`] types so you don't omit dropping a value for which it matters.
    ///
    /// `!NoDrop` types can still be turned into slices via <code>BumpBox::[leak](BumpBox::leak)(vec.[into_boxed_slice](Self::into_boxed_slice)())</code>.
    #[must_use]
    #[inline(always)]
    pub fn into_slice(self) -> &'a mut [T]
    where
        [T]: NoDrop,
    {
        self.into_boxed_slice().into_mut()
    }

    /// Turns this `FixedBumpVec<T>` into a `BumpBox<[T]>`.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_slice(self) -> BumpBox<'a, [T]> {
        self.initialized
    }

    /// Turns this `FixedBumpVec<T>` into a `BumpVec<T>`.
    #[must_use]
    #[inline(always)]
    pub fn into_vec<A: BumpAllocatorScopeExt<'a>>(self, allocator: A) -> BumpVec<T, A> {
        BumpVec::from_parts(self, allocator)
    }

    /// Removes the last element from a vector and returns it, or [`None`] if it
    /// is empty.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(3, &bump);
    /// vec.append([1, 2, 3]);
    /// assert_eq!(vec.pop(), Some(3));
    /// assert_eq!(vec, [1, 2]);
    /// ```
    ///
    /// # Time complexity
    /// Takes *O*(1) time.
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        self.initialized.pop()
    }

    /// Removes and returns the last element from a vector if the predicate
    /// returns `true`, or [`None`] if the predicate returns false or the vector
    /// is empty (the predicate will not be called in that case).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::<i32>::with_capacity_in(4, &bump);
    /// vec.append([1, 2, 3, 4]);
    /// let pred = |x: &mut i32| *x % 2 == 0;
    ///
    /// assert_eq!(vec.pop_if(pred), Some(4));
    /// assert_eq!(vec, [1, 2, 3]);
    /// assert_eq!(vec.pop_if(pred), None);
    /// ```
    pub fn pop_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        let last = self.last_mut()?;
        if predicate(last) { self.pop() } else { None }
    }

    /// Clears the vector, removing all values.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.clear();
    /// assert!(vec.is_empty());
    /// ```
    #[inline(always)]
    pub fn clear(&mut self) {
        self.initialized.clear();
    }

    /// Shortens the vector, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// If `len` is greater than the vector's current length, this has no
    /// effect.
    ///
    /// The [`drain`] method can emulate `truncate`, but causes the excess
    /// elements to be returned instead of dropped.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the vector.
    ///
    /// # Examples
    ///
    /// Truncating a five element vector to two elements:
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3, 4, 5]);
    /// vec.truncate(2);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current
    /// length:
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.truncate(8);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.truncate(0);
    /// assert_eq!(vec, []);
    /// ```
    ///
    /// [`clear`]: Self::clear
    /// [`drain`]: Self::drain
    pub fn truncate(&mut self, len: usize) {
        self.initialized.truncate(len);
    }

    /// Removes and returns the element at position `index` within the vector,
    /// shifting all elements after it to the left.
    ///
    /// Note: Because this shifts over the remaining elements, it has a
    /// worst-case performance of *O*(*n*). If you don't need the order of elements
    /// to be preserved, use [`swap_remove`] instead.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    ///
    /// [`swap_remove`]: Self::swap_remove
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut v = bump.alloc_slice_copy(&[1, 2, 3]);
    /// assert_eq!(v.remove(1), 2);
    /// assert_eq!(v, [1, 3]);
    /// ```
    #[inline(always)]
    pub fn remove(&mut self, index: usize) -> T {
        self.initialized.remove(index)
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    #[must_use]
    #[inline(always)]
    pub const fn as_slice(&self) -> &[T] {
        self.initialized.as_slice()
    }

    /// Extracts a mutable slice containing the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.initialized.as_mut_slice()
    }

    /// Returns a raw pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        self.initialized.as_ptr()
    }

    /// Returns an unsafe mutable pointer to the slice, or a dangling
    /// raw pointer valid for zero sized reads.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.initialized.as_mut_ptr()
    }

    /// Returns a `NonNull` pointer to the vector's buffer, or a dangling
    /// `NonNull` pointer valid for zero sized reads if the vector didn't allocate.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up dangling.
    /// Modifying the vector may cause its buffer to be reallocated,
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
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// // Allocate vector big enough for 4 elements.
    /// let size = 4;
    /// let mut x: FixedBumpVec<i32> = FixedBumpVec::with_capacity_in(size, &bump);
    /// let x_ptr = x.as_non_null();
    ///
    /// // Initialize elements via raw pointer writes, then set length.
    /// unsafe {
    ///     for i in 0..size {
    ///         x_ptr.add(i).write(i as i32);
    ///     }
    ///     x.set_len(size);
    /// }
    /// assert_eq!(&*x, &[0, 1, 2, 3]);
    /// ```
    ///
    /// Due to the aliasing guarantee, the following code is legal:
    ///
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// unsafe {
    ///     let v = bump_vec![in &bump; 0].into_fixed_vec();
    ///     let ptr1 = v.as_non_null();
    ///     ptr1.write(1);
    ///     let ptr2 = v.as_non_null();
    ///     ptr2.write(2);
    ///     // Notably, the write to `ptr2` did *not* invalidate `ptr1`:
    ///     ptr1.write(3);
    /// }
    /// ```
    ///
    /// [`as_mut_ptr`]: Self::as_mut_ptr
    /// [`as_ptr`]: Self::as_ptr
    /// [`as_non_null`]: Self::as_non_null
    #[must_use]
    #[inline(always)]
    pub const fn as_non_null(&self) -> NonNull<T> {
        self.initialized.as_non_null()
    }

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[doc(hidden)]
    #[deprecated = "renamed to `as_non_null`"]
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_ptr(&self) -> NonNull<T> {
        self.initialized.as_non_null()
    }

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[doc(hidden)]
    #[deprecated = "too niche; compute this yourself if needed"]
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_slice(&self) -> NonNull<[T]> {
        #[expect(deprecated)]
        self.initialized.as_non_null_slice()
    }

    #[inline(always)]
    pub(crate) unsafe fn set_ptr(&mut self, new_ptr: NonNull<T>) {
        unsafe { self.initialized.set_ptr(new_ptr) };
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type. Normally changing the length of a vector
    /// is done using one of the safe operations instead, such as
    /// [`truncate`] or [`clear`].
    ///
    /// # Safety
    /// - `new_len` must be less than or equal to the [`capacity`].
    /// - The elements at `old_len..new_len` must be initialized.
    ///
    /// [`truncate`]: Self::truncate
    /// [`clear`]: Self::clear
    /// [`capacity`]: Self::capacity
    #[inline(always)]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        unsafe { self.initialized.set_len(new_len) };
    }

    #[inline(always)]
    pub(crate) unsafe fn set_cap(&mut self, new_cap: usize) {
        self.capacity = new_cap;
    }

    #[inline]
    pub(crate) unsafe fn inc_len(&mut self, amount: usize) {
        unsafe {
            self.initialized.inc_len(amount);
        }
    }

    /// Removes an element from the vector and returns it.
    ///
    /// The removed element is replaced by the last element of the vector.
    ///
    /// This does not preserve ordering, but is *O*(1).
    /// If you need to preserve the element order, use [`remove`] instead.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    ///
    /// [`remove`]: Self::remove
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut v = bump.alloc_slice_copy(&["foo", "bar", "baz", "qux"]);
    ///
    /// assert_eq!(v.swap_remove(1), "bar");
    /// assert_eq!(v, ["foo", "qux", "baz"]);
    ///
    /// assert_eq!(v.swap_remove(0), "foo");
    /// assert_eq!(v, ["baz", "qux"]);
    /// ```
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        self.initialized.swap_remove(index)
    }

    /// Splits the vector into two by removing the specified range.
    ///
    /// This method does not allocate and does not change the order of the elements.
    ///
    /// The excess capacity may end up in either vector.
    /// This behavior is different from <code>Vec::[split_off](alloc_crate::vec::Vec::split_off)</code> which allocates a new vector for the split-off elements
    /// so the original vector keeps its capacity.
    /// If you rather want that behavior then you can write this instead:
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// # let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
    /// # vec.append([1, 2, 3, 4, 5]);
    /// # let start = 1;
    /// # let end = 4;
    /// let mut other = FixedBumpVec::with_capacity_in(end - start, &bump);
    /// other.append(vec.drain(start..end));
    /// # assert_eq!(vec, [1, 5]);
    /// # assert_eq!(other, [2, 3, 4]);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if the end point is greater than the length of the vector.
    ///
    /// # Complexity
    ///
    /// This operation takes `O(1)` time if either the range starts at 0, ends at `len`, or is empty.
    /// Otherwise it takes `O(min(end, len - start))` time.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
    /// vec.append([1, 2, 3, 4, 5, 6, 7, 8]);
    ///
    /// let front = vec.split_off(..2);
    /// assert_eq!(front, [1, 2]);
    /// assert_eq!(vec, [3, 4, 5, 6, 7, 8]);
    ///
    /// let back = vec.split_off(4..);
    /// assert_eq!(back, [7, 8]);
    /// assert_eq!(vec, [3, 4, 5, 6]);
    ///
    /// let middle = vec.split_off(1..3);
    /// assert_eq!(middle, [4, 5]);
    /// assert_eq!(vec, [3, 6]);
    ///
    /// let rest = vec.split_off(..);
    /// assert_eq!(rest, [3, 6]);
    /// assert_eq!(vec, []);
    /// ```
    #[inline]
    #[expect(clippy::return_self_not_must_use)]
    pub fn split_off(&mut self, range: impl RangeBounds<usize>) -> Self {
        let len = self.len();
        let ops::Range { start, end } = polyfill::slice::range(range, ..len);
        let ptr = self.initialized.as_non_null();

        unsafe {
            if T::IS_ZST {
                let range_len = end - start;
                let remaining_len = len - range_len;

                self.set_len(remaining_len);

                return FixedBumpVec {
                    initialized: BumpBox::zst_slice_from_len(range_len),
                    capacity: usize::MAX,
                };
            }

            if end == len {
                let lhs = ptr;
                let rhs = ptr.add(start);

                let lhs_len = start;
                let rhs_len = len - start;

                let lhs_cap = start;
                let rhs_cap = self.capacity - lhs_cap;

                self.set_ptr(lhs);
                self.set_len(lhs_len);
                self.set_cap(lhs_cap);

                return FixedBumpVec {
                    initialized: BumpBox::from_raw(NonNull::slice_from_raw_parts(rhs, rhs_len)),
                    capacity: rhs_cap,
                };
            }

            if start == 0 {
                let lhs = ptr;
                let rhs = ptr.add(end);

                let lhs_len = end;
                let rhs_len = len - end;

                let lhs_cap = end;
                let rhs_cap = self.capacity - lhs_cap;

                self.set_ptr(rhs);
                self.set_len(rhs_len);
                self.set_cap(rhs_cap);

                return FixedBumpVec {
                    initialized: BumpBox::from_raw(NonNull::slice_from_raw_parts(lhs, lhs_len)),
                    capacity: lhs_cap,
                };
            }

            if start == end {
                return FixedBumpVec::new();
            }

            let head_len = start;
            let tail_len = len - end;

            let range_len = end - start;
            let remaining_len = len - range_len;

            if head_len < tail_len {
                // move the range of elements to split off to the start
                self.as_mut_slice().get_unchecked_mut(..end).rotate_right(range_len);

                let lhs = ptr;
                let rhs = ptr.add(range_len);

                let lhs_len = range_len;
                let rhs_len = remaining_len;

                let lhs_cap = range_len;
                let rhs_cap = self.capacity - lhs_cap;

                self.set_ptr(rhs);
                self.set_len(rhs_len);
                self.set_cap(rhs_cap);

                FixedBumpVec {
                    initialized: BumpBox::from_raw(NonNull::slice_from_raw_parts(lhs, lhs_len)),
                    capacity: lhs_cap,
                }
            } else {
                // move the range of elements to split off to the end
                self.as_mut_slice().get_unchecked_mut(start..).rotate_left(range_len);

                let lhs = ptr;
                let rhs = ptr.add(remaining_len);

                let lhs_len = remaining_len;
                let rhs_len = range_len;

                let lhs_cap = remaining_len;
                let rhs_cap = self.capacity - lhs_cap;

                self.set_ptr(lhs);
                self.set_len(lhs_len);
                self.set_cap(lhs_cap);

                FixedBumpVec {
                    initialized: BumpBox::from_raw(NonNull::slice_from_raw_parts(rhs, rhs_len)),
                    capacity: rhs_cap,
                }
            }
        }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(3, &bump);
    /// vec.extend_from_slice_copy(&[1, 2]);
    /// vec.push(3);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn push(&mut self, value: T) {
        panic_on_error(self.generic_push(value));
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = FixedBumpVec::try_with_capacity_in(3, &bump)?;
    /// vec.try_extend_from_slice_copy(&[1, 2])?;
    /// vec.try_push(3)?;
    /// assert_eq!(vec, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_push(&mut self, value: T) -> Result<(), AllocError> {
        self.generic_push(value)
    }

    #[inline]
    pub(crate) fn generic_push<E: ErrorBehavior>(&mut self, value: T) -> Result<(), E> {
        self.generic_push_with(|| value)
    }

    /// Reserves space for one more element, then calls `f`
    /// to produce the value that is appended.
    ///
    /// In some cases this could be more performant than `push(f())` because it
    /// permits the compiler to directly place `T` in the vector instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(3, &bump);
    /// vec.append([1, 2]);
    /// vec.push_with(|| 3);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn push_with(&mut self, f: impl FnOnce() -> T) {
        panic_on_error(self.generic_push_with(f));
    }

    /// Reserves space for one more element, then calls `f`
    /// to produce the value that is appended.
    ///
    /// In some cases this could be more performant than `push(f())` because it
    /// permits the compiler to directly place `T` in the vector instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::try_with_capacity_in(3, &bump)?;
    /// vec.try_append([1, 2])?;
    /// vec.try_push_with(|| 3)?;
    /// assert_eq!(vec, [1, 2, 3]);
    ///
    /// let push_result = vec.try_push_with(|| unreachable!());
    /// assert!(push_result.is_err());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_push_with(&mut self, f: impl FnOnce() -> T) -> Result<(), AllocError> {
        self.generic_push_with(f)
    }

    #[inline]
    pub(crate) fn generic_push_with<E: ErrorBehavior>(&mut self, f: impl FnOnce() -> T) -> Result<(), E> {
        self.generic_reserve_one()?;
        unsafe {
            self.push_with_unchecked(f);
        }
        Ok(())
    }

    /// Inserts an element at position `index` within the vector, shifting all elements after it to the right.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// Panics if `index > len`.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.insert(1, 4);
    /// assert_eq!(vec, [1, 4, 2, 3]);
    /// vec.insert(4, 5);
    /// assert_eq!(vec, [1, 4, 2, 3, 5]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn insert(&mut self, index: usize, element: T) {
        panic_on_error(self.generic_insert(index, element));
    }

    /// Inserts an element at position `index` within the vector, shifting all elements after it to the right.
    ///
    /// # Panics
    /// Panics if `index > len`.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = FixedBumpVec::try_with_capacity_in(5, &bump)?;
    /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
    /// vec.try_insert(1, 4)?;
    /// assert_eq!(vec, [1, 4, 2, 3]);
    /// vec.try_insert(4, 5)?;
    /// assert_eq!(vec, [1, 4, 2, 3, 5]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_insert(&mut self, index: usize, element: T) -> Result<(), AllocError> {
        self.generic_insert(index, element)
    }

    #[inline]
    pub(crate) fn generic_insert<E: ErrorBehavior>(&mut self, index: usize, element: T) -> Result<(), E> {
        #[cold]
        #[track_caller]
        #[inline(never)]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("insertion index (is {index}) should be <= len (is {len})");
        }

        if index > self.len() {
            assert_failed(index, self.len());
        }

        self.generic_reserve_one()?;

        unsafe {
            let pos = self.as_mut_ptr().add(index);

            if index != self.len() {
                let len = self.len() - index;
                ptr::copy(pos, pos.add(1), len);
            }

            pos.write(element);
            self.inc_len(1);
        }

        Ok(())
    }

    /// Copies and appends all elements in a slice to the `FixedBumpVec`.
    ///
    /// Iterates over the `slice`, copies each element, and then appends
    /// it to this `FixedBumpVec`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with copyable slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpVec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(4, &bump);
    /// vec.push(1);
    /// vec.extend_from_slice_copy(&[2, 3, 4]);
    /// assert_eq!(vec, [1, 2, 3, 4]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_from_slice_copy(&mut self, slice: &[T])
    where
        T: Copy,
    {
        panic_on_error(self.generic_extend_from_slice_copy(slice));
    }

    /// Copies and appends all elements in a slice to the `FixedBumpVec`.
    ///
    /// Iterates over the `slice`, copies each element, and then appends
    /// it to this `FixedBumpVec`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with copyable slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpVec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::try_with_capacity_in(4, &bump)?;
    /// vec.try_push(1)?;
    /// vec.try_extend_from_slice_copy(&[2, 3, 4])?;
    /// assert_eq!(vec, [1, 2, 3, 4]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_extend_from_slice_copy(&mut self, slice: &[T]) -> Result<(), AllocError>
    where
        T: Copy,
    {
        self.generic_extend_from_slice_copy(slice)
    }

    #[inline]
    pub(crate) fn generic_extend_from_slice_copy<E: ErrorBehavior>(&mut self, slice: &[T]) -> Result<(), E>
    where
        T: Copy,
    {
        unsafe { self.extend_by_copy_nonoverlapping(slice) }
    }

    /// Clones and appends all elements in a slice to the `FixedBumpVec`.
    ///
    /// Iterates over the `slice`, clones each element, and then appends
    /// it to this `FixedBumpVec`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use std::string::String;
    /// # use bump_scope::{ Bump, FixedBumpVec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(3, &bump);
    /// vec.push(String::from("a"));
    /// vec.extend_from_slice_clone(&[String::from("b"), String::from("c")]);
    /// assert_eq!(vec, ["a", "b", "c"]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_from_slice_clone(&mut self, slice: &[T])
    where
        T: Clone,
    {
        panic_on_error(self.generic_extend_from_slice_clone(slice));
    }

    /// Clones and appends all elements in a slice to the `FixedBumpVec`.
    ///
    /// Iterates over the `slice`, clones each element, and then appends
    /// it to this `FixedBumpVec`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use std::string::String;
    /// # use bump_scope::{ Bump, FixedBumpVec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::try_with_capacity_in(3, &bump)?;
    /// vec.try_push(String::from("a"))?;
    /// vec.try_extend_from_slice_clone(&[String::from("b"), String::from("c")])?;
    /// assert_eq!(vec, ["a", "b", "c"]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_extend_from_slice_clone(&mut self, slice: &[T]) -> Result<(), AllocError>
    where
        T: Clone,
    {
        self.generic_extend_from_slice_clone(slice)
    }

    #[inline]
    pub(crate) fn generic_extend_from_slice_clone<E: ErrorBehavior>(&mut self, slice: &[T]) -> Result<(), E>
    where
        T: Clone,
    {
        self.generic_reserve(slice.len())?;

        unsafe {
            let mut pos = 0usize;

            while likely(pos != slice.len()) {
                let elem = slice.get_unchecked(pos);
                self.push_unchecked(elem.clone());
                pos += 1;
            }
        }

        Ok(())
    }

    /// Copies elements from `src` range to the end of the vector.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(100, &bump);
    /// vec.extend_from_slice_copy(&[0, 1, 2, 3, 4]);
    ///
    /// vec.extend_from_within_copy(2..);
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4]);
    ///
    /// vec.extend_from_within_copy(..2);
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1]);
    ///
    /// vec.extend_from_within_copy(4..8);
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1, 4, 2, 3, 4]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_from_within_copy<R>(&mut self, src: R)
    where
        T: Copy,
        R: RangeBounds<usize>,
    {
        panic_on_error(self.generic_extend_from_within_copy(src));
    }

    /// Copies elements from `src` range to the end of the vector.
    ///
    /// # Panics
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = FixedBumpVec::try_with_capacity_in(100, &bump)?;
    /// vec.try_extend_from_slice_copy(&[0, 1, 2, 3, 4])?;
    ///
    /// vec.try_extend_from_within_copy(2..)?;
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4]);
    ///
    /// vec.try_extend_from_within_copy(..2)?;
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1]);
    ///
    /// vec.try_extend_from_within_copy(4..8)?;
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1, 4, 2, 3, 4]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_extend_from_within_copy<R>(&mut self, src: R) -> Result<(), AllocError>
    where
        T: Copy,
        R: RangeBounds<usize>,
    {
        self.generic_extend_from_within_copy(src)
    }

    #[inline]
    pub(crate) fn generic_extend_from_within_copy<E: ErrorBehavior, R>(&mut self, src: R) -> Result<(), E>
    where
        T: Copy,
        R: RangeBounds<usize>,
    {
        let range = slice::range(src, ..self.len());
        let count = range.len();

        self.generic_reserve(count)?;

        // SAFETY:
        // - `slice::range` guarantees that the given range is valid for indexing self
        unsafe {
            let ptr = self.as_mut_ptr();

            let src = ptr.add(range.start);
            let dst = ptr.add(self.len());
            ptr::copy_nonoverlapping(src, dst, count);

            self.inc_len(count);
            Ok(())
        }
    }

    /// Clones elements from `src` range to the end of the vector.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(14, &bump);
    /// vec.extend_from_slice_copy(&[0, 1, 2, 3, 4]);
    ///
    /// vec.extend_from_within_clone(2..);
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4]);
    ///
    /// vec.extend_from_within_clone(..2);
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1]);
    ///
    /// vec.extend_from_within_clone(4..8);
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1, 4, 2, 3, 4]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_from_within_clone<R>(&mut self, src: R)
    where
        T: Clone,
        R: RangeBounds<usize>,
    {
        panic_on_error(self.generic_extend_from_within_clone(src));
    }

    /// Clones elements from `src` range to the end of the vector.
    ///
    /// # Panics
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = FixedBumpVec::try_with_capacity_in(14, &bump)?;
    /// vec.try_extend_from_slice_copy(&[0, 1, 2, 3, 4])?;
    ///
    /// vec.try_extend_from_within_clone(2..)?;
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4]);
    ///
    /// vec.try_extend_from_within_clone(..2)?;
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1]);
    ///
    /// vec.try_extend_from_within_clone(4..8)?;
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1, 4, 2, 3, 4]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_extend_from_within_clone<R>(&mut self, src: R) -> Result<(), AllocError>
    where
        T: Clone,
        R: RangeBounds<usize>,
    {
        self.generic_extend_from_within_clone(src)
    }

    #[inline]
    pub(crate) fn generic_extend_from_within_clone<E: ErrorBehavior, R>(&mut self, src: R) -> Result<(), E>
    where
        T: Clone,
        R: RangeBounds<usize>,
    {
        let range = slice::range(src, ..self.len());
        let count = range.len();

        self.generic_reserve(count)?;

        if T::IS_ZST {
            unsafe {
                // We can materialize ZST's from nothing.
                #[expect(clippy::uninit_assumed_init)]
                let fake = ManuallyDrop::new(MaybeUninit::<T>::uninit().assume_init());

                for _ in 0..count {
                    self.push_unchecked((*fake).clone());
                }

                return Ok(());
            }
        }

        // SAFETY:
        // - `slice::range` guarantees that the given range is valid for indexing self
        unsafe {
            let ptr = self.as_mut_ptr();

            let mut src = ptr.add(range.start);
            let mut dst = ptr.add(self.len());

            let src_end = src.add(count);

            while src != src_end {
                dst.write((*src).clone());

                src = src.add(1);
                dst = dst.add(1);
                self.inc_len(1);
            }
        }

        Ok(())
    }

    /// Checks if at least `additional` more elements can be inserted
    /// in the given `FixedBumpVec<T>` due to capacity.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve(&mut self, additional: usize) {
        panic_on_error(self.generic_reserve(additional));
    }

    /// Checks if at least `additional` more elements can be inserted
    /// in the given `FixedBumpVec<T>` due to capacity.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    #[inline(always)]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), AllocError> {
        self.generic_reserve(additional)
    }

    #[inline]
    pub(crate) fn generic_reserve<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        if additional > (self.capacity() - self.len()) {
            Err(E::fixed_size_vector_no_space(additional))
        } else {
            Ok(())
        }
    }

    /// Resizes the `FixedBumpVec` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `FixedBumpVec` is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the `FixedBumpVec` is simply truncated.
    ///
    /// This method requires `T` to implement [`Clone`],
    /// in order to be able to clone the passed value.
    /// If you need more flexibility (or want to rely on [`Default`] instead of
    /// [`Clone`]), use [`resize_with`].
    /// If you only need to resize to a smaller size, use [`truncate`].
    ///
    /// [`resize_with`]: Self::resize_with
    /// [`truncate`]: Self::truncate
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
    /// vec.extend_from_slice_copy(&["hello"]);
    /// vec.resize(3, "world");
    /// assert_eq!(vec, ["hello", "world", "world"]);
    /// drop(vec);
    ///
    /// let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3, 4]);
    /// vec.resize(2, 0);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        panic_on_error(self.generic_resize(new_len, value));
    }

    /// Resizes the `FixedBumpVec` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `FixedBumpVec` is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the `FixedBumpVec` is simply truncated.
    ///
    /// This method requires `T` to implement [`Clone`],
    /// in order to be able to clone the passed value.
    /// If you need more flexibility (or want to rely on [`Default`] instead of
    /// [`Clone`]), use [`resize_with`].
    /// If you only need to resize to a smaller size, use [`truncate`].
    ///
    /// [`resize_with`]: Self::resize_with
    /// [`truncate`]: Self::truncate
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = FixedBumpVec::try_with_capacity_in(10, &bump)?;
    /// vec.try_extend_from_slice_copy(&["hello"])?;
    /// vec.try_resize(3, "world")?;
    /// assert_eq!(vec, ["hello", "world", "world"]);
    /// drop(vec);
    ///
    /// let mut vec = FixedBumpVec::try_with_capacity_in(10, &bump)?;
    /// vec.try_extend_from_slice_copy(&[1, 2, 3, 4])?;
    /// vec.try_resize(2, 0)?;
    /// assert_eq!(vec, [1, 2]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_resize(&mut self, new_len: usize, value: T) -> Result<(), AllocError>
    where
        T: Clone,
    {
        self.generic_resize(new_len, value)
    }

    #[inline]
    pub(crate) fn generic_resize<E: ErrorBehavior>(&mut self, new_len: usize, value: T) -> Result<(), E>
    where
        T: Clone,
    {
        let len = self.len();

        if new_len > len {
            self.extend_with(new_len - len, value)
        } else {
            self.truncate(new_len);
            Ok(())
        }
    }

    /// Resizes the `FixedBumpVec` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `FixedBumpVec` is extended by the
    /// difference, with each additional slot filled with the result of
    /// calling the closure `f`. The return values from `f` will end up
    /// in the `FixedBumpVec` in the order they have been generated.
    ///
    /// If `new_len` is less than `len`, the `FixedBumpVec` is simply truncated.
    ///
    /// This method uses a closure to create new values on every push. If
    /// you'd rather [`Clone`] a given value, use [`FixedBumpVec::resize`]. If you
    /// want to use the [`Default`] trait to generate values, you can
    /// pass [`Default::default`] as the second argument.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.resize_with(5, Default::default);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// drop(vec);
    ///
    /// let mut vec = FixedBumpVec::with_capacity_in(4, &bump);
    /// let mut p = 1;
    /// vec.resize_with(4, || { p *= 2; p });
    /// assert_eq!(vec, [2, 4, 8, 16]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> T,
    {
        panic_on_error(self.generic_resize_with(new_len, f));
    }

    /// Resizes the `FixedBumpVec` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `FixedBumpVec` is extended by the
    /// difference, with each additional slot filled with the result of
    /// calling the closure `f`. The return values from `f` will end up
    /// in the `FixedBumpVec` in the order they have been generated.
    ///
    /// If `new_len` is less than `len`, the `FixedBumpVec` is simply truncated.
    ///
    /// This method uses a closure to create new values on every push. If
    /// you'd rather [`Clone`] a given value, use [`FixedBumpVec::resize`]. If you
    /// want to use the [`Default`] trait to generate values, you can
    /// pass [`Default::default`] as the second argument.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = FixedBumpVec::try_with_capacity_in(5, &bump)?;
    /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
    /// vec.try_resize_with(5, Default::default)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// drop(vec);
    ///
    /// let mut vec = FixedBumpVec::try_with_capacity_in(4, &bump)?;
    /// let mut p = 1;
    /// vec.try_resize_with(4, || { p *= 2; p })?;
    /// assert_eq!(vec, [2, 4, 8, 16]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_resize_with<F>(&mut self, new_len: usize, f: F) -> Result<(), AllocError>
    where
        F: FnMut() -> T,
    {
        self.generic_resize_with(new_len, f)
    }

    #[inline]
    pub(crate) fn generic_resize_with<E: ErrorBehavior, F>(&mut self, new_len: usize, f: F) -> Result<(), E>
    where
        F: FnMut() -> T,
    {
        let len = self.len();
        if new_len > len {
            unsafe { self.extend_trusted(iter::repeat_with(f).take(new_len - len)) }
        } else {
            self.truncate(new_len);
            Ok(())
        }
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(8, &bump);
    ///
    /// // append by value
    /// vec.append([1, 2]);
    /// vec.append(vec![3, 4]);
    /// vec.append(bump.alloc_iter(5..=6));
    ///
    /// // append by mutable reference
    /// let mut other = vec![7, 8];
    /// vec.append(&mut other);
    ///
    /// assert_eq!(other, []);
    /// assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn append(&mut self, other: impl OwnedSlice<Item = T>) {
        panic_on_error(self.generic_append(other));
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::try_with_capacity_in(8, &bump)?;
    ///
    /// // append by value
    /// vec.try_append([1, 2])?;
    /// vec.try_append(vec![3, 4])?;
    /// vec.try_append(bump.alloc_iter(5..=6))?;
    ///
    /// // append by mutable reference
    /// let mut other = vec![7, 8];
    /// vec.try_append(&mut other)?;
    ///
    /// assert_eq!(other, []);
    /// assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_append(&mut self, other: impl OwnedSlice<Item = T>) -> Result<(), AllocError> {
        self.generic_append(other)
    }

    #[inline]
    pub(crate) fn generic_append<E: ErrorBehavior>(&mut self, other: impl OwnedSlice<Item = T>) -> Result<(), E> {
        unsafe {
            let mut owned_slice = other.into_take_owned_slice();

            let slice = NonNull::from(owned_slice.owned_slice_ref());
            self.generic_reserve(slice.len())?;

            let src = slice.cast::<T>().as_ptr();
            let dst = self.as_mut_ptr().add(self.len());
            ptr::copy_nonoverlapping(src, dst, slice.len());

            owned_slice.take_owned_slice();
            self.inc_len(slice.len());
            Ok(())
        }
    }

    /// Returns a fixed vector of the same size as `self`, with function `f` applied to each element in order.
    ///
    /// This function only compiles when `U`s size and alignment is less or equal to `T`'s or if `U` has a size of 0.
    ///
    /// # Examples
    /// Mapping to a type with an equal alignment and size:
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # use core::num::NonZero;
    /// # let bump: Bump = Bump::new();
    /// let a = FixedBumpVec::from_iter_exact_in([0, 1, 2], &bump);
    /// let b = a.map_in_place(NonZero::new);
    /// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
    /// ```
    ///
    /// Mapping to a type with a smaller alignment and size:
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let a: FixedBumpVec<u32> = FixedBumpVec::from_iter_exact_in([0, 1, 2], &bump);
    /// assert_eq!(a.capacity(), 3);
    ///
    /// let b: FixedBumpVec<u16> = a.map_in_place(|i| i as u16);
    /// assert_eq!(b.capacity(), 6);
    ///
    /// assert_eq!(b, [0, 1, 2]);
    /// ```
    ///
    /// Mapping to a type with a greater alignment won't compile:
    /// ```compile_fail,E0080
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let a: FixedBumpVec<[u8; 4]> = FixedBumpVec::from_iter_exact_in([[0, 1, 2, 3]], &bump);
    /// let b: FixedBumpVec<u32> = a.map_in_place(u32::from_le_bytes);
    /// # _ = b;
    /// ```
    ///
    /// Mapping to a type with a greater size won't compile:
    /// ```compile_fail,E0080
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let a: FixedBumpVec<u32> = FixedBumpVec::from_iter_exact_in([42], &bump);
    /// let b: FixedBumpVec<[u32; 2]> = a.map_in_place(|i| [i; 2]);
    /// # _ = b;
    /// ```
    pub fn map_in_place<U>(self, f: impl FnMut(T) -> U) -> FixedBumpVec<'a, U> {
        let Self { initialized, capacity } = self;

        FixedBumpVec {
            initialized: initialized.map_in_place(f),
            capacity: if U::IS_ZST {
                usize::MAX
            } else {
                (capacity * T::SIZE) / U::SIZE
            },
        }
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, value: T) {
        unsafe {
            self.push_with_unchecked(|| value);
        }
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn push_with_unchecked(&mut self, f: impl FnOnce() -> T) {
        debug_assert!(!self.is_full());

        unsafe {
            let ptr = self.as_mut_ptr().add(self.len());
            pointer::write_with(ptr, f);
            self.inc_len(1);
        }
    }

    /// Extend the vector by `n` clones of value.
    fn extend_with<B: ErrorBehavior>(&mut self, n: usize, value: T) -> Result<(), B>
    where
        T: Clone,
    {
        self.generic_reserve(n)?;
        unsafe {
            self.extend_with_unchecked(n, value);
        }
        Ok(())
    }

    /// Extend the vector by `n` clones of value.
    ///
    /// # Safety
    /// Capacity must be sufficient.
    pub(crate) unsafe fn extend_with_unchecked(&mut self, n: usize, value: T)
    where
        T: Clone,
    {
        unsafe {
            let mut ptr = self.as_mut_ptr().add(self.len());

            // Use SetLenOnDrop to work around bug where compiler
            // might not realize the store through `ptr` through self.set_len()
            // don't alias.
            let mut local_len = self.initialized.set_len_on_drop();

            // Write all elements except the last one
            for _ in 1..n {
                pointer::write_with(ptr, || value.clone());
                ptr = ptr.add(1);

                // Increment the length in every step in case clone() panics
                local_len.increment_len(1);
            }

            if n > 0 {
                // We can write the last element directly without cloning needlessly
                ptr.write(value);
                local_len.increment_len(1);
            }

            // len set by scope guard
        }
    }

    /// Retains only the elements specified by the predicate, passing a mutable reference to it.
    ///
    /// In other words, remove all elements `e` such that `f(&mut e)` returns `false`.
    /// This method operates in place, visiting each element exactly once in the
    /// original order, and preserves the order of the retained elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = FixedBumpVec::with_capacity_in(4, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3, 4]);
    ///
    /// vec.retain(|x| if *x <= 3 {
    ///     *x += 1;
    ///     true
    /// } else {
    ///     false
    /// });
    ///
    /// assert_eq!(vec, [2, 3, 4]);
    /// ```
    #[expect(clippy::pedantic)]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        self.initialized.retain(f)
    }

    /// Removes the specified range from the vector in bulk, returning all
    /// removed elements as an iterator. If the iterator is dropped before
    /// being fully consumed, it drops the remaining removed elements.
    ///
    /// The returned iterator keeps a mutable borrow on the vector to optimize
    /// its implementation.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Leaking
    ///
    /// If the returned iterator goes out of scope without being dropped (due to
    /// [`mem::forget`](core::mem::forget), for example), the vector may have lost and leaked
    /// elements arbitrarily, including elements outside the range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut v = FixedBumpVec::with_capacity_in(3, &bump);
    /// v.extend_from_slice_copy(&[1, 2, 3]);
    /// let u = bump.alloc_iter(v.drain(1..));
    /// assert_eq!(v, [1]);
    /// assert_eq!(u, [2, 3]);
    ///
    /// // A full range clears the vector, like `clear()` does
    /// v.drain(..);
    /// assert_eq!(v, []);
    /// ```
    pub fn drain<R>(&mut self, range: R) -> owned_slice::Drain<'_, T>
    where
        R: RangeBounds<usize>,
    {
        self.initialized.drain(range)
    }

    /// Creates an iterator which uses a closure to determine if an element should be removed.
    ///
    /// If the closure returns true, then the element is removed and yielded.
    /// If the closure returns false, the element will remain in the vector and will not be yielded
    /// by the iterator.
    ///
    /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped without iterating
    /// or the iteration short-circuits, then the remaining elements will be retained.
    /// Use [`retain`] with a negated predicate if you do not need the returned iterator.
    ///
    /// Using this method is equivalent to the following code:
    ///
    /// ```
    /// # let some_predicate = |x: &mut i32| { *x == 2 || *x == 3 || *x == 6 };
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// # let mut vec = FixedBumpVec::with_capacity_in(6, &bump);
    /// # vec.extend_from_slice_copy(&[1, 2, 3, 4, 5, 6]);
    /// let mut i = 0;
    /// while i < vec.len() {
    ///     if some_predicate(&mut vec[i]) {
    ///         let val = vec.remove(i);
    ///         // your code here
    ///         # _ = val;
    ///     } else {
    ///         i += 1;
    ///     }
    /// }
    ///
    /// # assert_eq!(vec, [1, 4, 5]);
    /// ```
    ///
    /// But `extract_if` is easier to use. `extract_if` is also more efficient,
    /// because it can backshift the elements of the array in bulk.
    ///
    /// Note that `extract_if` also lets you mutate every element in the filter closure,
    /// regardless of whether you choose to keep or remove it.
    ///
    /// # Examples
    ///
    /// Splitting an array into evens and odds, reusing the original allocation:
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut numbers = FixedBumpVec::with_capacity_in(16, &bump);
    /// numbers.extend_from_slice_copy(&[1, 2, 3, 4, 5, 6, 8, 9, 11, 13, 14, 15]);
    ///
    /// let evens = bump.alloc_iter(numbers.extract_if(|x| *x % 2 == 0));
    /// let odds = numbers;
    ///
    /// assert_eq!(evens, [2, 4, 6, 8, 14]);
    /// assert_eq!(odds, [1, 3, 5, 9, 11, 13, 15]);
    /// ```
    ///
    /// [`retain`]: Self::retain
    pub fn extract_if<F>(&mut self, filter: F) -> owned_slice::ExtractIf<'_, T, F>
    where
        F: FnMut(&mut T) -> bool,
    {
        self.initialized.extract_if(filter)
    }

    /// Removes consecutive repeated elements in the vector according to the
    /// [`PartialEq`] trait implementation.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 2, 3, 2]);
    ///
    /// vec.dedup();
    ///
    /// assert_eq!(vec, [1, 2, 3, 2]);
    /// ```
    #[inline]
    pub fn dedup(&mut self)
    where
        T: PartialEq,
    {
        self.initialized.dedup();
    }

    /// Removes all but the first of consecutive elements in the vector that resolve to the same
    /// key.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
    /// vec.extend_from_slice_copy(&[10, 20, 21, 30, 20]);
    ///
    /// vec.dedup_by_key(|i| *i / 10);
    ///
    /// assert_eq!(vec, [10, 20, 30, 20]);
    /// ```
    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.initialized.dedup_by_key(key);
    }

    /// Removes all but the first of consecutive elements in the vector satisfying a given equality
    /// relation.
    ///
    /// The `same_bucket` function is passed references to two elements from the vector and
    /// must determine if the elements compare equal. The elements are passed in opposite order
    /// from their order in the vector, so if `same_bucket(a, b)` returns `true`, `a` is removed.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
    /// vec.extend_from_slice_copy(&["foo", "bar", "Bar", "baz", "bar"]);
    ///
    /// vec.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    ///
    /// assert_eq!(vec, ["foo", "bar", "baz", "bar"]);
    /// ```
    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        self.initialized.dedup_by(same_bucket);
    }

    /// Returns the vector content as a boxed slice of `T`, along with the remaining spare
    /// capacity of the vector as a boxed slice of `MaybeUninit<T>`.
    #[inline]
    #[must_use]
    pub fn split_at_spare(self) -> (BumpBox<'a, [T]>, BumpBox<'a, [MaybeUninit<T>]>) {
        unsafe {
            let uninitialized_ptr = self
                .initialized
                .as_non_null()
                .add(self.initialized.len())
                .cast::<MaybeUninit<T>>();
            let uninitialized_len = self.capacity - self.len();
            let uninitialized = BumpBox::from_raw(NonNull::slice_from_raw_parts(uninitialized_ptr, uninitialized_len));
            (self.initialized, uninitialized)
        }
    }

    #[inline(always)]
    pub(crate) fn into_raw_parts(self) -> (BumpBox<'a, [T]>, usize) {
        (self.initialized, self.capacity)
    }

    #[inline(always)]
    pub(crate) unsafe fn from_raw_parts(initialized: BumpBox<'a, [T]>, capacity: usize) -> Self {
        Self { initialized, capacity }
    }

    #[inline(always)]
    unsafe fn extend_by_copy_nonoverlapping<B: ErrorBehavior>(&mut self, other: *const [T]) -> Result<(), B> {
        unsafe {
            let len = other.len();
            self.generic_reserve(len)?;

            let src = other.cast::<T>();
            let dst = self.as_mut_ptr().add(self.len());
            ptr::copy_nonoverlapping(src, dst, len);

            self.inc_len(len);
            Ok(())
        }
    }

    /// # Safety
    ///
    /// `iterator` must satisfy the invariants of nightly's `TrustedLen`.
    unsafe fn extend_trusted<B: ErrorBehavior>(&mut self, iterator: impl Iterator<Item = T>) -> Result<(), B> {
        unsafe {
            let (low, high) = iterator.size_hint();
            if let Some(additional) = high {
                debug_assert_eq!(
                    low,
                    additional,
                    "TrustedLen iterator's size hint is not exact: {:?}",
                    (low, high)
                );

                self.generic_reserve(additional)?;

                let ptr = self.as_mut_ptr();
                let mut local_len = self.initialized.set_len_on_drop();

                iterator.for_each(move |element| {
                    let dst = ptr.add(local_len.current_len());

                    ptr::write(dst, element);
                    // Since the loop executes user code which can panic we have to update
                    // the length every step to correctly drop what we've written.
                    // NB can't overflow since we would have had to alloc the address space
                    local_len.increment_len(1);
                });

                Ok(())
            } else {
                // Per TrustedLen contract a `None` upper bound means that the iterator length
                // truly exceeds usize::MAX, which would eventually lead to a capacity overflow anyway.
                // Since the other branch already panics eagerly (via `reserve()`) we do the same here.
                // This avoids additional codegen for a fallback code path which would eventually
                // panic anyway.
                Err(B::fixed_size_vector_is_full())
            }
        }
    }

    #[inline(always)]
    fn generic_reserve_one<B: ErrorBehavior>(&self) -> Result<(), B> {
        if self.is_full() {
            Err(B::fixed_size_vector_is_full())
        } else {
            Ok(())
        }
    }
}

impl<'a, T, const N: usize> FixedBumpVec<'a, [T; N]> {
    /// Takes a `FixedBumpVec<[T; N]>` and flattens it into a `FixedBumpVec<T>`.
    ///
    /// # Panics
    ///
    /// Panics if the length of the resulting vector would overflow a `usize`.
    ///
    /// This is only possible when flattening a vector of arrays of zero-sized
    /// types, and thus tends to be irrelevant in practice. If
    /// `size_of::<T>() > 0`, this will never panic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, FixedBumpVec};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = FixedBumpVec::with_capacity_in(3, &bump);
    /// vec.append([[1, 2, 3], [4, 5, 6], [7, 8, 9]]);
    /// assert_eq!(vec.pop(), Some([7, 8, 9]));
    ///
    /// let mut flattened = vec.into_flattened();
    /// assert_eq!(flattened.pop(), Some(6));
    /// ```
    #[must_use]
    pub fn into_flattened(self) -> FixedBumpVec<'a, T> {
        let (initialized, cap) = self.into_raw_parts();
        let ptr = initialized.into_raw();
        let len = ptr.len();

        let (new_len, new_cap) = if T::IS_ZST {
            (len.checked_mul(N).expect("vec len overflow"), usize::MAX)
        } else {
            // SAFETY:
            // - `cap * N` cannot overflow because the allocation is already in
            // the address space.
            // - Each `[T; N]` has `N` valid elements, so there are `len * N`
            // valid elements in the allocation.
            unsafe { (polyfill::usize::unchecked_mul(len, N), polyfill::usize::unchecked_mul(cap, N)) }
        };

        // SAFETY:
        // - `ptr` was allocated by `self`
        // - `ptr` is well-aligned because `[T; N]` has the same alignment as `T`.
        // - `new_cap` refers to the same sized allocation as `cap` because
        // `new_cap * size_of::<T>()` == `cap * size_of::<[T; N]>()`
        // - `len` <= `cap`, so `len * N` <= `cap * N`.
        unsafe {
            let slice = NonNull::slice_from_raw_parts(ptr.cast(), new_len);
            let initialized = BumpBox::from_raw(slice);
            FixedBumpVec::from_raw_parts(initialized, new_cap)
        }
    }
}

impl<T> Debug for FixedBumpVec<'_, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.initialized.fmt(f)
    }
}

impl<T> Default for FixedBumpVec<'_, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for FixedBumpVec<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.initialized
    }
}

impl<T> DerefMut for FixedBumpVec<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.initialized
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for FixedBumpVec<'_, T> {
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self.as_slice(), index)
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for FixedBumpVec<'_, T> {
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self.as_mut_slice(), index)
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<T> Extend<T> for FixedBumpVec<'_, T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        self.reserve(iter.size_hint().0);

        for value in iter {
            self.push(value);
        }
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<'t, T> Extend<&'t T> for FixedBumpVec<'_, T>
where
    T: Clone + 't,
{
    #[inline]
    fn extend<I: IntoIterator<Item = &'t T>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        self.reserve(iter.size_hint().0);

        for value in iter {
            self.push(value.clone());
        }
    }
}

impl<'a, T> IntoIterator for FixedBumpVec<'a, T> {
    type Item = T;
    type IntoIter = owned_slice::IntoIter<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.initialized.into_iter()
    }
}

impl<'c, T> IntoIterator for &'c FixedBumpVec<'_, T> {
    type Item = &'c T;
    type IntoIter = slice::Iter<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'c, T> IntoIterator for &'c mut FixedBumpVec<'_, T> {
    type Item = &'c mut T;
    type IntoIter = slice::IterMut<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<T> AsRef<Self> for FixedBumpVec<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T> AsMut<Self> for FixedBumpVec<'_, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<T> AsRef<[T]> for FixedBumpVec<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> AsMut<[T]> for FixedBumpVec<'_, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T> Borrow<[T]> for FixedBumpVec<'_, T> {
    #[inline(always)]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T> BorrowMut<[T]> for FixedBumpVec<'_, T> {
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T: Hash> Hash for FixedBumpVec<'_, T> {
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

/// Returns [`ErrorKind::OutOfMemory`](std::io::ErrorKind::OutOfMemory) when extending fails.
#[cfg(feature = "std")]
impl std::io::Write for FixedBumpVec<'_, u8> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.try_extend_from_slice_copy(buf).is_err() {
            return Err(std::io::ErrorKind::OutOfMemory.into());
        }

        Ok(buf.len())
    }

    #[inline(always)]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        let len = bufs.iter().map(|b| b.len()).sum();
        self.try_reserve(len).map_err(|_| std::io::ErrorKind::OutOfMemory)?;
        for buf in bufs {
            self.try_extend_from_slice_copy(buf)
                .map_err(|_| std::io::ErrorKind::OutOfMemory)?;
        }
        Ok(len)
    }

    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if self.try_extend_from_slice_copy(buf).is_err() {
            return Err(std::io::ErrorKind::OutOfMemory.into());
        }

        Ok(())
    }
}

impl<T: NoDrop> NoDrop for FixedBumpVec<'_, T> {}
