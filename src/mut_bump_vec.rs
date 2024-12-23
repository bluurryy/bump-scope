mod into_iter;

pub use into_iter::IntoIter;

use crate::{
    error_behavior_generic_methods_allocation_failure, min_non_zero_cap, mut_collection_method_allocator_stats,
    owned_slice::{self, IntoOwnedSlice, OwnedSlice},
    polyfill::{nonnull, pointer, slice},
    raw_bump_box::RawBumpBox,
    raw_fixed_bump_vec::RawFixedBumpVec,
    BumpBox, ErrorBehavior, MutBumpAllocator, MutBumpAllocatorScope, NoDrop, SetLenOnDropByPtr, SizedTypeProperties, Stats,
};
use core::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    hash::Hash,
    iter,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut, Index, IndexMut, RangeBounds},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::{self, NonNull},
    slice::SliceIndex,
};

/// This is like [`vec!`] but allocates inside a bump allocator, returning a [`MutBumpVec`].
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
/// - Create an empty [`MutBumpVec`]:
/// ```
/// # use bump_scope::{ mut_bump_vec, Bump, MutBumpVec };
/// # let mut bump: Bump = Bump::new();
/// let vec: MutBumpVec<i32, _> = mut_bump_vec![in &mut bump];
/// assert!(vec.is_empty());
/// ```
///
/// - Create a [`MutBumpVec`] containing a given list of elements:
///
/// ```
/// # use bump_scope::{ mut_bump_vec, Bump };
/// # let mut bump: Bump = Bump::new();
/// let vec = mut_bump_vec![in &mut bump; 1, 2, 3];
/// assert_eq!(vec[0], 1);
/// assert_eq!(vec[1], 2);
/// assert_eq!(vec[2], 3);
/// ```
///
/// - Create a [`MutBumpVec`] from a given element and size:
///
/// ```
/// # use bump_scope::{ mut_bump_vec, Bump };
/// # let mut bump: Bump = Bump::new();
/// let vec = mut_bump_vec![in &mut bump; 1; 3];
/// assert_eq!(vec, [1, 1, 1]);
/// ```
///
/// Note that unlike array expressions this syntax supports all elements
/// which implement [`Clone`] and the number of elements doesn't have to be
/// a constant.
///
/// This will use `clone` to duplicate an expression, so one should be careful
/// using this with types having a nonstandard `Clone` implementation. For
/// example, `mut_bump_vec![in &mut bump; Rc::new(1); 5]` will create a vector of five references
/// to the same boxed integer value, not five references pointing to independently
/// boxed integers.
///
/// Also, note that `mut_bump_vec![in &mut bump; expr; 0]` is allowed, and produces an empty vector.
/// This will still evaluate `expr`, however, and immediately drop the resulting value, so
/// be mindful of side effects.
#[macro_export]
macro_rules! mut_bump_vec {
    [in $bump:expr] => {
        $crate::MutBumpVec::new_in($bump)
    };
    [in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::MutBumpVec::from_array_in([$($values),*], $bump)
    };
    [in $bump:expr; $value:expr; $count:expr] => {
        $crate::MutBumpVec::from_elem_in($value, $count, $bump)
    };
    [try in $bump:expr] => {
        Ok::<_, $crate::allocator_api2::alloc::AllocError>($crate::MutBumpVec::new_in($bump))
    };
    [try in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::MutBumpVec::try_from_array_in([$($values),*], $bump)
    };
    [try in $bump:expr; $value:expr; $count:expr] => {
        $crate::MutBumpVec::try_from_elem_in($value, $count, $bump)
    };
}

/// A type like [`BumpVec`](crate::BumpVec), optimized for a mutable bump allocator.
///
/// It has the advantage that it can assume the entire remaining chunk space as its capacity.
/// It also only needs to update the bump pointer when calling [`into_slice`](Self::into_slice) or [`into_boxed_slice`](Self::into_boxed_slice).
///
/// # Examples
///
/// This type can be used to allocate a slice, when `alloc_*` methods are too limiting:
/// ```
/// # use bump_scope::{ Bump, MutBumpVec };
/// # let mut bump: Bump = Bump::new();
/// let mut vec = MutBumpVec::new_in(&mut bump);
///
/// vec.push(1);
/// vec.push(2);
/// vec.push(3);
///
/// let slice: &[i32] = vec.into_slice();
///
/// assert_eq!(slice, [1, 2, 3]);
/// ```
//
// MutBumpVec never actually moves a bump pointer.
// It may force allocation of a new chunk, but it does not move the pointer within.
// So we don't need to move the bump pointer when dropping.
//
// If we want to reset the bump pointer to a previous chunk, we use a bump scope.
// We could do it here, by resetting to the last non-empty chunk but that would require a loop.
// Chunk allocations are supposed to be very rare, so this wouldn't be worth it.
#[repr(C)]
pub struct MutBumpVec<T, A> {
    fixed: RawFixedBumpVec<T>,
    pub(crate) allocator: A,
}

impl<T: UnwindSafe, A: UnwindSafe> UnwindSafe for MutBumpVec<T, A> {}
impl<T: RefUnwindSafe, A: RefUnwindSafe> RefUnwindSafe for MutBumpVec<T, A> {}

impl<T, A> Deref for MutBumpVec<T, A> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { self.fixed.cook_ref() }
    }
}

impl<T, A> DerefMut for MutBumpVec<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.fixed.cook_mut() }
    }
}

impl<T, A> MutBumpVec<T, A> {
    /// Constructs a new empty `MutBumpVec<T>`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, MutBumpVec };
    /// # let mut bump: Bump = Bump::new();
    /// # #[allow(unused_mut)]
    /// let mut vec = MutBumpVec::<i32, _>::new_in(&mut bump);
    /// # let _ = vec;
    /// ```
    #[inline]
    pub fn new_in(allocator: A) -> Self {
        Self {
            fixed: RawFixedBumpVec::EMPTY,
            allocator,
        }
    }

    /// Returns the total number of elements the vector can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, MutBumpVec };
    /// # let mut bump: Bump = Bump::new();
    /// let vec = MutBumpVec::<i32, _>::with_capacity_in(2048, &mut bump);
    /// assert!(vec.capacity() >= 2048);
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.fixed.capacity
    }

    /// Returns the number of elements in the vector, also referred to
    /// as its 'length'.
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.fixed.len()
    }

    /// Returns `true` if the vector contains no elements.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.fixed.len() == 0
    }

    /// Removes the last element from a vector and returns it, or [`None`] if it
    /// is empty.
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        unsafe { self.fixed.cook_mut() }.pop()
    }

    /// Clears the vector, removing all values.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    /// vec.clear();
    /// assert!(vec.is_empty());
    /// ```
    #[inline(always)]
    pub fn clear(&mut self) {
        unsafe { self.fixed.cook_mut() }.clear();
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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3, 4, 5];
    /// vec.truncate(2);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current
    /// length:
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    /// vec.truncate(8);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    /// vec.truncate(0);
    /// assert_eq!(vec, []);
    /// ```
    ///
    /// [`clear`]: Self::clear
    /// [`drain`]: Self::drain
    pub fn truncate(&mut self, len: usize) {
        unsafe { self.fixed.cook_mut() }.truncate(len);
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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// let mut v = mut_bump_vec![in &mut bump; 1, 2, 3];
    /// assert_eq!(v.remove(1), 2);
    /// assert_eq!(v, [1, 3]);
    /// ```
    #[track_caller]
    pub fn remove(&mut self, index: usize) -> T {
        unsafe { self.fixed.cook_mut() }.remove(index)
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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut v = mut_bump_vec![in &mut bump; "foo", "bar", "baz", "qux"];
    ///
    /// assert_eq!(v.swap_remove(1), "bar");
    /// assert_eq!(v, ["foo", "qux", "baz"]);
    ///
    /// assert_eq!(v.swap_remove(0), "foo");
    /// assert_eq!(v, ["baz", "qux"]);
    /// ```
    ///
    /// [`remove`]: Self::remove
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        unsafe { self.fixed.cook_mut() }.swap_remove(index)
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    #[must_use]
    #[inline(always)]
    pub const fn as_slice(&self) -> &[T] {
        unsafe { self.fixed.cook_ref() }.as_slice()
    }

    /// Extracts a mutable slice containing the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { self.fixed.cook_mut() }.as_mut_slice()
    }

    /// Returns a raw pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        self.fixed.as_ptr()
    }

    /// Returns an unsafe mutable pointer to slice, or a dangling
    /// raw pointer valid for zero sized reads.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.fixed.as_mut_ptr()
    }

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_ptr(&self) -> NonNull<T> {
        self.fixed.as_non_null_ptr()
    }

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_slice(&self) -> NonNull<[T]> {
        self.fixed.initialized.ptr
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, value: T) {
        unsafe { self.fixed.cook_mut() }.push_unchecked(value);
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn push_with_unchecked(&mut self, f: impl FnOnce() -> T) {
        unsafe { self.fixed.cook_mut() }.push_with_unchecked(f);
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type. Normally changing the length of a vector
    /// is done using one of the safe operations instead, such as
    /// [`resize`], [`truncate`], [`extend`], or [`clear`].
    ///
    /// # Safety
    /// - `new_len` must be less than or equal to the [`capacity`].
    /// - The elements at `old_len..new_len` must be initialized.
    ///
    /// [`resize`]: Self::resize
    /// [`truncate`]: Self::truncate
    /// [`extend`]: Self::extend
    /// [`clear`]: Self::clear
    /// [`capacity`]: Self::capacity
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.fixed.set_len(new_len);
    }

    #[inline]
    pub(crate) unsafe fn inc_len(&mut self, amount: usize) {
        unsafe { self.fixed.cook_mut() }.inc_len(amount);
    }
}

impl<T, A: MutBumpAllocator> MutBumpVec<T, A> {
    error_behavior_generic_methods_allocation_failure! {
        /// Constructs a new empty vector with at least the specified capacity
        /// with the provided bump allocator.
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
        /// [Capacity and reallocation]: alloc::vec::Vec#capacity-and-reallocation
        impl
        for fn with_capacity_in
        for fn try_with_capacity_in
        #[inline]
        use fn generic_with_capacity_in(capacity: usize, allocator: A) -> Self {
            let mut allocator = allocator;

            if T::IS_ZST {
                return Ok(Self {
                    fixed: RawFixedBumpVec::EMPTY,
                    allocator,
                });
            }

            if capacity == 0 {
                return Ok(Self {
                    fixed: RawFixedBumpVec::EMPTY,
                    allocator,
                });
            }

            Ok(Self {
                fixed: unsafe { RawFixedBumpVec::prepare_allocation(&mut allocator, capacity)? },
                allocator,
            })
        }

        /// Constructs a new `MutBumpVec<T>` and pushes `value` `count` times.
        impl
        for fn from_elem_in
        for fn try_from_elem_in
        #[inline]
        use fn generic_from_elem_in(value: T, count: usize, allocator: A) -> Self
        where {
            T: Clone
        } in {
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

        /// Constructs a new `MutBumpVec<T>` from a `[T; N]`.
        impl
        for fn from_array_in
        for fn try_from_array_in
        #[inline]
        use fn generic_from_array_in<{const N: usize}>(array: [T; N], allocator: A) -> Self {
            #![allow(clippy::needless_pass_by_value)]
            #![allow(clippy::needless_pass_by_ref_mut)]

            let array = ManuallyDrop::new(array);
            let mut allocator = allocator;

            if T::IS_ZST {
                return Ok(Self {
                    fixed: RawFixedBumpVec { initialized: unsafe { RawBumpBox::from_ptr(nonnull::slice_from_raw_parts(NonNull::dangling(), N)) }, capacity: usize::MAX },
                    allocator,
                });
            }

            if N == 0 {
                return Ok(Self {
                    fixed: RawFixedBumpVec::EMPTY,
                    allocator,
                });
            }

            let mut fixed = unsafe { RawFixedBumpVec::prepare_allocation(&mut allocator, N)? };

            let src = array.as_ptr();
            let dst = fixed.initialized.ptr.cast::<T>().as_ptr();

            unsafe {
                ptr::copy_nonoverlapping(src, dst, N);
                fixed.set_len(N);
            }

            Ok(Self { fixed, allocator })
        }

        /// Create a new [`MutBumpVec`] whose elements are taken from an iterator and allocated in the given `bump`.
        ///
        /// This is behaviorally identical to [`FromIterator::from_iter`].
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, MutBumpVec };
        /// # let mut bump: Bump = Bump::new();
        /// let vec = MutBumpVec::from_iter_in([1, 2, 3], &mut bump);
        /// assert_eq!(vec, [1, 2, 3]);
        /// ```
        for fn from_iter_in
        do examples
        /// ```
        /// # use bump_scope::{ Bump, MutBumpVec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let vec = MutBumpVec::try_from_iter_in([1, 2, 3], &mut bump)?;
        /// assert_eq!(vec, [1, 2, 3]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_from_iter_in
        #[inline]
        use fn generic_from_iter_in<{I}>(iter: I, allocator: A) -> Self
        where {
            I: IntoIterator<Item = T>
        } in {
            let iter = iter.into_iter();
            let capacity = iter.size_hint().0;
            let mut vec = Self::generic_with_capacity_in(capacity, allocator)?;

            for value in iter {
                vec.generic_push(value)?;
            }

            Ok(vec)
        }

        /// Appends an element to the back of a collection.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec![in &mut bump; 1, 2];
        /// vec.push(3);
        /// assert_eq!(vec, [1, 2, 3]);
        /// ```
        for fn push
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut vec = mut_bump_vec![try in &mut bump; 1, 2]?;
        /// vec.try_push(3)?;
        /// assert_eq!(vec, [1, 2, 3]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_push
        #[inline]
        use fn generic_push(&mut self, value: T) {
            self.generic_push_with(|| value)
        }

        /// Appends an element to the back of a collection.
        impl
        for fn push_with
        for fn try_push_with
        #[inline]
        use fn generic_push_with(&mut self, f: impl FnOnce() -> T) {
            self.generic_reserve_one()?;
            unsafe {
                self.push_with_unchecked(f);
            }
            Ok(())
        }

        /// Inserts an element at position `index` within the vector, shifting all elements after it to the right.
        do panics
        /// Panics if `index > len`.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
        /// vec.insert(1, 4);
        /// assert_eq!(vec, [1, 4, 2, 3]);
        /// vec.insert(4, 5);
        /// assert_eq!(vec, [1, 4, 2, 3, 5]);
        /// ```
        for fn insert
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
        /// vec.try_insert(1, 4)?;
        /// assert_eq!(vec, [1, 4, 2, 3]);
        /// vec.try_insert(4, 5)?;
        /// assert_eq!(vec, [1, 4, 2, 3, 5]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_insert
        #[inline]
        use fn generic_insert(&mut self, index: usize, element: T) {
            #[cold]
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

        /// Copies and appends all elements in a slice to the `MutBumpVec`.
        ///
        /// Iterates over the `slice`, copies each element, and then appends
        /// it to this `MutBumpVec`. The `slice` is traversed in-order.
        ///
        /// Note that this function is same as [`extend`] except that it is
        /// specialized to work with copyable slices instead.
        ///
        /// [`extend`]: Self::extend
        impl
        for fn extend_from_slice_copy
        for fn try_extend_from_slice_copy
        #[inline]
        use fn generic_extend_from_slice_copy(&mut self, slice: &[T])
        where {
            T: Copy
        } in {
            unsafe { self.extend_by_copy_nonoverlapping(slice) }
        }

        /// Clones and appends all elements in a slice to the `MutBumpVec`.
        ///
        /// Iterates over the `slice`, clones each element, and then appends
        /// it to this `MutBumpVec`. The `slice` is traversed in-order.
        ///
        /// Note that this function is same as [`extend`] except that it is
        /// specialized to work with slices instead.
        ///
        /// [`extend`]: Self::extend
        impl
        for fn extend_from_slice_clone
        for fn try_extend_from_slice_clone
        #[inline]
        use fn generic_extend_from_slice_clone(&mut self, slice: &[T])
        where {
            T: Clone
        } in {
            self.generic_reserve(slice.len())?;

            unsafe {
                let mut ptr = self.as_mut_ptr().add(self.len());

                for value in slice {
                    pointer::write_with(ptr, || value.clone());
                    ptr = ptr.add(1);
                    self.inc_len(1);
                }
            }

            Ok(())
        }

        /// Appends all elements in an array to the `MutBumpVec`.
        ///
        /// Iterates over the `array`, copies each element, and then appends
        /// it to this `MutBumpVec`. The `array` is traversed in-order.
        ///
        /// Note that this function is same as [`extend`] except that it is
        /// specialized to work with arrays instead.
        ///
        /// [`extend`]: Self::extend
        #[allow(clippy::needless_pass_by_value)]
        impl
        #[deprecated = "use `append` instead"]
        for fn extend_from_array
        #[deprecated = "use `append` instead"]
        for fn try_extend_from_array
        #[inline]
        use fn generic_extend_from_array<{const N: usize}>(&mut self, array: [T; N]) {
            unsafe { self.extend_by_copy_nonoverlapping(&array) }
        }

        /// Copies elements from `src` range to the end of the vector.
        do panics
        /// Panics if the starting point is greater than the end point or if
        /// the end point is greater than the length of the vector.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec![in &mut bump; 0, 1, 2, 3, 4];
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
        for fn extend_from_within_copy
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut vec = mut_bump_vec![try in &mut bump; 0, 1, 2, 3, 4]?;
        ///
        /// vec.try_extend_from_within_copy(2..)?;
        /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4]);
        ///
        /// vec.try_extend_from_within_copy(..2)?;
        /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1]);
        ///
        /// vec.try_extend_from_within_copy(4..8)?;
        /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1, 4, 2, 3, 4]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_extend_from_within_copy
        #[inline]
        use fn generic_extend_from_within_copy<{R}>(&mut self, src: R)
        where {
            T: Copy,
            R: RangeBounds<usize>,
        } in {
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
        ///
        /// Panics if the starting point is greater than the end point or if
        /// the end point is greater than the length of the vector.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec![in &mut bump; 0, 1, 2, 3, 4];
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
        for fn extend_from_within_clone
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut vec = mut_bump_vec![try in &mut bump; 0, 1, 2, 3, 4]?;
        ///
        /// vec.try_extend_from_within_clone(2..)?;
        /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4]);
        ///
        /// vec.try_extend_from_within_clone(..2)?;
        /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1]);
        ///
        /// vec.try_extend_from_within_clone(4..8)?;
        /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1, 4, 2, 3, 4]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_extend_from_within_clone
        #[inline]
        use fn generic_extend_from_within_clone<{R}>(&mut self, src: R)
        where {
            T: Clone,
            R: RangeBounds<usize>,
        } in {
            let range = slice::range(src, ..self.len());
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

        #[cfg(feature = "zerocopy")]
        /// Extends this vector by pushing `additional` new items onto the end.
        /// The new items are initialized with zeroes.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
        /// vec.extend_zeroed(2);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// ```
        for fn extend_zeroed
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
        /// vec.try_extend_zeroed(2)?;
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_extend_zeroed
        #[inline]
        use fn generic_extend_zeroed(&mut self, additional: usize)
        where {
            T: zerocopy::FromZeros
        } in {
            self.generic_reserve(additional)?;

            unsafe {
                let ptr = self.as_mut_ptr();
                let len = self.len();

                ptr.add(len).write_bytes(0, additional);
                self.set_len(len + additional);
            }

            Ok(())
        }

        /// Reserves capacity for at least `additional` more elements to be inserted
        /// in the given `MutBumpVec<T>`. The collection may reserve more space to
        /// speculatively avoid frequent reallocations. After calling `reserve`,
        /// capacity will be greater than or equal to `self.len() + additional`.
        /// Does nothing if capacity is already sufficient.
        do panics
        /// Panics if the new capacity exceeds `isize::MAX` bytes.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec![in &mut bump; 1];
        /// vec.reserve(10);
        /// assert!(vec.capacity() >= 11);
        /// ```
        for fn reserve
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut vec = mut_bump_vec![try in &mut bump; 1]?;
        /// vec.try_reserve(10)?;
        /// assert!(vec.capacity() >= 11);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_reserve
        #[inline]
        use fn generic_reserve(&mut self, additional: usize) {
            if additional > (self.capacity() - self.len()) {
                self.generic_grow_amortized(additional)?;
            }

            Ok(())
        }

        /// Reserves the minimum capacity for at least `additional` more elements to
        /// be inserted in the given `MutBumpVec<T>`. Unlike [`reserve`], this will not
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
        do panics
        /// Panics if the new capacity exceeds `isize::MAX` bytes.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec![in &mut bump; 1];
        /// vec.reserve_exact(10);
        /// assert!(vec.capacity() >= 11);
        /// ```
        for fn reserve_exact
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut vec = mut_bump_vec![try in &mut bump; 1]?;
        /// vec.try_reserve_exact(10)?;
        /// assert!(vec.capacity() >= 11);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_reserve_exact
        #[inline]
        use fn generic_reserve_exact(&mut self, additional: usize) {
            if additional > (self.capacity() - self.len()) {
                self.generic_grow_exact(additional)?;
            }

            Ok(())
        }

        /// Resizes the `MutBumpVec` in-place so that `len` is equal to `new_len`.
        ///
        /// If `new_len` is greater than `len`, the `MutBumpVec` is extended by the
        /// difference, with each additional slot filled with `value`.
        /// If `new_len` is less than `len`, the `MutBumpVec` is simply truncated.
        ///
        /// This method requires `T` to implement [`Clone`],
        /// in order to be able to clone the passed value.
        /// If you need more flexibility (or want to rely on [`Default`] instead of
        /// [`Clone`]), use [`resize_with`].
        /// If you only need to resize to a smaller size, use [`truncate`].
        ///
        /// [`resize_with`]: Self::resize_with
        /// [`truncate`]: Self::truncate
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec![in &mut bump; "hello"];
        /// vec.resize(3, "world");
        /// assert_eq!(vec, ["hello", "world", "world"]);
        /// drop(vec);
        ///
        /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3, 4];
        /// vec.resize(2, 0);
        /// assert_eq!(vec, [1, 2]);
        /// ```
        for fn resize
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut vec = mut_bump_vec![try in &mut bump; "hello"]?;
        /// vec.try_resize(3, "world")?;
        /// assert_eq!(vec, ["hello", "world", "world"]);
        /// drop(vec);
        ///
        /// let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3, 4]?;
        /// vec.try_resize(2, 0)?;
        /// assert_eq!(vec, [1, 2]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_resize
        #[inline]
        use fn generic_resize(&mut self, new_len: usize, value: T)
        where { T: Clone } in
        {
            let len = self.len();

            if new_len > len {
                self.extend_with(new_len - len, value)
            } else {
                self.truncate(new_len);
                Ok(())
            }
        }

        /// Resizes the `MutBumpVec` in-place so that `len` is equal to `new_len`.
        ///
        /// If `new_len` is greater than `len`, the `MutBumpVec` is extended by the
        /// difference, with each additional slot filled with the result of
        /// calling the closure `f`. The return values from `f` will end up
        /// in the `MutBumpVec` in the order they have been generated.
        ///
        /// If `new_len` is less than `len`, the `MutBumpVec` is simply truncated.
        ///
        /// This method uses a closure to create new values on every push. If
        /// you'd rather [`Clone`] a given value, use [`MutBumpVec::resize`]. If you
        /// want to use the [`Default`] trait to generate values, you can
        /// pass [`Default::default`] as the second argument.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
        /// vec.resize_with(5, Default::default);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// drop(vec);
        ///
        /// let mut vec = mut_bump_vec![in &mut bump];
        /// let mut p = 1;
        /// vec.resize_with(4, || { p *= 2; p });
        /// assert_eq!(vec, [2, 4, 8, 16]);
        /// ```
        for fn resize_with
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
        /// vec.try_resize_with(5, Default::default)?;
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// drop(vec);
        ///
        /// let mut vec = mut_bump_vec![try in &mut bump]?;
        /// let mut p = 1;
        /// vec.try_resize_with(4, || { p *= 2; p })?;
        /// assert_eq!(vec, [2, 4, 8, 16]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_resize_with
        #[inline]
        use fn generic_resize_with<{F}>(&mut self, new_len: usize, f: F)
        where {
            F: FnMut() -> T,
        } in {
            let len = self.len();
            if new_len > len {
                unsafe { self.extend_trusted(iter::repeat_with(f).take(new_len - len)) }
            } else {
                self.truncate(new_len);
                Ok(())
            }
        }

        #[cfg(feature = "zerocopy")]
        /// Resizes this vector in-place so that `len` is equal to `new_len`.
        ///
        /// If `new_len` is greater than `len`, the vector is extended by the
        /// difference, with each additional slot filled with `value`.
        /// If `new_len` is less than `len`, the vector is simply truncated.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// {
        ///     let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
        ///     vec.resize_zeroed(5);
        ///     assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// }
        ///
        /// {
        ///    let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
        ///    vec.resize_zeroed(2);
        ///    assert_eq!(vec, [1, 2]);
        /// }
        /// ```
        for fn resize_zeroed
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// {
        ///     let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
        ///     vec.try_resize_zeroed(5)?;
        ///     assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// }
        ///
        /// {
        ///    let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
        ///    vec.try_resize_zeroed(2)?;
        ///    assert_eq!(vec, [1, 2]);
        /// }
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_resize_zeroed
        #[inline]
        use fn generic_resize_zeroed(&mut self, new_len: usize)
        where {
            T: zerocopy::FromZeros
        } in {
            let len = self.len();

            if new_len > len {
                self.generic_extend_zeroed(new_len - len)
            } else {
                self.truncate(new_len);
                Ok(())
            }
        }

        /// Moves all the elements of `other` into `self`, leaving `other` empty.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// // needs a scope because of lifetime shenanigans
        /// let bump = bump.as_mut_scope();
        /// let mut slice = bump.alloc_slice_copy(&[4, 5, 6]);
        /// let mut vec = mut_bump_vec![in bump; 1, 2, 3];
        /// vec.append(&mut slice);
        /// assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
        /// assert_eq!(slice, []);
        /// ```
        for fn append
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::try_new()?;
        /// // needs a scope because of lifetime shenanigans
        /// let bump = bump.as_mut_scope();
        /// let mut slice = bump.try_alloc_slice_copy(&[4, 5, 6])?;
        /// let mut vec = mut_bump_vec![try in bump; 1, 2, 3]?;
        /// vec.try_append(&mut slice)?;
        /// assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
        /// assert_eq!(slice, []);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_append
        #[inline]
        use fn generic_append(&mut self, other: impl IntoOwnedSlice<Item = T>) {
            unsafe {
                let mut owned_slice = other.into_owned_slice();

                let slice = owned_slice.owned_slice_ptr();
                self.generic_reserve(slice.len())?;

                let src = slice.cast::<T>().as_ptr();
                let dst = self.as_mut_ptr().add(self.len());
                ptr::copy_nonoverlapping(src, dst, slice.len());

                owned_slice.take_owned_slice();
                self.inc_len(slice.len());
                Ok(())
            }
        }
    }

    /// Returns a vector of the same size as `self`, with function `f` applied to each element in order.
    ///
    /// This function only compiles when `U`s size and alignment is less or equal to `T`'s or if `U` has a size of 0.
    ///
    /// # Examples
    /// Mapping to a type with an equal alignment and size:
    /// ```
    /// # use bump_scope::{ Bump, MutBumpVec };
    /// # use core::num::NonZero;
    /// # let mut bump: Bump = Bump::new();
    /// let a = MutBumpVec::from_iter_in([0, 1, 2], &mut bump);
    /// let b = a.map_in_place(NonZero::new);
    /// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
    /// ```
    ///
    /// Mapping to a type with a smaller alignment and size:
    /// ```
    /// # use bump_scope::{ Bump, MutBumpVec };
    /// # let mut bump: Bump = Bump::new();
    /// let a: MutBumpVec<u32, _> = MutBumpVec::from_iter_in([0, 1, 2], &mut bump);
    /// let a_capacity = a.capacity();
    ///
    /// let b: MutBumpVec<u16, _> = a.map_in_place(|i| i as u16);
    /// assert_eq!(b.capacity(), a_capacity * 2);
    ///
    /// assert_eq!(b, [0, 1, 2]);
    /// ```
    ///
    /// Mapping to a type with a higher alignment or size won't compile:
    /// ```compile_fail,E0080
    /// # use bump_scope::{ Bump, MutBumpVec };
    /// # let mut bump: Bump = Bump::new();
    /// let a: MutBumpVec<u16, _> = MutBumpVec::from_iter_in([0, 1, 2], &mut bump);
    /// let b: MutBumpVec<u32, _> = a.map_in_place(|i| i as u32);
    /// # _ = b;
    /// ```
    pub fn map_in_place<U>(self, f: impl FnMut(T) -> U) -> MutBumpVec<U, A> {
        let MutBumpVec { fixed, allocator } = self;
        let fixed = unsafe { RawFixedBumpVec::from_cooked(fixed.cook().map_in_place(f)) };
        MutBumpVec { fixed, allocator }
    }

    /// Extend the vector by `n` clones of value.
    fn extend_with<B: ErrorBehavior>(&mut self, n: usize, value: T) -> Result<(), B>
    where
        T: Clone,
    {
        self.generic_reserve(n)?;
        unsafe {
            self.fixed.cook_mut().extend_with_unchecked(n, value);
        }
        Ok(())
    }

    #[inline(always)]
    unsafe fn extend_by_copy_nonoverlapping<E: ErrorBehavior>(&mut self, other: *const [T]) -> Result<(), E> {
        let len = pointer::len(other);
        self.generic_reserve(len)?;

        let src = other.cast::<T>();
        let dst = self.as_mut_ptr().add(self.len());
        ptr::copy_nonoverlapping(src, dst, len);

        self.inc_len(len);
        Ok(())
    }

    #[inline]
    fn generic_reserve_one<E: ErrorBehavior>(&mut self) -> Result<(), E> {
        if self.capacity() == self.len() {
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
        let Self { fixed, allocator } = self;
        fixed.grow_prepared_allocation(allocator, new_capacity)
    }

    #[must_use]
    #[inline]
    fn into_slice_ptr(self) -> NonNull<[T]> {
        let mut this = ManuallyDrop::new(self);

        unsafe {
            if T::IS_ZST {
                return nonnull::slice_from_raw_parts(NonNull::dangling(), this.len());
            }

            if this.capacity() == 0 {
                // We didn't touch the allocator, so no need to do anything.
                debug_assert_eq!(this.as_non_null_ptr(), NonNull::<T>::dangling());
                return nonnull::slice_from_raw_parts(NonNull::<T>::dangling(), 0);
            }

            let ptr = this.as_non_null_ptr();
            let len = this.len();
            let cap = this.capacity();

            this.allocator.use_prepared_slice_allocation(ptr, len, cap)
        }
    }

    /// # Safety
    ///
    /// `iterator` must satisfy the invariants of nightly's `TrustedLen`.
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

            let ptr = self.as_mut_ptr();
            let mut local_len = SetLenOnDropByPtr::new(&mut self.fixed.initialized.ptr);

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
            Err(B::capacity_overflow())
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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3, 4];
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
    #[allow(clippy::pedantic)]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        unsafe { self.fixed.cook_mut() }.retain(f)
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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump1: Bump = Bump::new();
    /// # let bump2: Bump = Bump::new();
    /// #
    /// let mut v = mut_bump_vec![in &mut bump1; 1, 2, 3];
    /// let u = bump2.alloc_iter(v.drain(1..));
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
        unsafe { self.fixed.cook_mut() }.drain(range)
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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let some_predicate = |x: &mut i32| { *x == 2 || *x == 3 || *x == 6 };
    /// # let mut bump: Bump = Bump::new();
    /// # let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3, 4, 5, 6];
    /// let mut i = 0;
    /// while i < vec.len() {
    ///     if some_predicate(&mut vec[i]) {
    ///         let val = vec.remove(i);
    ///         // your code here
    /// #       let _ = val;
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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// let mut numbers = mut_bump_vec![in &mut bump; 1, 2, 3, 4, 5, 6, 8, 9, 11, 13, 14, 15];
    ///
    /// let evens = numbers.extract_if(|x| *x % 2 == 0).collect::<Vec<_>>();
    /// let odds = numbers;
    ///
    /// assert_eq!(evens, [2, 4, 6, 8, 14]);
    /// assert_eq!(odds, [1, 3, 5, 9, 11, 13, 15]);
    /// ```
    ///
    /// [`retain`]: Self::retain
    pub fn extract_if<F>(&mut self, filter: F) -> owned_slice::ExtractIf<T, F>
    where
        F: FnMut(&mut T) -> bool,
    {
        unsafe { self.fixed.cook_mut() }.extract_if(filter)
    }

    /// Removes consecutive repeated elements in the vector according to the
    /// [`PartialEq`] trait implementation.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 2, 3, 2];
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
        unsafe { self.fixed.cook_mut() }.dedup();
    }

    /// Removes all but the first of consecutive elements in the vector that resolve to the same
    /// key.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec![in &mut bump; 10, 20, 21, 30, 20];
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
        unsafe { self.fixed.cook_mut() }.dedup_by_key(key);
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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec![in &mut bump; "foo", "bar", "Bar", "baz", "bar"];
    ///
    /// vec.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    ///
    /// assert_eq!(vec, ["foo", "bar", "baz", "bar"]);
    /// ```
    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        unsafe { self.fixed.cook_mut() }.dedup_by(same_bucket);
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
    /// # use bump_scope::{ Bump, MutBumpVec };
    /// # let mut bump: Bump = Bump::new();
    /// // Allocate vector big enough for 10 elements.
    /// let mut v = MutBumpVec::with_capacity_in(10, &mut bump);
    ///
    /// // Fill in the first 3 elements.
    /// let uninit = v.spare_capacity_mut();
    /// uninit[0].write(0);
    /// uninit[1].write(1);
    /// uninit[2].write(2);
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
                self.as_mut_ptr().add(self.len()).cast::<MaybeUninit<T>>(),
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
    /// optimization purposes. If you need to append data to a `MutBumpVec`
    /// you can use [`push`], [`extend`], `extend_from_slice`[`_copy`](MutBumpVec::extend_from_slice_copy)`/`[`_clone`](MutBumpVec::extend_from_within_clone),
    /// `extend_from_within`[`_copy`](MutBumpVec::extend_from_within_copy)`/`[`_clone`](MutBumpVec::extend_from_within_clone), [`insert`], [`resize`] or
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
        let ptr = self.as_mut_ptr();

        // SAFETY:
        // - `ptr` is guaranteed to be valid for `self.len` elements
        // - but the allocation extends out to `self.buf.capacity()` elements, possibly
        // uninitialized
        let spare_ptr = unsafe { ptr.add(self.len()) };
        let spare_ptr = spare_ptr.cast::<MaybeUninit<T>>();
        let spare_len = self.capacity() - self.len();

        // SAFETY:
        // - `ptr` is guaranteed to be valid for `self.len` elements
        // - `spare_ptr` is pointing one element past the buffer, so it doesn't overlap with `initialized`
        unsafe {
            let initialized = slice::from_raw_parts_mut(ptr, self.len());
            let spare = slice::from_raw_parts_mut(spare_ptr, spare_len);

            (initialized, spare)
        }
    }

    mut_collection_method_allocator_stats!();
}

impl<'a, T, A: MutBumpAllocatorScope<'a>> MutBumpVec<T, A> {
    /// Turns this `MutBumpVec<T>` into a `BumpBox<[T]>`.
    ///
    /// Unused capacity does not take up space.<br/>
    /// When [bumping downwards](crate#bumping-upwards-or-downwards) this needs to shift all elements to the other end of the chunk.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_slice(self) -> BumpBox<'a, [T]> {
        unsafe { BumpBox::from_raw(self.into_slice_ptr()) }
    }

    /// Turns this `MutBumpVec<T>` into a `&[T]` that is live for this bump scope.
    ///
    /// Unused capacity does not take up space.<br/>
    /// When [bumping downwards](crate#bumping-upwards-or-downwards) this needs to shift all elements to the other end of the chunk.
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

impl<T, const N: usize, A> MutBumpVec<[T; N], A> {
    /// Takes a `MutBumpVec<[T; N]>` and flattens it into a `MutBumpVec<T>`.
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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec![in &mut bump; [1, 2, 3], [4, 5, 6], [7, 8, 9]];
    /// assert_eq!(vec.pop(), Some([7, 8, 9]));
    ///
    /// let mut flattened = vec.into_flattened();
    /// assert_eq!(flattened.pop(), Some(6));
    /// ```
    #[must_use]
    pub fn into_flattened(self) -> MutBumpVec<T, A> {
        let Self { fixed, allocator } = self;
        let fixed = unsafe { RawFixedBumpVec::from_cooked(fixed.cook().into_flattened()) };
        MutBumpVec { fixed, allocator }
    }
}

impl<T: Debug, A> Debug for MutBumpVec<T, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_slice(), f)
    }
}

impl<T, A: MutBumpAllocator, I: SliceIndex<[T]>> Index<I> for MutBumpVec<T, A> {
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self.as_slice(), index)
    }
}

impl<T, A: MutBumpAllocator, I: SliceIndex<[T]>> IndexMut<I> for MutBumpVec<T, A> {
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self.as_mut_slice(), index)
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<T, A: MutBumpAllocator> Extend<T> for MutBumpVec<T, A> {
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
impl<'t, T: Clone + 't, A: MutBumpAllocator> Extend<&'t T> for MutBumpVec<T, A> {
    #[inline]
    fn extend<I: IntoIterator<Item = &'t T>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        self.reserve(iter.size_hint().0);

        for value in iter {
            self.push(value.clone());
        }
    }
}

impl<T, A> IntoIterator for MutBumpVec<T, A> {
    type Item = T;
    type IntoIter = IntoIter<T, A>;

    /// If you need to use the allocator while iterating you can first turn it to a slice with [`into_slice`] or [`into_boxed_slice`].
    ///
    /// [`into_slice`]: Self::into_slice
    /// [`into_boxed_slice`]: Self::into_boxed_slice
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        let Self { fixed, allocator } = self;
        let slice = fixed.initialized.into_ptr();
        unsafe { IntoIter::new(slice, allocator) }
    }
}

impl<'c, T, A> IntoIterator for &'c MutBumpVec<T, A> {
    type Item = &'c T;
    type IntoIter = slice::Iter<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'c, T, A> IntoIterator for &'c mut MutBumpVec<T, A> {
    type Item = &'c mut T;
    type IntoIter = slice::IterMut<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<T, A> AsRef<[T]> for MutBumpVec<T, A> {
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, A> AsMut<[T]> for MutBumpVec<T, A> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T, A> Borrow<[T]> for MutBumpVec<T, A> {
    #[inline(always)]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T, A> BorrowMut<[T]> for MutBumpVec<T, A> {
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T: Hash, A> Hash for MutBumpVec<T, A> {
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

/// Returns [`ErrorKind::OutOfMemory`](std::io::ErrorKind::OutOfMemory) when allocations fail.
#[cfg(feature = "std")]
impl<A: MutBumpAllocator> std::io::Write for MutBumpVec<u8, A> {
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
