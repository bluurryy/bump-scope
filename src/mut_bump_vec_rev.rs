use core::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    hash::Hash,
    iter,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut, Index, IndexMut, RangeBounds},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::{self, NonNull},
    slice::{self, SliceIndex},
};

use crate::{
    alloc::AllocError,
    destructure::destructure,
    min_non_zero_cap,
    mut_bump_vec::IntoIter,
    mut_collection_method_allocator_stats,
    owned_slice::{OwnedSlice, TakeOwnedSlice},
    polyfill::{self, nonnull, pointer},
    BumpBox, ErrorBehavior, MutBumpAllocator, MutBumpAllocatorScope, NoDrop, SetLenOnDrop, SizedTypeProperties, Stats,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// This is like [`vec!`](alloc_crate::vec!) but allocates inside a bump allocator, returning a [`MutBumpVecRev`].
///
/// `$bump` can be any type that implements [`MutBumpAllocator`].
///
/// # Panics
/// If used without `try`, panics on allocation failure.
///
/// # Errors
/// If used with `try`, errors on allocation failure.
///
/// # Examples
///
/// There are three forms of this macro:
///
/// - Create an empty [`MutBumpVecRev`]:
/// ```
/// # use bump_scope::{Bump, mut_bump_vec_rev, MutBumpVecRev};
/// # let mut bump: Bump = Bump::new();
/// let vec: MutBumpVecRev<i32, _> = mut_bump_vec_rev![in &mut bump];
/// assert!(vec.is_empty());
/// ```
///
/// - Create a [`MutBumpVecRev`] containing a given list of elements:
///
/// ```
/// # use bump_scope::{Bump, mut_bump_vec_rev};
/// # let mut bump: Bump = Bump::new();
/// let vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
/// assert_eq!(vec[0], 1);
/// assert_eq!(vec[1], 2);
/// assert_eq!(vec[2], 3);
/// ```
///
/// - Create a [`MutBumpVecRev`] from a given element and size:
///
/// ```
/// # use bump_scope::{Bump, mut_bump_vec_rev};
/// # let mut bump: Bump = Bump::new();
/// let vec = mut_bump_vec_rev![in &mut bump; 1; 3];
/// assert_eq!(vec, [1, 1, 1]);
/// ```
///
/// Note that unlike array expressions this syntax supports all elements
/// which implement [`Clone`] and the number of elements doesn't have to be
/// a constant.
///
/// This will use `clone` to duplicate an expression, so one should be careful
/// using this with types having a nonstandard `Clone` implementation. For
/// example, `mut_bump_vec_rev![in &mut bump; Rc::new(1); 5]` will create a vector of five references
/// to the same boxed integer value, not five references pointing to independently
/// boxed integers.
///
/// Also, note that `mut_bump_vec_rev![in &mut bump; expr; 0]` is allowed, and produces an empty vector.
/// This will still evaluate `expr`, however, and immediately drop the resulting value, so
/// be mindful of side effects.
#[macro_export]
macro_rules! mut_bump_vec_rev {
    [in $bump:expr] => {
        $crate::MutBumpVecRev::new_in($bump)
    };
    [in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::MutBumpVecRev::from_owned_slice_in([$($values),*], $bump)
    };
    [in $bump:expr; $value:expr; $count:expr] => {
        $crate::MutBumpVecRev::from_elem_in($value, $count, $bump)
    };
    [try in $bump:expr] => {
        Ok::<_, $crate::alloc::AllocError>($crate::MutBumpVecRev::new_in($bump))
    };
    [try in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::MutBumpVecRev::try_from_owned_slice_in([$($values),*], $bump)
    };
    [try in $bump:expr; $value:expr; $count:expr] => {
        $crate::MutBumpVecRev::try_from_elem_in($value, $count, $bump)
    };
}

/// A type like [`MutBumpVec`](crate::MutBumpVec) but new elements are pushed to the front.
///
/// The point of this vector is to have a more performant <code>[into](Self::into_slice)([_boxed](Self::into_boxed_slice))[_slice](Self::into_slice)</code> for a downwards bumping allocator.
///
/// # Examples
///
/// This type can be used to allocate a slice, when `alloc_*` methods are too limiting:
/// ```
/// # use bump_scope::{Bump, mut_bump_vec_rev};
/// # let mut bump: Bump = Bump::new();
/// let mut vec = mut_bump_vec_rev![in &mut bump];
///
/// vec.push(1);
/// vec.push(2);
/// vec.push(3);
///
/// let slice: &[i32] = vec.into_slice();
///
/// assert_eq!(slice, [3, 2, 1]);
/// ```
///
/// When extending a `MutBumpVecRev` by a slice, the elements have the same order as in the source slice.
///
/// ```
/// # use bump_scope::{Bump, mut_bump_vec_rev};
/// # let mut bump: Bump = Bump::new();
/// let mut vec = mut_bump_vec_rev![in &mut bump; 4, 5, 6];
///
/// vec.extend_from_slice_copy(&[1, 2, 3]);
///
/// assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
/// ```
//
// MutBumpVecRev never actually moves a bump pointer.
// It may force allocation of a new chunk, but it does not move the pointer within.
// So we don't need to move the bump pointer when dropping.
//
// If we want to reset the bump pointer to a previous chunk, we use a bump scope.
// We could do it here, by resetting to the last non-empty chunk but that would require a loop.
// Chunk allocations are supposed to be very rare, so this wouldn't be worth it.
pub struct MutBumpVecRev<T, A> {
    /// This points at the end of the slice (`ptr` + `len`).
    /// When `T` is a ZST this is always `NonNull::<T>::dangling()`.
    pub(crate) end: NonNull<T>,
    pub(crate) len: usize,

    /// When `T` is a ZST this is always `usize::MAX`.
    cap: usize,

    allocator: A,

    /// Marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    marker: PhantomData<T>,
}

impl<T: UnwindSafe, A: UnwindSafe> UnwindSafe for MutBumpVecRev<T, A> {}
impl<T: RefUnwindSafe, A: RefUnwindSafe> RefUnwindSafe for MutBumpVecRev<T, A> {}

impl<T, A> MutBumpVecRev<T, A> {
    /// Constructs a new empty `MutBumpVecRev<T>`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// let vec = MutBumpVecRev::<i32, _>::new_in(&mut bump);
    /// assert_eq!(vec.len(), 0);
    /// assert_eq!(vec.capacity(), 0);
    /// ```
    #[inline]
    pub fn new_in(allocator: A) -> Self {
        Self {
            end: NonNull::dangling(),
            len: 0,
            cap: if T::IS_ZST { usize::MAX } else { 0 },
            allocator,
            marker: PhantomData,
        }
    }

    /// Returns the total number of elements the vector can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// let vec = MutBumpVecRev::<i32, _>::with_capacity_in(2048, &mut bump);
    /// assert!(vec.capacity() >= 2048);
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.cap
    }

    /// Returns the number of elements in the vector, also referred to
    /// as its 'length'.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let a = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    /// assert_eq!(a.len(), 3);
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut v = MutBumpVecRev::new_in(&mut bump);
    /// assert!(v.is_empty());
    ///
    /// v.push(1);
    /// assert!(!v.is_empty());
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, value: T) {
        self.push_with_unchecked(|| value);
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn push_with_unchecked(&mut self, f: impl FnOnce() -> T) {
        debug_assert!(self.len < self.cap);

        let ptr = nonnull::sub(self.end, self.len + 1);
        nonnull::write_with(ptr, f);

        // We set the len here so when `f` panics, `self.len` doesn't change.
        self.len += 1;
    }

    /// Removes the first element from a vector and returns it, or [`None`] if it
    /// is empty.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    /// assert_eq!(vec.pop(), Some(1));
    /// assert_eq!(vec, [2, 3]);
    /// ```
    ///
    /// # Time complexity
    /// Takes *O*(1) time.
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                let ptr = self.as_ptr();
                self.len -= 1;
                Some(ptr.read())
            }
        }
    }

    /// Removes and returns the last element from a vector if the predicate
    /// returns `true`, or [`None`] if the predicate returns false or the vector
    /// is empty (the predicate will not be called in that case).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4];
    /// let pred = |x: &mut i32| *x % 2 != 0;
    ///
    /// assert_eq!(vec.pop_if(pred), Some(1));
    /// assert_eq!(vec, [2, 3, 4]);
    /// assert_eq!(vec.pop_if(pred), None);
    /// ```
    pub fn pop_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        let last = self.first_mut()?;
        if predicate(last) {
            self.pop()
        } else {
            None
        }
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut v = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    ///
    /// v.clear();
    ///
    /// assert!(v.is_empty());
    /// ```
    #[inline(always)]
    pub fn clear(&mut self) {
        let elems: *mut [T] = self.as_mut_slice();

        // SAFETY:
        // - `elems` comes directly from `as_mut_slice` and is therefore valid.
        // - Setting `self.len` before calling `drop_in_place` means that,
        //   if an element's `Drop` impl panics, the vector's `Drop` impl will
        //   do nothing (leaking the rest of the elements) instead of dropping
        //   some twice.
        unsafe {
            self.len = 0;
            ptr::drop_in_place(elems);
        }
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    #[must_use]
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    /// Extracts a mutable slice of the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }

    /// Returns a raw pointer to the vector's buffer, or a dangling raw pointer
    /// valid for zero sized reads if the vector didn't allocate.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the vector may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    ///
    /// The caller must also ensure that the memory the pointer (non-transitively) points to
    /// is never written to (except inside an `UnsafeCell`) using this pointer or any pointer
    /// derived from it. If you need to mutate the contents of the slice, use [`as_mut_ptr`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let x = mut_bump_vec_rev![in &mut bump; 1, 2, 4];
    /// let x_ptr = x.as_ptr();
    ///
    /// unsafe {
    ///     for i in 0..x.len() {
    ///         assert_eq!(*x_ptr.add(i), 1 << i);
    ///     }
    /// }
    /// ```
    ///
    /// [`as_mut_ptr`]: Self::as_mut_ptr
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        // We shadow the slice method of the same name to avoid going through
        // `deref`, which creates an intermediate reference.
        self.as_non_null().as_ptr()
    }

    /// Returns an unsafe mutable pointer to the vector's buffer, or a dangling
    /// raw pointer valid for zero sized reads if the vector didn't allocate.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the vector may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// // Allocate vector big enough for 4 elements.
    /// let size = 4;
    /// let mut x = MutBumpVecRev::<i32, _>::with_capacity_in(size, &mut bump);
    /// let x_ptr = unsafe { x.as_mut_ptr().sub(size) };
    ///
    /// // Initialize elements via raw pointer writes, then set length.
    /// unsafe {
    ///     for i in 0..size {
    ///         *x_ptr.add(i) = i as i32;
    ///     }
    ///     x.set_len(size);
    /// }
    ///
    /// assert_eq!(&*x, &[0, 1, 2, 3]);
    /// ```
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        // We shadow the slice method of the same name to avoid going through
        // `deref_mut`, which creates an intermediate reference.
        self.as_non_null().as_ptr()
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
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// // Allocate vector big enough for 4 elements.
    /// let size = 4;
    /// let mut x: MutBumpVecRev<i32, _> = MutBumpVecRev::with_capacity_in(size, &mut bump);
    /// let x_ptr = x.as_non_null();
    ///
    /// // Initialize elements via raw pointer writes, then set length.
    /// unsafe {
    ///     for i in 0..size {
    ///         x_ptr.sub(i + 1).write(i as i32);
    ///     }
    ///     x.set_len(size);
    /// }
    /// assert_eq!(&*x, &[3, 2, 1, 0]);
    /// ```
    ///
    /// Due to the aliasing guarantee, the following code is legal:
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// unsafe {
    ///     let v = mut_bump_vec_rev![in &mut bump; 0];
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
        // SAFETY: The start pointer is never null.
        unsafe { nonnull::sub(self.end, self.len) }
    }

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[doc(hidden)]
    #[deprecated = "renamed to `as_non_null`"]
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_ptr(&self) -> NonNull<T> {
        unsafe { nonnull::sub(self.end, self.len) }
    }

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[doc(hidden)]
    #[deprecated = "too niche; compute this yourself if needed"]
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_slice(&self) -> NonNull<[T]> {
        nonnull::slice_from_raw_parts(self.as_non_null(), self.len)
    }

    /// Shortens the vector, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// If `len` is greater than the vector's current length, this has no
    /// effect.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the vector.
    ///
    /// # Examples
    ///
    /// Truncating a five element vector to two elements:
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4, 5];
    /// vec.truncate(2);
    /// assert_eq!(vec, [4, 5]);
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current
    /// length:
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    /// vec.truncate(8);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    /// vec.truncate(0);
    /// assert_eq!(vec, []);
    /// ```
    ///
    /// [`clear`]: Self::clear
    pub fn truncate(&mut self, len: usize) {
        // This is safe because:
        //
        // * the slice passed to `drop_in_place` is valid; the `len > self.len`
        //   case avoids creating an invalid slice, and
        // * the `len` of the vector is shrunk before calling `drop_in_place`,
        //   such that no value will be dropped twice in case `drop_in_place`
        //   were to panic once (if it panics twice, the program aborts).
        unsafe {
            // Unlike std this is `>=`. Std uses `>` because when a call is inlined with `len` of `0` that optimizes better.
            // But this was likely only motivated because `clear` used to be implemented as `truncate(0)`.
            // See <https://github.com/rust-lang/rust/issues/76089#issuecomment-1889416842>.
            if len >= self.len {
                return;
            }

            let remaining_len = self.len - len;

            let ptr = self.as_mut_ptr();
            let slice = ptr::slice_from_raw_parts_mut(ptr, remaining_len);

            self.len = len;
            slice.drop_in_place();
        }
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type. Normally changing the length of a vector
    /// is done using one of the safe operations instead, such as
    /// [`resize`], [`truncate`], [`extend`], or [`clear`].
    ///
    /// [`truncate`]: Self::truncate
    /// [`resize`]: Self::resize
    /// [`extend`]: Self::extend
    /// [`clear`]: Self::clear
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to [`capacity`].
    /// - The elements at `old_len..new_len` must be initialized.
    ///
    /// [`capacity`]: Self::capacity
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.cap);
        self.len = new_len;
    }

    #[inline]
    pub(crate) unsafe fn inc_len(&mut self, amount: usize) {
        self.len += amount;
    }

    #[inline(always)]
    fn into_raw_parts(self) -> (NonNull<T>, usize, usize, A) {
        destructure!(let Self { end, len, cap, allocator } = self);
        (end, len, cap, allocator)
    }

    #[allow(dead_code)]
    #[inline(always)]
    unsafe fn from_raw_parts(end: NonNull<T>, len: usize, cap: usize, allocator: A) -> Self {
        Self {
            end,
            len,
            cap,
            allocator,
            marker: PhantomData,
        }
    }
}

impl<T, A: MutBumpAllocator> MutBumpVecRev<T, A> {
    /// Constructs a new empty vector with at least the specified capacity
    /// in the provided bump allocator.
    ///
    /// The vector will be able to hold `capacity` elements without
    /// reallocating. If `capacity` is 0, the vector will not allocate.
    ///
    /// It is important to note that although the returned vector has the
    /// minimum *capacity* specified, the vector will have a zero *length*. For
    /// an explanation of the difference between length and capacity, see
    /// *[Capacity and reallocation]*.
    ///
    /// When `T` is a zero-sized type, there will be no allocation
    /// and the capacity will always be `usize::MAX`.
    ///
    /// [Capacity and reallocation]: alloc_crate::vec::Vec#capacity-and-reallocation
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = MutBumpVecRev::<i32, _>::with_capacity_in(10, &mut bump);
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     vec.push(i);
    /// }
    /// assert_eq!(vec.len(), 10);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // ...but this may make the vector reallocate
    /// vec.push(11);
    /// assert_eq!(vec.len(), 11);
    /// assert!(vec.capacity() >= 11);
    ///
    /// drop(vec);
    ///
    /// // A vector of a zero-sized type will always over-allocate, since no
    /// // allocation is necessary
    /// let vec_units = MutBumpVecRev::<(), _>::with_capacity_in(10, &mut bump);
    /// assert_eq!(vec_units.capacity(), usize::MAX);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_capacity_in(capacity: usize, allocator: A) -> Self {
        panic_on_error(Self::generic_with_capacity_in(capacity, allocator))
    }

    /// Constructs a new empty vector with at least the specified capacity
    /// in the provided bump allocator.
    ///
    /// The vector will be able to hold `capacity` elements without
    /// reallocating. If `capacity` is 0, the vector will not allocate.
    ///
    /// It is important to note that although the returned vector has the
    /// minimum *capacity* specified, the vector will have a zero *length*. For
    /// an explanation of the difference between length and capacity, see
    /// *[Capacity and reallocation]*.
    ///
    /// When `T` is a zero-sized type, there will be no allocation
    /// and the capacity will always be `usize::MAX`.
    ///
    /// [Capacity and reallocation]: alloc_crate::vec::Vec#capacity-and-reallocation
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = MutBumpVecRev::<i32, _>::try_with_capacity_in(10, &mut bump)?;
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     vec.push(i);
    /// }
    /// assert_eq!(vec.len(), 10);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // ...but this may make the vector reallocate
    /// vec.push(11);
    /// assert_eq!(vec.len(), 11);
    /// assert!(vec.capacity() >= 11);
    ///
    /// drop(vec);
    ///
    /// // A vector of a zero-sized type will always over-allocate, since no
    /// // allocation is necessary
    /// let vec_units = MutBumpVecRev::<(), _>::try_with_capacity_in(10, &mut bump)?;
    /// assert_eq!(vec_units.capacity(), usize::MAX);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_capacity_in(capacity: usize, allocator: A) -> Result<Self, AllocError> {
        Self::generic_with_capacity_in(capacity, allocator)
    }

    #[inline]
    pub(crate) fn generic_with_capacity_in<E: ErrorBehavior>(capacity: usize, allocator: A) -> Result<Self, E> {
        let mut allocator = allocator;

        if T::IS_ZST {
            return Ok(Self {
                end: NonNull::dangling(),
                len: 0,
                cap: usize::MAX,
                allocator,
                marker: PhantomData,
            });
        }

        if capacity == 0 {
            return Ok(Self {
                end: NonNull::dangling(),
                len: 0,
                cap: 0,
                allocator,
                marker: PhantomData,
            });
        }

        let (end, cap) = unsafe { E::prepare_slice_allocation_rev(&mut allocator, capacity)? };

        Ok(Self {
            end,
            len: 0,
            cap,
            allocator,
            marker: PhantomData,
        })
    }

    /// Constructs a new `MutBumpVecRev<T>` and pushes `value` `count` times.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// let vec = MutBumpVecRev::from_elem_in("ho", 3, &mut bump);
    /// assert_eq!(vec, ["ho", "ho", "ho"]);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_elem_in(value: T, count: usize, allocator: A) -> Self
    where
        T: Clone,
    {
        panic_on_error(Self::generic_from_elem_in(value, count, allocator))
    }

    /// Constructs a new `MutBumpVecRev<T>` and pushes `value` `count` times.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let vec = MutBumpVecRev::try_from_elem_in("ho", 3, &mut bump)?;
    /// assert_eq!(vec, ["ho", "ho", "ho"]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_elem_in(value: T, count: usize, allocator: A) -> Result<Self, AllocError>
    where
        T: Clone,
    {
        Self::generic_from_elem_in(value, count, allocator)
    }

    #[inline]
    pub(crate) fn generic_from_elem_in<E: ErrorBehavior>(value: T, count: usize, allocator: A) -> Result<Self, E>
    where
        T: Clone,
    {
        let mut vec = Self::generic_with_capacity_in(count, allocator)?;

        unsafe {
            if count != 0 {
                for _ in 0..(count - 1) {
                    vec.push_with_unchecked(|| value.clone());
                }

                vec.push_with_unchecked(|| value);
            }
        }

        Ok(vec)
    }

    /// Constructs a new `MutBumpVecRev<T>` from an [`OwnedSlice`].
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let bump: Bump = Bump::new();
    /// # let mut bump_a: Bump = Bump::new();
    /// # let mut bump_b: Bump = Bump::new();
    /// # let mut bump_c: Bump = Bump::new();
    /// # let mut bump_d: Bump = Bump::new();
    /// // by value
    /// let a = MutBumpVecRev::from_owned_slice_in([1, 2], &mut bump_a);
    /// let b = MutBumpVecRev::from_owned_slice_in(vec![3, 4], &mut bump_b);
    /// let c = MutBumpVecRev::from_owned_slice_in(bump.alloc_iter(5..=6), &mut bump_c);
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = MutBumpVecRev::from_owned_slice_in(&mut other, &mut bump_d);
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(a, [1, 2]);
    /// assert_eq!(b, [3, 4]);
    /// assert_eq!(c, [5, 6]);
    /// assert_eq!(d, [7, 8]);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_owned_slice_in(owned_slice: impl OwnedSlice<Item = T>, allocator: A) -> Self {
        panic_on_error(Self::generic_from_owned_slice_in(owned_slice, allocator))
    }

    /// Constructs a new `MutBumpVecRev<T>` from an [`OwnedSlice`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let bump: Bump = Bump::new();
    /// # let mut bump_a: Bump = Bump::new();
    /// # let mut bump_b: Bump = Bump::new();
    /// # let mut bump_c: Bump = Bump::new();
    /// # let mut bump_d: Bump = Bump::new();
    /// // by value
    /// let a = MutBumpVecRev::try_from_owned_slice_in([1, 2], &mut bump_a)?;
    /// let b = MutBumpVecRev::try_from_owned_slice_in(vec![3, 4], &mut bump_b)?;
    /// let c = MutBumpVecRev::try_from_owned_slice_in(bump.alloc_iter(5..=6), &mut bump_c)?;
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = MutBumpVecRev::try_from_owned_slice_in(&mut other, &mut bump_d)?;
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(a, [1, 2]);
    /// assert_eq!(b, [3, 4]);
    /// assert_eq!(c, [5, 6]);
    /// assert_eq!(d, [7, 8]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_owned_slice_in(owned_slice: impl OwnedSlice<Item = T>, allocator: A) -> Result<Self, AllocError> {
        Self::generic_from_owned_slice_in(owned_slice, allocator)
    }

    #[inline]
    pub(crate) fn generic_from_owned_slice_in<E: ErrorBehavior>(
        owned_slice: impl OwnedSlice<Item = T>,
        allocator: A,
    ) -> Result<Self, E> {
        let owned_slice = owned_slice.into_take_owned_slice();
        let mut this = Self::generic_with_capacity_in(owned_slice.owned_slice_ref().len(), allocator)?;
        this.generic_append(owned_slice)?;
        Ok(this)
    }

    /// Constructs a new `MutBumpVecRev<T>` from a `[T; N]`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[doc(hidden)]
    #[deprecated = "use `from_owned_slice_in` instead"]
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_array_in<const N: usize>(array: [T; N], allocator: A) -> Self {
        panic_on_error(Self::generic_from_array_in(array, allocator))
    }

    /// Constructs a new `MutBumpVecRev<T>` from a `[T; N]`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[doc(hidden)]
    #[deprecated = "use `try_from_owned_slice_in` instead"]
    #[inline(always)]
    pub fn try_from_array_in<const N: usize>(array: [T; N], allocator: A) -> Result<Self, AllocError> {
        Self::generic_from_array_in(array, allocator)
    }

    #[inline]
    pub(crate) fn generic_from_array_in<E: ErrorBehavior, const N: usize>(array: [T; N], allocator: A) -> Result<Self, E> {
        #![allow(clippy::needless_pass_by_value)]
        #![allow(clippy::needless_pass_by_ref_mut)]

        let array = ManuallyDrop::new(array);
        let mut allocator = allocator;

        if T::IS_ZST {
            return Ok(Self {
                end: NonNull::dangling(),
                len: N,
                cap: usize::MAX,
                allocator,
                marker: PhantomData,
            });
        }

        if N == 0 {
            return Ok(Self {
                end: NonNull::dangling(),
                len: 0,
                cap: 0,
                allocator,
                marker: PhantomData,
            });
        }

        let (end, cap) = unsafe { E::prepare_slice_allocation_rev::<T>(&mut allocator, N)? };
        let src = array.as_ptr();

        unsafe {
            let dst = end.as_ptr().sub(N);
            ptr::copy_nonoverlapping(src, dst, N);
        };

        Ok(Self {
            end,
            len: N,
            cap,
            allocator,
            marker: PhantomData,
        })
    }

    /// Create a new [`MutBumpVecRev`] whose elements are taken from an iterator and allocated in the given `bump`.
    ///
    /// This is behaviorally identical to [`FromIterator::from_iter`].
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// let vec = MutBumpVecRev::from_iter_in([1, 2, 3], &mut bump);
    /// assert_eq!(vec, [3, 2, 1]);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_iter_in<I>(iter: I, allocator: A) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        panic_on_error(Self::generic_from_iter_in(iter, allocator))
    }

    /// Create a new [`MutBumpVecRev`] whose elements are taken from an iterator and allocated in the given `bump`.
    ///
    /// This is behaviorally identical to [`FromIterator::from_iter`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let vec = MutBumpVecRev::try_from_iter_in([1, 2, 3], &mut bump)?;
    /// assert_eq!(vec, [3, 2, 1]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_iter_in<I>(iter: I, allocator: A) -> Result<Self, AllocError>
    where
        I: IntoIterator<Item = T>,
    {
        Self::generic_from_iter_in(iter, allocator)
    }

    #[inline]
    pub(crate) fn generic_from_iter_in<E: ErrorBehavior, I>(iter: I, allocator: A) -> Result<Self, E>
    where
        I: IntoIterator<Item = T>,
    {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = Self::generic_with_capacity_in(capacity, allocator)?;

        for value in iter {
            vec.generic_push(value)?;
        }

        Ok(vec)
    }

    /// Appends an element to the front of a collection.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{mut_bump_vec_rev, Bump};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 2, 1];
    /// vec.push(3);
    /// assert_eq!(vec, [3, 2, 1]);
    /// # let _ = vec;
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn push(&mut self, value: T) {
        panic_on_error(self.generic_push(value));
    }

    /// Appends an element to the front of a collection.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{mut_bump_vec_rev, Bump};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec_rev![try in bump; 2, 1]?;
    /// vec.try_push(3)?;
    /// assert_eq!(vec, [3, 2, 1]);
    /// # let _ = vec;
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

    /// Appends an element to the front of a collection.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn push_with(&mut self, f: impl FnOnce() -> T) {
        panic_on_error(self.generic_push_with(f));
    }

    /// Appends an element to the front of a collection.
    ///
    /// # Errors
    /// Errors if the allocation fails.
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
    /// Panics if the allocation fails.
    ///
    /// Panics if `index > len`.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{mut_bump_vec_rev, Bump};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
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
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{mut_bump_vec_rev, Bump};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec_rev![try in bump; 1, 2, 3]?;
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

        if index > self.len {
            assert_failed(index, self.len);
        }

        self.generic_reserve_one()?;

        unsafe {
            if index == 0 {
                self.len += 1;
                self.as_mut_ptr().write(element);
            } else {
                let start = self.as_mut_ptr();
                let start_sub = start.sub(1);
                ptr::copy(start, start_sub, index);
                self.len += 1;
                start_sub.add(index).write(element);
            }
        }

        Ok(())
    }

    /// Copies and appends all elements in a slice to the `MutBumpVecRev`.
    ///
    /// Iterates over the `slice`, copies each element, and then appends
    /// it to this `MutBumpVecRev`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with copyable slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_from_slice_copy(&mut self, slice: &[T])
    where
        T: Copy,
    {
        panic_on_error(self.generic_extend_from_slice_copy(slice));
    }

    /// Copies and appends all elements in a slice to the `MutBumpVecRev`.
    ///
    /// Iterates over the `slice`, copies each element, and then appends
    /// it to this `MutBumpVecRev`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with copyable slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Errors
    /// Errors if the allocation fails.
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

    /// Clones and appends all elements in a slice to the `MutBumpVecRev`.
    ///
    /// Iterates over the `slice`, clones each element, and then appends
    /// it to this `MutBumpVecRev`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn extend_from_slice_clone(&mut self, slice: &[T])
    where
        T: Clone,
    {
        panic_on_error(self.generic_extend_from_slice_clone(slice));
    }

    /// Clones and appends all elements in a slice to the `MutBumpVecRev`.
    ///
    /// Iterates over the `slice`, clones each element, and then appends
    /// it to this `MutBumpVecRev`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Errors
    /// Errors if the allocation fails.
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
            // Addition doesn't overflow because `reserve` checked for that.
            let mut ptr = nonnull::sub(self.end, self.len);

            for value in slice.iter().rev() {
                ptr = nonnull::sub(ptr, 1);
                nonnull::write_with(ptr, || value.clone());
                self.len += 1;
            }
        }

        Ok(())
    }

    /// Copies elements from `src` range to the start of the vector.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 0, 1, 2, 3, 4];
    ///
    /// vec.extend_from_within_copy(2..);
    /// assert_eq!(vec, [2, 3, 4, 0, 1, 2, 3, 4]);
    ///
    /// vec.extend_from_within_copy(..2);
    /// assert_eq!(vec, [2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);
    ///
    /// vec.extend_from_within_copy(4..8);
    /// assert_eq!(vec, [4, 0, 1, 2, 2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);
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

    /// Copies elements from `src` range to the start of the vector.
    ///
    /// # Panics
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec_rev![try in bump; 0, 1, 2, 3, 4]?;
    ///
    /// vec.try_extend_from_within_copy(2..)?;
    /// assert_eq!(vec, [2, 3, 4, 0, 1, 2, 3, 4]);
    ///
    /// vec.try_extend_from_within_copy(..2)?;
    /// assert_eq!(vec, [2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);
    ///
    /// vec.try_extend_from_within_copy(4..8)?;
    /// assert_eq!(vec, [4, 0, 1, 2, 2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);
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
        let range = polyfill::slice::range(src, ..self.len());
        let count = range.len();

        self.generic_reserve(count)?;

        // SAFETY:
        // - `slice::range` guarantees that the given range is valid for indexing self
        unsafe {
            let ptr = self.as_mut_ptr();

            let src = ptr.add(range.start);
            let dst = ptr.sub(count);
            ptr::copy_nonoverlapping(src, dst, count);
        }

        self.len += count;
        Ok(())
    }

    /// Clones elements from `src` range to the end of the vector.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 0, 1, 2, 3, 4];
    ///
    /// vec.extend_from_within_clone(2..);
    /// assert_eq!(vec, [2, 3, 4, 0, 1, 2, 3, 4]);
    ///
    /// vec.extend_from_within_clone(..2);
    /// assert_eq!(vec, [2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);
    ///
    /// vec.extend_from_within_clone(4..8);
    /// assert_eq!(vec, [4, 0, 1, 2, 2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);
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
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec_rev![try in bump; 0, 1, 2, 3, 4]?;
    ///
    /// vec.try_extend_from_within_clone(2..)?;
    /// assert_eq!(vec, [2, 3, 4, 0, 1, 2, 3, 4]);
    ///
    /// vec.try_extend_from_within_clone(..2)?;
    /// assert_eq!(vec, [2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);
    ///
    /// vec.try_extend_from_within_clone(4..8)?;
    /// assert_eq!(vec, [4, 0, 1, 2, 2, 3, 2, 3, 4, 0, 1, 2, 3, 4]);
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
        let range = polyfill::slice::range(src, ..self.len());
        let count = range.len();

        self.generic_reserve(count)?;

        if T::IS_ZST {
            unsafe {
                // We can materialize ZST's from nothing.
                #[allow(clippy::uninit_assumed_init)]
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

            let mut src = ptr.add(range.end);
            let mut dst = ptr;

            let src_end = src.sub(count);

            while src != src_end {
                src = src.sub(1);
                dst = dst.sub(1);

                dst.write((*src).clone());

                self.len += 1;
            }
        }

        Ok(())
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `MutBumpVecRev<T>`. The collection may reserve more space to
    /// speculatively avoid frequent reallocations. After calling `reserve`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1];
    /// vec.reserve(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve(&mut self, additional: usize) {
        panic_on_error(self.generic_reserve(additional));
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `MutBumpVecRev<T>`. The collection may reserve more space to
    /// speculatively avoid frequent reallocations. After calling `reserve`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec_rev![try in bump; 1]?;
    /// vec.try_reserve(10)?;
    /// assert!(vec.capacity() >= 11);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), AllocError> {
        self.generic_reserve(additional)
    }

    #[inline]
    pub(crate) fn generic_reserve<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        if additional > (self.capacity() - self.len()) {
            self.generic_grow_amortized(additional)?;
        }

        Ok(())
    }

    /// Reserves the minimum capacity for at least `additional` more elements to
    /// be inserted in the given `MutBumpVecRev<T>`. Unlike [`reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `reserve_exact`, capacity will be greater than or equal to
    /// `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity can not be relied upon to be precisely
    /// minimal. Prefer [`reserve`] if future insertions are expected.
    ///
    /// [`reserve`]: Self::reserve
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1];
    /// vec.reserve_exact(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve_exact(&mut self, additional: usize) {
        panic_on_error(self.generic_reserve_exact(additional));
    }

    /// Reserves the minimum capacity for at least `additional` more elements to
    /// be inserted in the given `MutBumpVecRev<T>`. Unlike [`reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `reserve_exact`, capacity will be greater than or equal to
    /// `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity can not be relied upon to be precisely
    /// minimal. Prefer [`reserve`] if future insertions are expected.
    ///
    /// [`reserve`]: Self::reserve
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec_rev![try in bump; 1]?;
    /// vec.try_reserve_exact(10)?;
    /// assert!(vec.capacity() >= 11);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), AllocError> {
        self.generic_reserve_exact(additional)
    }

    #[inline]
    pub(crate) fn generic_reserve_exact<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        if additional > (self.capacity() - self.len()) {
            self.generic_grow_exact(additional)?;
        }

        Ok(())
    }

    /// Resizes the `MutBumpVecRev` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `MutBumpVecRev` is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the `MutBumpVecRev` is simply truncated.
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
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; "hello"];
    /// vec.resize(3, "world");
    /// assert_eq!(vec, ["world", "world", "hello"]);
    /// drop(vec);
    ///
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3, 4];
    /// vec.resize(2, 0);
    /// assert_eq!(vec, [3, 4]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        panic_on_error(self.generic_resize(new_len, value));
    }

    /// Resizes the `MutBumpVecRev` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `MutBumpVecRev` is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the `MutBumpVecRev` is simply truncated.
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
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec_rev![try in &mut bump; "hello"]?;
    /// vec.try_resize(3, "world")?;
    /// assert_eq!(vec, ["world", "world", "hello"]);
    /// drop(vec);
    ///
    /// let mut vec = mut_bump_vec_rev![try in &mut bump; 1, 2, 3, 4]?;
    /// vec.try_resize(2, 0)?;
    /// assert_eq!(vec, [3, 4]);
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

    /// Resizes the `MutBumpVecRev` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `MutBumpVecRev` is extended by the
    /// difference, with each additional slot filled with the result of
    /// calling the closure `f`. The return values from `f` will end up
    /// in the `MutBumpVecRev` in the order they have been generated.
    ///
    /// If `new_len` is less than `len`, the `MutBumpVecRev` is simply truncated.
    ///
    /// This method uses a closure to create new values on every push. If
    /// you'd rather [`Clone`] a given value, use [`MutBumpVecRev::resize`]. If you
    /// want to use the [`Default`] trait to generate values, you can
    /// pass [`Default::default`] as the second argument.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    /// vec.resize_with(5, Default::default);
    /// assert_eq!(vec, [0, 0, 1, 2, 3]);
    /// drop(vec);
    ///
    /// let mut vec = mut_bump_vec_rev![in &mut bump];
    /// let mut p = 1;
    /// vec.resize_with(4, || { p *= 2; p });
    /// assert_eq!(vec, [16, 8, 4, 2]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> T,
    {
        panic_on_error(self.generic_resize_with(new_len, f));
    }

    /// Resizes the `MutBumpVecRev` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `MutBumpVecRev` is extended by the
    /// difference, with each additional slot filled with the result of
    /// calling the closure `f`. The return values from `f` will end up
    /// in the `MutBumpVecRev` in the order they have been generated.
    ///
    /// If `new_len` is less than `len`, the `MutBumpVecRev` is simply truncated.
    ///
    /// This method uses a closure to create new values on every push. If
    /// you'd rather [`Clone`] a given value, use [`MutBumpVecRev::resize`]. If you
    /// want to use the [`Default`] trait to generate values, you can
    /// pass [`Default::default`] as the second argument.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec_rev![try in &mut bump; 1, 2, 3]?;
    /// vec.try_resize_with(5, Default::default)?;
    /// assert_eq!(vec, [0, 0, 1, 2, 3]);
    /// drop(vec);
    ///
    /// let mut vec = mut_bump_vec_rev![try in &mut bump]?;
    /// let mut p = 1;
    /// vec.try_resize_with(4, || { p *= 2; p })?;
    /// assert_eq!(vec, [16, 8, 4, 2]);
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
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// # let bump2: Bump = Bump::new();
    /// let mut vec = MutBumpVecRev::new_in(&mut bump);
    ///
    /// // append by value
    /// vec.append([1, 2]);
    /// vec.append(vec![3, 4]);
    /// vec.append(bump2.alloc_iter(5..=6));
    ///
    /// // append by mutable reference
    /// let mut other = vec![7, 8];
    /// vec.append(&mut other);
    ///
    /// assert_eq!(other, []);
    /// assert_eq!(vec, [7, 8, 5, 6, 3, 4, 1, 2]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn append(&mut self, other: impl OwnedSlice<Item = T>) {
        panic_on_error(self.generic_append(other));
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// # let bump2: Bump = Bump::new();
    /// let mut vec = MutBumpVecRev::new_in(&mut bump);
    ///
    /// // append by value
    /// vec.try_append([1, 2])?;
    /// vec.try_append(vec![3, 4])?;
    /// vec.try_append(bump2.alloc_iter(5..=6))?;
    ///
    /// // append by mutable reference
    /// let mut other = vec![7, 8];
    /// vec.try_append(&mut other)?;
    ///
    /// assert_eq!(other, []);
    /// assert_eq!(vec, [7, 8, 5, 6, 3, 4, 1, 2]);
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
            let dst = self.end.as_ptr().sub(self.len + slice.len());
            ptr::copy_nonoverlapping(src, dst, slice.len());

            owned_slice.take_owned_slice();
            self.inc_len(slice.len());
            Ok(())
        }
    }

    /// Extend the vector by `n` clones of value.
    fn extend_with<B: ErrorBehavior>(&mut self, n: usize, value: T) -> Result<(), B>
    where
        T: Clone,
    {
        self.generic_reserve(n)?;

        unsafe {
            let mut ptr = self.as_mut_ptr().sub(1);

            // Use SetLenOnDrop to work around bug where compiler
            // might not realize the store through `ptr` through self.set_len()
            // don't alias.
            let mut local_len = SetLenOnDrop::new(&mut self.len);

            // Write all elements except the last one
            for _ in 1..n {
                pointer::write_with(ptr, || value.clone());
                ptr = ptr.sub(1);

                // Increment the length in every step in case clone() panics
                local_len.increment_len(1);
            }

            if n > 0 {
                // We can write the last element directly without cloning needlessly
                ptr.write(value);
                local_len.increment_len(1);
            }

            Ok(())
            // len set by scope guard
        }
    }

    #[inline(always)]
    unsafe fn extend_by_copy_nonoverlapping<E: ErrorBehavior>(&mut self, other: *const [T]) -> Result<(), E> {
        let len = pointer::len(other);
        self.generic_reserve(len)?;

        let src = other.cast::<T>();
        self.len += len;
        let dst = self.as_mut_ptr();

        ptr::copy_nonoverlapping(src, dst, len);

        Ok(())
    }

    #[inline]
    fn generic_reserve_one<E: ErrorBehavior>(&mut self) -> Result<(), E> {
        if self.cap == self.len {
            self.generic_grow_amortized::<E>(1)?;
        }

        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn generic_grow_amortized<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        if T::IS_ZST {
            // This function is only called after we checked that the current capacity is not
            // sufficient. When `T::IS_ZST` the capacity is `usize::MAX`, so it can't grow.
            return Err(E::capacity_overflow());
        }

        let required_cap = match self.len().checked_add(additional) {
            Some(required_cap) => required_cap,
            None => return Err(E::capacity_overflow())?,
        };

        // This guarantees exponential growth. The doubling cannot overflow
        // because `capacity <= isize::MAX` and the type of `capacity` is usize;
        let new_cap = (self.capacity() * 2).max(required_cap).max(min_non_zero_cap(T::SIZE));

        unsafe { self.generic_grow_to(new_cap) }
    }

    #[cold]
    #[inline(never)]
    fn generic_grow_exact<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        if T::IS_ZST {
            // This function is only called after we checked that the current capacity is not
            // sufficient. When `T::IS_ZST` the capacity is `usize::MAX`, so it can't grow.
            return Err(E::capacity_overflow());
        }

        let required_cap = match self.len().checked_add(additional) {
            Some(required_cap) => required_cap,
            None => return Err(E::capacity_overflow())?,
        };

        unsafe { self.generic_grow_to(required_cap) }
    }

    /// # Safety
    ///
    /// `new_capacity` must be greater than the current capacity.
    unsafe fn generic_grow_to<E: ErrorBehavior>(&mut self, new_capacity: usize) -> Result<(), E> {
        let (end, cap) = E::prepare_slice_allocation_rev::<T>(&mut self.allocator, new_capacity)?;

        let src = self.as_mut_ptr();
        let dst = end.as_ptr().sub(self.len);
        ptr::copy_nonoverlapping(src, dst, self.len);

        self.end = end;
        self.cap = cap;

        Ok(())
    }

    /// Removes and returns the element at position `index` within the vector,
    /// shifting all elements after it to the right.
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
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// let mut v = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    /// assert_eq!(v.remove(1), 2);
    /// assert_eq!(v, [1, 3]);
    /// ```
    #[track_caller]
    pub fn remove(&mut self, index: usize) -> T {
        #[cold]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("removal index (is {index}) should be < len (is {len})");
        }

        if index >= self.len {
            assert_failed(index, self.len);
        }

        unsafe {
            let start = self.as_mut_ptr();
            let value_ptr = start.add(index);

            // copy it out, unsafely having a copy of the value on
            // the stack and in the vector at the same time
            let value = value_ptr.read();

            // shift everything to fill in that spot
            if index != 0 {
                start.copy_to(start.add(1), index);
            }

            self.len -= 1;
            value
        }
    }

    /// Removes an element from the vector and returns it.
    ///
    /// The removed element is replaced by the first element of the vector.
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
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut v = mut_bump_vec_rev![in &mut bump; "foo", "bar", "baz", "qux"];
    ///
    /// assert_eq!(v.swap_remove(1), "bar");
    /// assert_eq!(v, ["foo", "baz", "qux"]);
    ///
    /// assert_eq!(v.swap_remove(0), "foo");
    /// assert_eq!(v, ["baz", "qux"]);
    /// ```
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("swap_remove index (is {index}) should be < len (is {len})");
        }

        if index >= self.len {
            assert_failed(index, self.len);
        }

        unsafe {
            // We replace self[index] with the first element. Note that if the
            // bounds check above succeeds there must be a first element (which
            // can be self[index] itself).

            let start = self.as_mut_ptr();
            let value_ptr = start.add(index);
            let value = value_ptr.read();
            self.len -= 1;

            start.copy_to(value_ptr, 1);
            value
        }
    }

    #[must_use]
    #[inline]
    fn into_slice_ptr(self) -> NonNull<[T]> {
        let mut this = ManuallyDrop::new(self);

        if T::IS_ZST {
            return nonnull::slice_from_raw_parts(NonNull::dangling(), this.len());
        }

        if this.cap == 0 {
            // We didn't touch the bump, so no need to do anything.
            debug_assert_eq!(this.end, NonNull::<T>::dangling());
            return nonnull::slice_from_raw_parts(NonNull::<T>::dangling(), 0);
        }

        let end = this.end;
        let len = this.len;
        let cap = this.cap;
        unsafe { this.allocator.use_prepared_slice_allocation_rev(end, len, cap) }
    }

    /// # Safety
    ///
    /// `iterator` must satisfy the invariants of nightly's `TrustedLen`.
    // specific extend for `TrustedLen` iterators, called both by the specializations
    // and internal places where resolving specialization makes compilation slower
    unsafe fn extend_trusted<B: ErrorBehavior>(&mut self, iterator: impl Iterator<Item = T>) -> Result<(), B> {
        let (low, high) = iterator.size_hint();
        if let Some(additional) = high {
            debug_assert_eq!(
                low,
                additional,
                "TrustedLen iterator's size hint is not exact: {:?}",
                (low, high)
            );

            self.generic_reserve(additional)?;

            let ptr = self.end.as_ptr();
            let mut local_len = SetLenOnDrop::new(&mut self.len);

            iterator.for_each(move |element| {
                let dst = ptr.sub(local_len.current_len() + 1);

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
            Err(B::capacity_overflow())
        }
    }

    /// Returns the remaining spare capacity of the vector as a slice of
    /// `MaybeUninit<T>`.
    ///
    /// The returned slice can be used to fill the vector with data (e.g. by
    /// reading from a file) before marking the data as initialized using the
    /// [`set_len`] method.
    ///
    /// [`set_len`]: Self::set_len
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, MutBumpVecRev};
    /// # let mut bump: Bump = Bump::new();
    /// // Allocate vector big enough for 10 elements.
    /// let mut v = MutBumpVecRev::with_capacity_in(10, &mut bump);
    ///
    /// // Fill in the first 3 elements.
    /// let uninit = v.spare_capacity_mut();
    /// let len = uninit.len();
    /// uninit[len - 3].write(0);
    /// uninit[len - 2].write(1);
    /// uninit[len - 1].write(2);
    ///
    /// // Mark the first 3 elements of the vector as being initialized.
    /// unsafe {
    ///     v.set_len(3);
    /// }
    ///
    /// assert_eq!(&v, &[0, 1, 2]);
    /// ```
    #[inline]
    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        // Note:
        // This method is not implemented in terms of `split_at_spare_mut`,
        // to prevent invalidation of pointers to the buffer.
        unsafe {
            slice::from_raw_parts_mut(
                self.end.as_ptr().sub(self.capacity()).cast::<MaybeUninit<T>>(),
                self.capacity() - self.len(),
            )
        }
    }

    /// Returns vector content as a slice of `T`, along with the remaining spare
    /// capacity of the vector as a slice of `MaybeUninit<T>`.
    ///
    /// The returned spare capacity slice can be used to fill the vector with data
    /// (e.g. by reading from a file) before marking the data as initialized using
    /// the [`set_len`] method.
    ///
    /// [`set_len`]: Self::set_len
    ///
    /// Note that this is a low-level API, which should be used with care for
    /// optimization purposes. If you need to append data to a `MutBumpVecRev`
    /// you can use [`push`], [`extend`], `extend_from_slice`[`_copy`](MutBumpVecRev::extend_from_slice_copy)`/`[`_clone`](MutBumpVecRev::extend_from_within_clone),
    /// `extend_from_within`[`_copy`](MutBumpVecRev::extend_from_within_copy)`/`[`_clone`](MutBumpVecRev::extend_from_within_clone), [`insert`], [`resize`] or
    /// [`resize_with`], depending on your exact needs.
    ///
    /// [`push`]: Self::push
    /// [`extend`]: Self::extend
    /// [`insert`]: Self::insert
    /// [`append`]: Self::append
    /// [`resize`]: Self::resize
    /// [`resize_with`]: Self::resize_with
    #[inline]
    pub fn split_at_spare_mut(&mut self) -> (&mut [T], &mut [MaybeUninit<T>]) {
        // SAFETY:
        // - len is ignored and so never changed
        let (init, spare, _) = unsafe { self.split_at_spare_mut_with_len() };
        (init, spare)
    }

    /// Safety: changing returned .2 (&mut usize) is considered the same as calling `.set_len(_)`.
    ///
    /// This method provides unique access to all vec parts at once in `extend_from_within_clone`.
    unsafe fn split_at_spare_mut_with_len(&mut self) -> (&mut [T], &mut [MaybeUninit<T>], &mut usize) {
        let end = self.end.as_ptr();

        let spare_ptr = end.sub(self.cap);
        let spare_ptr = spare_ptr.cast::<MaybeUninit<T>>();
        let spare_len = self.cap - self.len;

        let initialized = slice::from_raw_parts_mut(self.as_mut_ptr(), self.len);
        let spare = slice::from_raw_parts_mut(spare_ptr, spare_len);

        (initialized, spare, &mut self.len)
    }

    mut_collection_method_allocator_stats!();
}

impl<'a, T, A: MutBumpAllocatorScope<'a>> MutBumpVecRev<T, A> {
    /// Turns this `MutBumpVecRev<T>` into a `BumpBox<[T]>`.
    ///
    /// Unused capacity does not take up space.<br/>
    /// When [bumping upwards](crate#bumping-upwards-or-downwards) this needs to shift all elements to the other end of the chunk.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_slice(self) -> BumpBox<'a, [T]> {
        unsafe { BumpBox::from_raw(self.into_slice_ptr()) }
    }

    /// Turns this `MutBumpVecRev<T>` into a `&[T]` that is live for this bump scope.
    ///
    /// Unused capacity does not take up space.<br/>
    /// When [bumping upwards](crate#bumping-upwards-or-downwards) this needs to shift all elements to the other end of the chunk.
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
}

impl<T, A, const N: usize> MutBumpVecRev<[T; N], A> {
    /// Takes a `MutBumpVecRev<[T; N]>` and flattens it into a `MutBumpVecRev<T>`.
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
    /// # use bump_scope::{Bump, mut_bump_vec_rev};
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec_rev![in &mut bump; [1, 2, 3], [4, 5, 6], [7, 8, 9]];
    /// assert_eq!(vec.pop(), Some([1, 2, 3]));
    ///
    /// let mut flattened = vec.into_flattened();
    /// assert_eq!(flattened.pop(), Some(4));
    /// ```
    #[must_use]
    pub fn into_flattened(self) -> MutBumpVecRev<T, A> {
        let (end, len, cap, allocator) = self.into_raw_parts();

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

        MutBumpVecRev {
            end: end.cast(),
            len: new_len,
            cap: new_cap,
            allocator,
            marker: PhantomData,
        }
    }
}

impl<T: Debug, A> Debug for MutBumpVecRev<T, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_slice(), f)
    }
}

impl<T, A> Deref for MutBumpVecRev<T, A> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, A> DerefMut for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, A: Default> Default for MutBumpVecRev<T, A> {
    fn default() -> Self {
        Self::new_in(A::default())
    }
}

impl<T, A, I: SliceIndex<[T]>> Index<I> for MutBumpVecRev<T, A> {
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self.as_slice(), index)
    }
}

impl<T, A, I: SliceIndex<[T]>> IndexMut<I> for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self.as_mut_slice(), index)
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<U, A: MutBumpAllocator> Extend<U> for MutBumpVecRev<U, A> {
    #[inline]
    fn extend<T: IntoIterator<Item = U>>(&mut self, iter: T) {
        let iter = iter.into_iter();

        self.reserve(iter.size_hint().0);

        for value in iter {
            self.push(value);
        }
    }
}

impl<T, A> Drop for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn drop(&mut self) {
        // MutBumpVecRev never actually moves a bump pointer.
        // It may force allocation of a new chunk, but it does not move the pointer within.
        // So we don't need to move the bump pointer when dropping.

        // If we want to reset the bump pointer to a previous chunk, we use a bump scope.
        // We could do it here, by resetting to the last non-empty chunk but that would require a loop.
        // Chunk allocations are supposed to be very rare, so this wouldn't be worth it.

        unsafe {
            let to_drop = nonnull::slice_from_raw_parts(self.as_non_null(), self.len);
            to_drop.as_ptr().drop_in_place();
        }
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<'t, T: Clone + 't, A: MutBumpAllocator> Extend<&'t T> for MutBumpVecRev<T, A> {
    #[inline]
    fn extend<I: IntoIterator<Item = &'t T>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        self.reserve(iter.size_hint().0);

        for value in iter {
            self.push(value.clone());
        }
    }
}

impl<T, A> IntoIterator for MutBumpVecRev<T, A> {
    type Item = T;
    type IntoIter = IntoIter<T, A>;

    /// If you need to use the allocator while iterating you can first turn it to a slice with [`into_slice`] or [`into_boxed_slice`].
    ///
    /// [`into_slice`]: Self::into_slice
    /// [`into_boxed_slice`]: Self::into_boxed_slice
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        let (end, len, _cap, allocator) = self.into_raw_parts();
        let start = unsafe { nonnull::sub(end, len) };
        let slice = nonnull::slice_from_raw_parts(start, len);
        unsafe { IntoIter::new(slice, allocator) }
    }
}

impl<'c, T, A> IntoIterator for &'c MutBumpVecRev<T, A> {
    type Item = &'c T;
    type IntoIter = slice::Iter<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'c, T, A> IntoIterator for &'c mut MutBumpVecRev<T, A> {
    type Item = &'c mut T;
    type IntoIter = slice::IterMut<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<T, A> AsRef<Self> for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T, A> AsMut<Self> for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<T, A> AsRef<[T]> for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, A> AsMut<[T]> for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T, A> Borrow<[T]> for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T, A> BorrowMut<[T]> for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T: Hash, A> Hash for MutBumpVecRev<T, A> {
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<T, A: MutBumpAllocator + Default> FromIterator<T> for MutBumpVecRev<T, A> {
    #[inline]
    #[track_caller]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_iter_in(iter, A::default())
    }
}
