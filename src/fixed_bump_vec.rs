use crate::{
    error_behavior_generic_methods_if,
    polyfill::{self, nonnull, pointer, slice},
    set_len_on_drop_by_ptr::SetLenOnDropByPtr,
    BaseAllocator, BumpBox, BumpScope, BumpVec, Drain, ErrorBehavior, ExtractIf, IntoIter, MinimumAlignment, NoDrop,
    SizedTypeProperties, SupportedMinimumAlignment,
};
use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    hash::Hash,
    iter,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut, Index, IndexMut, RangeBounds},
    ptr::{self, NonNull},
    slice::SliceIndex,
};

/// A [`BumpVec`] but with a fixed capacity.
///
/// It can be constructed with [`alloc_fixed_vec`] or from a `BumpBox` via [`from_init`] or [`from_uninit`].
///
/// # Examples
/// ```
/// use bump_scope::Bump;
/// let mut bump: Bump = Bump::new();
/// let mut vec = bump.alloc_fixed_vec(3);
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
/// [`alloc_fixed_vec`]: crate::Bump::alloc_fixed_vec
/// [`from_uninit`]: FixedBumpVec::from_uninit
/// [`from_init`]: FixedBumpVec::from_init
pub struct FixedBumpVec<'a, T> {
    pub(crate) initialized: BumpBox<'a, [T]>,
    pub(crate) capacity: usize,
}

unsafe impl<'a, T: Send> Send for FixedBumpVec<'a, T> {}
unsafe impl<'a, T: Sync> Sync for FixedBumpVec<'a, T> {}

impl<'a, T> FixedBumpVec<'a, T> {
    /// Empty fixed vector.
    pub const EMPTY: Self = Self {
        initialized: BumpBox::EMPTY,
        capacity: if T::IS_ZST { usize::MAX } else { 0 },
    };

    /// Turns a `BumpBox<[T]>` into a full `FixedBumpVec<T>`.
    #[must_use]
    pub fn from_init(initialized: BumpBox<'a, [T]>) -> Self {
        let capacity = initialized.len();
        Self { initialized, capacity }
    }

    /// Turns a `BumpBox<[MaybeUninit<T>]>` into a `FixedBumpVec<T>` with a length of `0`.
    #[must_use]
    pub fn from_uninit(uninitialized: BumpBox<'a, [MaybeUninit<T>]>) -> Self {
        let uninitialized = uninitialized.into_raw();
        let capacity = uninitialized.len();

        let ptr = nonnull::as_non_null_ptr(uninitialized).cast::<T>();
        let initialized = unsafe { BumpBox::from_raw(nonnull::slice_from_raw_parts(ptr, 0)) };

        Self { initialized, capacity }
    }

    /// Returns the total number of elements the vector can hold.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, FixedBumpVec };
    /// # let mut bump: Bump = Bump::new();
    /// let vec = bump.alloc_fixed_vec::<i32>(2048);
    /// assert_eq!(vec.capacity(), 2048);
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    #[doc = include_str!("docs/vec/len.md")]
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.initialized.len()
    }

    #[doc = include_str!("docs/vec/is_empty.md")]
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the vector has reached its capacity.
    #[must_use]
    #[inline(always)]
    pub const fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// Returns the layout of the vector.
    #[must_use]
    pub const fn layout(&self) -> Layout {
        // We have an allocated slice. So the layout is valid.
        unsafe { Layout::from_size_align_unchecked(T::SIZE * self.len(), T::ALIGN) }
    }

    /// Turns this `FixedBumpVec<T>` into a `BumpVec<T>`.
    #[must_use]
    #[inline(always)]
    pub fn into_vec<'b, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>(
        self,
        bump: &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
    ) -> BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        A: BaseAllocator<GUARANTEED_ALLOCATED>,
    {
        BumpVec::from_parts(self, bump)
    }

    /// Turns this `FixedBumpVec<T>` into a `BumpBox<[T]>`.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_slice(self) -> BumpBox<'a, [T]> {
        self.initialized
    }

    /// Turns this `FixedBumpVec<T>` into a `&[T]` that is live for the entire bump scope.
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

    #[doc = include_str!("docs/vec/pop.md")]
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        self.initialized.pop()
    }

    #[doc = include_str!("docs/vec/clear.md")]
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump.alloc_fixed_vec(10);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.clear();
    /// assert!(vec.is_empty());
    /// ```
    #[inline(always)]
    pub fn clear(&mut self) {
        self.initialized.clear();
    }

    #[doc = include_str!("docs/vec/truncate.md")]
    /// # Examples
    ///
    /// Truncating a five element vector to two elements:
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump.alloc_fixed_vec(10);
    /// vec.extend_from_slice_copy(&[1, 2, 3, 4, 5]);
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
    /// let mut vec = bump.alloc_fixed_vec(10);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
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
    /// let mut vec = bump.alloc_fixed_vec(10);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.truncate(0);
    /// assert_eq!(vec, []);
    /// ```
    ///
    /// [`clear`]: FixedBumpVec::clear
    /// [`drain`]: FixedBumpVec::drain
    pub fn truncate(&mut self, len: usize) {
        self.initialized.truncate(len);
    }

    #[doc = include_str!("docs/vec/remove.md")]
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// let mut v = bump.alloc_slice_copy(&[1, 2, 3]);
    /// assert_eq!(v.remove(1), 2);
    /// assert_eq!(v, [1, 3]);
    /// ```
    #[track_caller]
    pub fn remove(&mut self, index: usize) -> T {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("removal index (is {index}) should be < len (is {len})");
        }

        if index >= self.len() {
            assert_failed(index, self.len());
        }

        unsafe {
            let start = self.as_mut_ptr();
            let value_ptr = start.add(index);

            // copy it out, unsafely having a copy of the value on
            // the stack and in the vector at the same time
            let value = value_ptr.read();

            // shift everything to fill in that spot
            if index != self.len() {
                let len = self.len() - index - 1;
                value_ptr.add(1).copy_to(value_ptr, len);
            }

            self.dec_len(1);
            value
        }
    }

    /// Extracts a boxed slice containing the entire vector.
    #[must_use]
    pub const fn as_boxed_slice(&self) -> &BumpBox<[T]> {
        &self.initialized
    }

    /// Extracts a mutable boxed slice containing the entire vector.
    #[must_use]
    pub fn as_mut_boxed_slice(&mut self) -> &mut BumpBox<'a, [T]> {
        &mut self.initialized
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

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_ptr(&self) -> NonNull<T> {
        self.initialized.as_non_null_ptr()
    }

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_slice(&self) -> NonNull<[T]> {
        self.initialized.as_non_null_slice()
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type. Normally changing the length of a vector
    /// is done using one of the safe operations instead, such as
    /// [`truncate`] or [`clear`].
    ///
    /// # Safety
    /// - `new_len` must be less than or equal to the [`capacity`] (capacity is not tracked by this type).
    /// - The elements at `old_len..new_len` must be initialized.
    ///
    /// [`truncate`]: FixedBumpVec::truncate
    /// [`clear`]: FixedBumpVec::clear
    /// [`capacity`]: FixedBumpVec::capacity
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.initialized.set_len(new_len);
    }

    #[inline]
    pub(crate) unsafe fn inc_len(&mut self, amount: usize) {
        self.initialized.inc_len(amount);
    }

    #[inline]
    pub(crate) unsafe fn dec_len(&mut self, amount: usize) {
        self.initialized.dec_len(amount);
    }

    #[doc = include_str!("docs/vec/swap_remove.md")]
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
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
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("swap_remove index (is {index}) should be < len (is {len})");
        }

        if index >= self.len() {
            assert_failed(index, self.len());
        }

        unsafe {
            // We replace self[index] with the last element. Note that if the
            // bounds check above succeeds there must be a last element (which
            // can be self[index] itself).

            let start = self.as_mut_ptr();
            let value_ptr = start.add(index);
            let value = value_ptr.read();
            self.dec_len(1);

            start.add(self.len()).copy_to(value_ptr, 1);
            value
        }
    }

    error_behavior_generic_methods_if! {
        if "the vector is full"

        /// Appends an element to the back of a collection.
        impl
        /// # Examples
        ///
        /// ```
        /// # use bump_scope::{ mut_bump_vec, Bump };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = bump.alloc_fixed_vec(3);
        /// vec.extend_from_slice_copy(&[1, 2]);
        /// vec.push(3);
        /// assert_eq!(vec, [1, 2, 3]);
        /// ```
        for pub fn push
        for pub fn try_push
        fn generic_push(&mut self, value: T) {
            self.generic_push_with(|| value)
        }

        /// Appends an element to the back of a collection.
        impl
        for pub fn push_with
        for pub fn try_push_with
        fn generic_push_with(&mut self, f: impl FnOnce() -> T) {
            self.generic_reserve_one()?;
            unsafe {
                self.unchecked_push_with(f);
            }
            Ok(())
        }

        /// Inserts an element at position `index` within the vector, shifting all elements after it to the right.
        do panics
        /// Panics if `index > len`.
        do examples
        /// ```
        /// # use bump_scope::{ mut_bump_vec, Bump, FixedBumpVec };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = bump.alloc_fixed_vec(5);
        /// vec.extend_from_slice_copy(&[1, 2, 3]);
        /// vec.insert(1, 4);
        /// assert_eq!(vec, [1, 4, 2, 3]);
        /// vec.insert(4, 5);
        /// assert_eq!(vec, [1, 4, 2, 3, 5]);
        /// ```
        impl
        for pub fn insert
        for pub fn try_insert
        fn generic_insert(&mut self, index: usize, element: T) {
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

        /// Copies and appends all elements in a slice to the `FixedBumpVec`.
        ///
        /// Iterates over the `slice`, copies each element, and then appends
        /// it to this `FixedBumpVec`. The `slice` is traversed in-order.
        ///
        /// Note that this function is same as [`extend`] except that it is
        /// specialized to work with copyable slices instead.
        ///
        /// [`extend`]: FixedBumpVec::extend
        impl
        for pub fn extend_from_slice_copy
        for pub fn try_extend_from_slice_copy
        fn generic_extend_from_slice_copy(&mut self, slice: &[T])
        where {
            T: Copy
        } in {
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
        /// [`extend`]: FixedBumpVec::extend
        impl
        for pub fn extend_from_slice_clone
        for pub fn try_extend_from_slice_clone
        fn generic_extend_from_slice_clone(&mut self, slice: &[T])
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

        /// Appends all elements in an array to the `FixedBumpVec`.
        ///
        /// Iterates over the `array`, copies each element, and then appends
        /// it to this `FixedBumpVec`. The `array` is traversed in-order.
        ///
        /// Note that this function is same as [`extend`] except that it is
        /// specialized to work with arrays instead.
        ///
        /// [`extend`]: FixedBumpVec::extend
        #[allow(clippy::needless_pass_by_value)]
        impl
        for pub fn extend_from_array
        for pub fn try_extend_from_array
        fn generic_extend_from_array<{const N: usize}>(&mut self, array: [T; N]) {
            unsafe { self.extend_by_copy_nonoverlapping(&array) }
        }

        /// Copies elements from `src` range to the end of the vector.
        do panics
        /// Panics if the starting point is greater than the end point or if
        /// the end point is greater than the length of the vector.
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// #
        /// let mut vec = bump.alloc_fixed_vec(100);
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
        impl
        for pub fn extend_from_within_copy
        for pub fn try_extend_from_within_copy
        fn generic_extend_from_within_copy<{R}>(&mut self, src: R)
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
        ///
        /// # Examples
        ///
        /// ```
        /// # use bump_scope::Bump;
        /// # let bump: Bump = Bump::new();
        /// #
        /// let mut vec = bump.alloc_fixed_vec(14);
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
        impl
        for pub fn extend_from_within_clone
        for pub fn try_extend_from_within_clone
        fn generic_extend_from_within_clone<{R}>(&mut self, src: R)
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
                        self.unchecked_push((*fake).clone());
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
        /// # use bump_scope::Bump;
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump.alloc_fixed_vec(5);
        /// vec.extend_from_slice_copy(&[1, 2, 3]);
        /// vec.extend_zeroed(2);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// ```
        for pub fn extend_zeroed
        for pub fn try_extend_zeroed
        fn generic_extend_zeroed(&mut self, additional: usize)
        where {
            T: zerocopy::FromZeroes
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
        /// in the given `FixedBumpVec<T>`. The collection may reserve more space to
        /// speculatively avoid frequent reallocations. After calling `reserve`,
        /// capacity will be greater than or equal to `self.len() + additional`.
        /// Does nothing if capacity is already sufficient.
        impl
        for fn reserve
        #[allow(dead_code)]
        for fn try_reserve
        fn generic_reserve(&mut self, additional: usize) {
            if additional > (self.capacity() - self.len()) {
                Err(B::fixed_size_vector_no_space(additional))
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
        /// # Examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec };
        /// # let mut bump: Bump = Bump::new();
        /// #
        /// let mut vec = bump.alloc_fixed_vec(10);
        /// vec.extend_from_slice_copy(&[1, 2, 3]);
        /// vec.resize_with(5, Default::default);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// drop(vec);
        ///
        /// let mut vec = bump.alloc_fixed_vec(10);
        /// let mut p = 1;
        /// vec.resize_with(4, || { p *= 2; p });
        /// assert_eq!(vec, [2, 4, 8, 16]);
        /// ```
        ///
        /// [`resize_with`]: FixedBumpVec::resize_with
        /// [`truncate`]: BumpBox::truncate
        impl
        for pub fn resize
        for pub fn try_resize
        fn generic_resize(&mut self, new_len: usize, value: T)
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
        do examples
        /// ```
        /// # use bump_scope::Bump;
        /// # let bump: Bump = Bump::new();
        /// #
        /// let mut vec = bump.alloc_fixed_vec(5);
        /// vec.extend_from_slice_copy(&[1, 2, 3]);
        /// vec.resize_with(5, Default::default);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// drop(vec);
        ///
        /// let mut vec = bump.alloc_fixed_vec(4);
        /// let mut p = 1;
        /// vec.resize_with(4, || { p *= 2; p });
        /// assert_eq!(vec, [2, 4, 8, 16]);
        /// ```
        impl
        for pub fn resize_with
        for pub fn try_resize_with
        fn generic_resize_with<{F}>(&mut self, new_len: usize, f: F)
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
        /// # use bump_scope::Bump;
        /// # let bump: Bump = Bump::new();
        /// #
        /// let mut vec = bump.alloc_fixed_vec(5);
        /// vec.extend_from_slice_copy(&[1, 2, 3]);
        /// vec.resize_zeroed(5);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        ///
        /// let mut vec = bump.alloc_fixed_vec(5);
        /// vec.extend_from_slice_copy(&[1, 2, 3]);
        /// vec.resize_zeroed(2);
        /// assert_eq!(vec, [1, 2]);
        /// ```
        for pub fn resize_zeroed
        for pub fn try_resize_zeroed
        fn generic_resize_zeroed(&mut self, new_len: usize)
        where {
            T: zerocopy::FromZeroes
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
        do examples
        /// ```
        /// # use bump_scope::Bump;
        /// # let mut bump: Bump = Bump::new();
        /// // needs a scope because of lifetime shenanigans
        /// let bump = bump.as_mut_scope();
        /// let mut a = bump.alloc_fixed_vec(6);
        /// let mut b = bump.alloc_fixed_vec(3);
        /// a.extend_from_slice_copy(&[1, 2, 3]);
        /// b.extend_from_slice_copy(&[4, 5, 6]);
        /// a.append(b.as_mut_boxed_slice());
        /// assert_eq!(a, [1, 2, 3, 4, 5, 6]);
        /// assert_eq!(b, []);
        /// ```
        impl
        for pub fn append
        for pub fn try_append
        fn generic_append(&mut self, other: &mut BumpBox<[T]>) {
            unsafe {
                self.extend_by_copy_nonoverlapping(other.as_slice())?;
                other.set_len(0);
                Ok(())
            }
        }
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn unchecked_push(&mut self, value: T) {
        self.unchecked_push_with(|| value);
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn unchecked_push_with(&mut self, f: impl FnOnce() -> T) {
        debug_assert!(!self.is_full());

        let ptr = self.as_mut_ptr().add(self.len());
        pointer::write_with(ptr, f);

        self.inc_len(1);
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
        let mut ptr = self.as_mut_ptr().add(self.len());

        // Use SetLenOnDrop to work around bug where compiler
        // might not realize the store through `ptr` through self.set_len()
        // don't alias.
        let mut local_len = SetLenOnDropByPtr::new(&mut self.initialized.ptr);

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

    #[doc = include_str!("docs/retain.md")]
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump.alloc_fixed_vec(4);
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
    #[allow(clippy::pedantic)]
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut v = bump.alloc_fixed_vec(3);
    /// v.extend_from_slice_copy(&[1, 2, 3]);
    /// let u = bump.alloc_iter(v.drain(1..));
    /// assert_eq!(v, [1]);
    /// assert_eq!(u, [2, 3]);
    ///
    /// // A full range clears the vector, like `clear()` does
    /// v.drain(..);
    /// assert_eq!(v, []);
    /// ```
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// # let mut vec = bump.alloc_fixed_vec(6);
    /// # vec.extend_from_slice_copy(&[1, 2, 3, 4, 5, 6]);
    /// let mut i = 0;
    /// while i < vec.len() {
    ///     if some_predicate(&mut vec[i]) {
    ///         let val = vec.remove(i);
    ///         // your code here
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut numbers = bump.alloc_fixed_vec(16);
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
    pub fn extract_if<F>(&mut self, filter: F) -> ExtractIf<T, F>
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump.alloc_fixed_vec(5);
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump.alloc_fixed_vec(5);
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump.alloc_fixed_vec(5);
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
            let uninitialized_ptr =
                nonnull::add(self.initialized.as_non_null_ptr(), self.initialized.len()).cast::<MaybeUninit<T>>();
            let uninitialized_len = self.capacity - self.len();
            let uninitialized = BumpBox::from_raw(nonnull::slice_from_raw_parts(uninitialized_ptr, uninitialized_len));
            (self.initialized, uninitialized)
        }
    }

    #[inline(always)]
    fn into_raw_parts(self) -> (BumpBox<'a, [T]>, usize) {
        (self.initialized, self.capacity)
    }

    #[inline(always)]
    fn from_raw_parts(initialized: BumpBox<'a, [T]>, capacity: usize) -> Self {
        Self { initialized, capacity }
    }

    #[inline(always)]
    unsafe fn extend_by_copy_nonoverlapping<B: ErrorBehavior>(&mut self, other: *const [T]) -> Result<(), B> {
        let len = pointer::len(other);
        self.generic_reserve(len)?;

        let src = other.cast::<T>();
        let dst = self.as_mut_ptr().add(self.len());
        ptr::copy_nonoverlapping(src, dst, len);

        self.inc_len(len);
        Ok(())
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
            let mut local_len = SetLenOnDropByPtr::new(&mut self.initialized.ptr);

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
    /// # use bump_scope::{ Bump, mut_bump_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec![in bump; [1, 2, 3], [4, 5, 6], [7, 8, 9]];
    /// assert_eq!(vec.pop(), Some([7, 8, 9]));
    ///
    /// let mut flattened = vec.into_flattened();
    /// assert_eq!(flattened.pop(), Some(6));
    /// ```
    #[must_use]
    pub fn into_flattened(self) -> FixedBumpVec<'a, T> {
        let (initialized, cap) = self.into_raw_parts();
        let ptr = initialized.as_non_null_ptr();
        let len = initialized.len();

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
            let slice = nonnull::slice_from_raw_parts(ptr.cast(), new_len);
            let initialized = BumpBox::from_raw(slice);
            FixedBumpVec::from_raw_parts(initialized, new_cap)
        }
    }
}

impl<'a, T> Debug for FixedBumpVec<'a, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.initialized.fmt(f)
    }
}

impl<'a, T> Default for FixedBumpVec<'a, T> {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl<'a, T> Deref for FixedBumpVec<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.initialized
    }
}

impl<'a, T> DerefMut for FixedBumpVec<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.initialized
    }
}

impl<'a, T, I: SliceIndex<[T]>> Index<I> for FixedBumpVec<'a, T> {
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self.as_slice(), index)
    }
}

impl<'a, T, I: SliceIndex<[T]>> IndexMut<I> for FixedBumpVec<'a, T> {
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self.as_mut_slice(), index)
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'a, T> Extend<T> for FixedBumpVec<'a, T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        self.reserve(iter.size_hint().0);

        for value in iter {
            self.push(value);
        }
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'a, 't, T> Extend<&'t T> for FixedBumpVec<'a, T>
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

impl<'a0, 'a1, T, U> PartialEq<FixedBumpVec<'a1, U>> for FixedBumpVec<'a0, T>
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &FixedBumpVec<'a1, U>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &FixedBumpVec<'a1, U>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U, const N: usize> PartialEq<[U; N]> for FixedBumpVec<'a, T>
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &[U; N]) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &[U; N]) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U, const N: usize> PartialEq<&[U; N]> for FixedBumpVec<'a, T>
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &&[U; N]) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, *other)
    }

    #[inline(always)]
    fn ne(&self, other: &&[U; N]) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, *other)
    }
}

impl<'a, T, U, const N: usize> PartialEq<&mut [U; N]> for FixedBumpVec<'a, T>
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &&mut [U; N]) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, *other)
    }

    #[inline(always)]
    fn ne(&self, other: &&mut [U; N]) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, *other)
    }
}

impl<'a, T, U> PartialEq<[U]> for FixedBumpVec<'a, T>
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &[U]) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &[U]) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<&[U]> for FixedBumpVec<'a, T>
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &&[U]) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &&[U]) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<&mut [U]> for FixedBumpVec<'a, T>
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &&mut [U]) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &&mut [U]) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<FixedBumpVec<'a, U>> for [T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &FixedBumpVec<'a, U>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &FixedBumpVec<'a, U>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<FixedBumpVec<'a, U>> for &[T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &FixedBumpVec<'a, U>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &FixedBumpVec<'a, U>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<FixedBumpVec<'a, U>> for &mut [T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &FixedBumpVec<'a, U>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &FixedBumpVec<'a, U>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T> IntoIterator for FixedBumpVec<'a, T> {
    type Item = T;
    type IntoIter = IntoIter<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.initialized.into_iter()
    }
}

impl<'c, 'a, T> IntoIterator for &'c FixedBumpVec<'a, T> {
    type Item = &'c T;
    type IntoIter = slice::Iter<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'c, 'a, T> IntoIterator for &'c mut FixedBumpVec<'a, T> {
    type Item = &'c mut T;
    type IntoIter = slice::IterMut<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<'a, T> AsRef<Self> for FixedBumpVec<'a, T> {
    #[inline(always)]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<'a, T> AsMut<Self> for FixedBumpVec<'a, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<'a, T> AsRef<[T]> for FixedBumpVec<'a, T> {
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<'a, T> AsMut<[T]> for FixedBumpVec<'a, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'a, T> Borrow<[T]> for FixedBumpVec<'a, T> {
    #[inline(always)]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<'a, T> BorrowMut<[T]> for FixedBumpVec<'a, T> {
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'a, T: Hash> Hash for FixedBumpVec<'a, T> {
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
        if (self.capacity - self.len()) < buf.len() {
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

        if (self.capacity - self.len()) < len {
            return Err(std::io::ErrorKind::OutOfMemory.into());
        }

        unsafe {
            let mut dst = self.as_mut_ptr().add(self.len());

            for buf in bufs {
                buf.as_ptr().copy_to_nonoverlapping(dst, buf.len());
                dst = dst.add(buf.len());
            }

            self.inc_len(len);
        }

        Ok(len)
    }

    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if (self.capacity - self.len()) < buf.len() {
            return Err(std::io::ErrorKind::OutOfMemory.into());
        }

        Ok(())
    }
}

impl<T> NoDrop for FixedBumpVec<'_, T> where T: NoDrop {}
