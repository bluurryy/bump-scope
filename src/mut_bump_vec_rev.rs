use crate::{
    error_behavior_generic_methods_allocation_failure, owned_slice,
    polyfill::{self, nonnull, pointer},
    BaseAllocator, BumpBox, BumpScope, ErrorBehavior, GuaranteedAllocatedStats, MinimumAlignment, NoDrop, SetLenOnDrop,
    SizedTypeProperties, Stats, SupportedMinimumAlignment,
};
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

/// This is like [`vec!`] but allocates inside a `Bump` or `BumpScope`, returning a [`MutBumpVecRev`].
///
/// `$bump` can be a mutable [`Bump`](crate::Bump) or [`BumpScope`] (anything where `$bump.as_mut_scope()` returns a `&mut BumpScope`).
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
/// # use bump_scope::{ mut_bump_vec_rev, Bump, MutBumpVecRev };
/// # let mut bump: Bump = Bump::new();
/// let vec: MutBumpVecRev<i32> = mut_bump_vec_rev![in bump];
/// assert!(vec.is_empty());
/// ```
///
/// - Create a [`MutBumpVecRev`] containing a given list of elements:
///
/// ```
/// # use bump_scope::{ mut_bump_vec_rev, Bump };
/// # let mut bump: Bump = Bump::new();
/// let vec = mut_bump_vec_rev![in bump; 1, 2, 3];
/// assert_eq!(vec[0], 1);
/// assert_eq!(vec[1], 2);
/// assert_eq!(vec[2], 3);
/// ```
///
/// - Create a [`MutBumpVecRev`] from a given element and size:
///
/// ```
/// # use bump_scope::{ mut_bump_vec_rev, Bump };
/// # let mut bump: Bump = Bump::new();
/// let vec = mut_bump_vec_rev![in bump; 1; 3];
/// assert_eq!(vec, [1, 1, 1]);
/// ```
///
/// Note that unlike array expressions this syntax supports all elements
/// which implement [`Clone`] and the number of elements doesn't have to be
/// a constant.
///
/// This will use `clone` to duplicate an expression, so one should be careful
/// using this with types having a nonstandard `Clone` implementation. For
/// example, `mut_bump_vec_rev![in bump; Rc::new(1); 5]` will create a vector of five references
/// to the same boxed integer value, not five references pointing to independently
/// boxed integers.
///
/// Also, note that `mut_bump_vec_rev![in bump; expr; 0]` is allowed, and produces an empty vector.
/// This will still evaluate `expr`, however, and immediately drop the resulting value, so
/// be mindful of side effects.
#[macro_export]
macro_rules! mut_bump_vec_rev {
    [in $bump:expr] => {
        $crate::MutBumpVecRev::new_in($bump.as_mut_scope())
    };
    [in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::MutBumpVecRev::from_array_in([$($values),*], $bump.as_mut_scope())
    };
    [in $bump:expr; $value:expr; $count:expr] => {
        $crate::MutBumpVecRev::from_elem_in($value, $count, $bump.as_mut_scope())
    };
    [try in $bump:expr] => {
        Ok::<_, $crate::allocator_api2::alloc::AllocError>($crate::MutBumpVecRev::new_in($bump.as_mut_scope()))
    };
    [try in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::MutBumpVecRev::try_from_array_in([$($values),*], $bump.as_mut_scope())
    };
    [try in $bump:expr; $value:expr; $count:expr] => {
        $crate::MutBumpVecRev::try_from_elem_in($value, $count, $bump.as_mut_scope())
    };
}

macro_rules! mut_bump_vec_rev_declaration {
    ($($allocator_parameter:tt)*) => {
        /// This is like a [`MutBumpVec`](crate::MutBumpVec), but new elements are pushed to the front.
        ///
        /// This type can be used to allocate a slice, when `alloc_*` methods are too limiting:
        /// ```
        /// use bump_scope::{ Bump, mut_bump_vec_rev };
        /// let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec_rev![in bump];
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
        /// use bump_scope::{ Bump, mut_bump_vec_rev };
        /// let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec_rev![in bump; 4, 5, 6];
        ///
        /// vec.extend_from_slice_copy(&[1, 2, 3]);
        ///
        /// assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
        /// ```
        pub struct MutBumpVecRev<
            'b,
            'a: 'b,
            T,
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        > {
            end: NonNull<T>,
            len: usize,
            cap: usize,

            pub(crate) bump: &'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,

            /// First field marks the lifetime.
            /// Second field marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
            marker: PhantomData<(&'a (), T)>,
        }
    };
}

crate::maybe_default_allocator!(mut_bump_vec_rev_declaration);

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> UnwindSafe
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: UnwindSafe,
    A: UnwindSafe,
{
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> RefUnwindSafe
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: RefUnwindSafe,
    A: RefUnwindSafe,
{
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Constructs a new empty `MutBumpVecRev<T>`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, MutBumpVecRev };
    /// # let mut bump: Bump = Bump::new();
    /// # #[allow(unused_mut)]
    /// let mut vec = MutBumpVecRev::<i32>::new_in(&mut bump);
    /// # let _ = vec;
    /// ```
    #[inline]
    pub fn new_in(bump: impl Into<&'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
        Self {
            end: NonNull::dangling(),
            len: 0,
            cap: if T::IS_ZST { usize::MAX } else { 0 },
            bump: bump.into(),
            marker: PhantomData,
        }
    }

    error_behavior_generic_methods_allocation_failure! {
        #[doc = include_str!("docs/vec/with_capacity.md")]
        impl
        for pub fn with_capacity_in
        for pub fn try_with_capacity_in
        fn generic_with_capacity_in(capacity: usize, bump: impl Into<&'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            let bump = bump.into();

            if T::IS_ZST {
                return Ok(Self {
                    end: NonNull::dangling(),
                    len: 0,
                    cap: usize::MAX,
                    bump,
                    marker: PhantomData,
                });
            }

            if capacity == 0 {
                return Ok(Self {
                    end: NonNull::dangling(),
                    len: 0,
                    cap: 0,
                    bump,
                    marker: PhantomData,
                });
            }

            let (end, cap) = bump.alloc_greedy_rev(capacity)?;

            Ok(Self {
                end,
                len: 0,
                cap,
                bump,
                marker: PhantomData,
            })
        }

        /// Constructs a new `MutBumpVecRev<T>` and pushes `value` `count` times.
        impl
        for pub fn from_elem_in
        for pub fn try_from_elem_in
        fn generic_from_elem_in(value: T, count: usize, bump: impl Into<&'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self
        where {
            T: Clone
        } in {
            let mut vec = Self::generic_with_capacity_in(count, bump)?;

            unsafe {
                if count != 0 {
                    for _ in 0..(count - 1) {
                        vec.unchecked_push_with(|| value.clone());
                    }

                    vec.unchecked_push_with(|| value);
                }
            }

            Ok(vec)
        }

        /// Constructs a new `MutBumpVecRev<T>` from a `[T; N]`.
        impl
        for pub fn from_array_in
        for pub fn try_from_array_in
        fn generic_from_array_in<{const N: usize}>(array: [T; N], bump: impl Into<&'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            #![allow(clippy::needless_pass_by_value)]
            #![allow(clippy::needless_pass_by_ref_mut)]

            let array = ManuallyDrop::new(array);
            let bump = bump.into();

            if T::IS_ZST {
                return Ok(Self {
                    end: NonNull::dangling(),
                    len: N,
                    cap: usize::MAX,
                    bump,
                    marker: PhantomData,
                });
            }

            if N == 0 {
                return Ok(Self {
                    end: NonNull::dangling(),
                    len: 0,
                    cap: 0,
                    bump,
                    marker: PhantomData,
                });
            }

            let (end, cap) = bump.alloc_greedy_rev::<B, T>(N)?;
            let src = array.as_ptr();

            unsafe {
                let dst = end.as_ptr().sub(N);
                ptr::copy_nonoverlapping(src, dst, N);
            };

            Ok(Self {
                end,
                len: N,
                cap,
                bump,
                marker: PhantomData,
            })
        }
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[must_use]
    #[inline(always)]
    fn as_nonnull_slice(&self) -> NonNull<[T]> {
        nonnull::slice_from_raw_parts(self.as_nonnull_ptr(), self.len)
    }

    #[doc = include_str!("docs/vec/capacity.md")]
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, MutBumpVecRev };
    /// # let mut bump: Bump = Bump::new();
    /// let vec = MutBumpVecRev::<i32>::with_capacity_in(2048, &mut bump);
    /// assert!(vec.capacity() >= 2048);
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.cap
    }

    #[doc = include_str!("docs/vec/len.md")]
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[doc = include_str!("docs/vec/is_empty.md")]
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    unsafe fn unchecked_push_with(&mut self, f: impl FnOnce() -> T) {
        debug_assert!(self.len < self.cap);

        let ptr = nonnull::sub(self.end, self.len + 1);
        nonnull::write_with(ptr, f);

        // We set the len here so when `f` panics, `self.len` doesn't change.
        self.len += 1;
    }

    /// Removes the first element from a vector and returns it, or [`None`] if it
    /// is empty.
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

    #[doc = include_str!("docs/vec/clear.md")]
    /// Note that this method has no effect on the allocated capacity
    /// of the vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec_rev };
    /// # let mut bump: Bump = Bump::new();
    /// let mut v = mut_bump_vec_rev![in bump; 1, 2, 3];
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
    /// # use bump_scope::{ Bump, mut_bump_vec_rev };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let x = mut_bump_vec_rev![in bump; 1, 2, 4];
    /// let x_ptr = x.as_ptr();
    ///
    /// unsafe {
    ///     for i in 0..x.len() {
    ///         assert_eq!(*x_ptr.add(i), 1 << i);
    ///     }
    /// }
    /// ```
    ///
    /// [`as_mut_ptr`]: MutBumpVecRev::as_mut_ptr
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        // We shadow the slice method of the same name to avoid going through
        // `deref`, which creates an intermediate reference.
        self.as_nonnull_ptr().as_ptr()
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
    /// # use bump_scope::{ Bump, MutBumpVecRev };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// // Allocate vector big enough for 4 elements.
    /// let size = 4;
    /// let mut x = MutBumpVecRev::<i32>::with_capacity_in(size, &mut bump);
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
        self.as_nonnull_ptr().as_ptr()
    }

    #[inline]
    fn as_nonnull_ptr(&self) -> NonNull<T> {
        unsafe { nonnull::sub(self.end, self.len) }
    }

    #[doc = include_str!("docs/vec/rev/truncate.md")]
    ///
    /// # Examples
    ///
    /// Truncating a five element vector to two elements:
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec_rev };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3, 4, 5];
    /// vec.truncate(2);
    /// assert_eq!(vec, [4, 5]);
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current
    /// length:
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec_rev };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3];
    /// vec.truncate(8);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec_rev };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3];
    /// vec.truncate(0);
    /// assert_eq!(vec, []);
    /// ```
    ///
    /// [`clear`]: MutBumpVecRev::clear
    /// [`drain`]: MutBumpVecRev::drain
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
    /// [`truncate`]: MutBumpVecRev::truncate
    /// [`resize`]: MutBumpVecRev::resize
    /// [`extend`]: MutBumpVecRev::extend
    /// [`clear`]: MutBumpVecRev::clear
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to [`capacity`].
    /// - The elements at `old_len..new_len` must be initialized.
    ///
    /// [`capacity`]: MutBumpVecRev::capacity
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.cap);
        self.len = new_len;
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    error_behavior_generic_methods_allocation_failure! {

        /// Appends an element to the front of a collection.
        impl
        /// # Examples
        ///
        /// ```
        /// # use bump_scope::{ mut_bump_vec_rev, Bump };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec_rev![in bump; 2, 1];
        /// vec.push(3);
        /// assert_eq!(vec, [3, 2, 1]);
        /// # let _ = vec;
        /// ```
        for pub fn push
        for pub fn try_push
        fn generic_push(&mut self, value: T) {
            self.generic_push_with(|| value)
        }

        /// Appends an element to the front of a collection.
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
        /// # use bump_scope::{ mut_bump_vec_rev, Bump };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3];
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
        impl
        for pub fn extend_from_slice_copy
        for pub fn try_extend_from_slice_copy
        fn generic_extend_from_slice_copy(&mut self, slice: &[T])
        where {
            T: Copy
        } in {
            unsafe { self.extend_by_copy_nonoverlapping(slice) }
        }

        /// Clones and appends all elements in a slice to the `MutBumpVecRev`.
        ///
        /// Iterates over the `slice`, clones each element, and then appends
        /// it to this `MutBumpVecRev`. The `slice` is traversed in-order.
        impl
        for pub fn extend_from_slice_clone
        for pub fn try_extend_from_slice_clone
        fn generic_extend_from_slice_clone(&mut self, slice: &[T])
        where {
            T: Clone
        } in {
            self.generic_reserve(slice.len())?;

            unsafe {
                // Addition doesn't overflow because `reserve` checked for that.
                let mut ptr = nonnull::sub(self.end, self.len + slice.len());

                for value in slice {
                    nonnull::write_with(ptr, || value.clone());
                    ptr = nonnull::add(ptr, 1);
                    self.len += 1;
                }
            }

            Ok(())
        }

        /// Appends all elements in an array to the `MutBumpVecRev`.
        ///
        /// Iterates over the `array`, copies each element, and then appends
        /// it to this `MutBumpVecRev`. The `array` is traversed in-order.
        #[allow(clippy::needless_pass_by_value)]
        impl
        for pub fn extend_from_array
        for pub fn try_extend_from_array
        fn generic_extend_from_array<{const N: usize}>(&mut self, array: [T; N]) {
            unsafe { self.extend_by_copy_nonoverlapping(&array) }
        }

        /// Copies elements from `src` range to the start of the vector.
        do panics
        /// Panics if the starting point is greater than the end point or if
        /// the end point is greater than the length of the vector.
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec_rev };
        /// # let mut bump: Bump = Bump::new();
        /// #
        /// let mut vec = mut_bump_vec_rev![in bump; 0, 1, 2, 3, 4];
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
        impl
        for pub fn extend_from_within_copy
        for pub fn try_extend_from_within_copy
        fn generic_extend_from_within_copy<{R}>(&mut self, src: R)
        where {
            T: Copy,
            R: RangeBounds<usize>,
        } in {
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

        #[cfg(feature = "zerocopy")]
        /// Extends this vector by pushing `additional` new items onto the end.
        /// The new items are initialized with zeroes.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec_rev };
        /// # let mut bump: Bump = Bump::new();
        /// let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3];
        /// vec.extend_zeroed(2);
        /// assert_eq!(vec, [0, 0, 1, 2, 3]);
        /// ```
        for pub fn extend_zeroed
        for pub fn try_extend_zeroed
        fn generic_extend_zeroed(&mut self, additional: usize)
        where {
            T: zerocopy::FromZeroes
        } in {
            self.generic_reserve(additional)?;

            unsafe {
                let new_len = self.len() + additional;
                nonnull::sub(self.end, new_len).as_ptr().write_bytes(0, additional);
                self.set_len(new_len);
            }

            Ok(())
        }

        /// Reserves capacity for at least `additional` more elements to be inserted
        /// in the given `MutBumpVecRev<T>`. The collection may reserve more space to
        /// speculatively avoid frequent reallocations. After calling `reserve`,
        /// capacity will be greater than or equal to `self.len() + additional`.
        /// Does nothing if capacity is already sufficient.
        impl
        for pub fn reserve
        for pub fn try_reserve
        fn generic_reserve(&mut self, additional: usize) {
            if additional > (self.cap - self.len) {
                self.generic_grow_cold(additional)?;
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
        /// [`Clone`]), use [`MutBumpVecRev::resize_with`].
        /// If you only need to resize to a smaller size, use [`MutBumpVecRev::truncate`].
        ///
        /// # Examples
        ///
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec_rev };
        /// # let mut bump: Bump = Bump::new();
        /// #
        /// let mut vec = mut_bump_vec_rev![in bump; "hello"];
        /// vec.resize(3, "world");
        /// assert_eq!(vec, ["world", "world", "hello"]);
        /// drop(vec);
        ///
        /// let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3, 4];
        /// vec.resize(2, 0);
        /// assert_eq!(vec, [3, 4]);
        /// ```
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
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec_rev };
        /// # let mut bump: Bump = Bump::new();
        /// #
        /// let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3];
        /// vec.resize_with(5, Default::default);
        /// assert_eq!(vec, [0, 0, 1, 2, 3]);
        /// drop(vec);
        ///
        /// let mut vec = mut_bump_vec_rev![in bump];
        /// let mut p = 1;
        /// vec.resize_with(4, || { p *= 2; p });
        /// assert_eq!(vec, [16, 8, 4, 2]);
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
        /// # use bump_scope::{ Bump, mut_bump_vec_rev };
        /// # let mut bump: Bump = Bump::new();
        /// #
        /// {
        ///     let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3];
        ///     vec.resize_zeroed(5);
        ///     assert_eq!(vec, [0, 0, 1, 2, 3]);
        /// }
        ///
        /// {
        ///     let mut vec = mut_bump_vec_rev![in bump; 1, 2, 3];
        ///     vec.resize_zeroed(2);
        ///     assert_eq!(vec, [2, 3]);
        /// }
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
            self.generic_grow_cold::<E>(1)?;
        }

        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn generic_grow_cold<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        let required_cap = match self.cap.checked_add(additional) {
            Some(required_cap) => required_cap,
            None => return Err(E::capacity_overflow())?,
        };

        if T::IS_ZST {
            return Ok(());
        }

        let min_cap = self.cap.checked_mul(2).unwrap_or(required_cap).max(required_cap);
        let (end, cap) = self.bump.alloc_greedy_rev::<E, T>(min_cap)?;

        unsafe {
            let src = self.as_mut_ptr();
            let dst = end.as_ptr().sub(self.len);
            ptr::copy_nonoverlapping(src, dst, self.len);

            self.end = end;
            self.cap = cap;
        }

        Ok(())
    }

    #[doc = include_str!("docs/vec/rev/remove.md")]
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec_rev };
    /// # let mut bump: Bump = Bump::new();
    /// let mut v = mut_bump_vec_rev![in bump; 1, 2, 3];
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

    #[doc = include_str!("docs/vec/rev/swap_remove.md")]
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec_rev };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut v = mut_bump_vec_rev![in bump; "foo", "bar", "baz", "qux"];
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
        let this = ManuallyDrop::new(self);

        if T::IS_ZST {
            return nonnull::slice_from_raw_parts(NonNull::dangling(), this.len());
        }

        if this.cap == 0 {
            // We didn't touch the bump, so no need to do anything.
            debug_assert_eq!(this.end, NonNull::<T>::dangling());
            return nonnull::slice_from_raw_parts(NonNull::<T>::dangling(), 0);
        }

        unsafe { this.bump.consolidate_greed_rev(this.end, this.len, this.cap) }
    }

    /// Turns this `MutBumpVecRev<T>` into a `BumpBox<[T]>`.
    ///
    /// Unused capacity does not take up space.<br/>
    /// When [bumping upwards](crate#bumping-upwards-or-downwards) this needs to shift all elements to the other end of the chunk.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_slice(self) -> BumpBox<'a, [T]> {
        unsafe { BumpBox::from_raw(self.into_slice_ptr()) }
    }

    /// Turns this `MutBumpVecRev<T>` into a `&[T]` that is live for the entire bump scope.
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

    /// Returns vector content as a slice of `T`, along with the remaining spare
    /// capacity of the vector as a slice of `MaybeUninit<T>`.
    ///
    /// The returned spare capacity slice can be used to fill the vector with data
    /// (e.g. by reading from a file) before marking the data as initialized using
    /// the [`set_len`] method.
    ///
    /// [`set_len`]: MutBumpVecRev::set_len
    ///
    /// Note that this is a low-level API, which should be used with care for
    /// optimization purposes. If you need to append data to a `MutBumpVecRev`
    /// you can use [`push`], [`extend`], `extend_from_slice`[`_copy`](MutBumpVecRev::extend_from_slice_copy)`/`[`_clone`](MutBumpVecRev::extend_from_within_clone),
    /// `extend_from_within`[`_copy`](MutBumpVecRev::extend_from_within_copy)`/`[`_clone`](MutBumpVecRev::extend_from_within_clone), [`insert`], [`resize`] or
    /// [`resize_with`], depending on your exact needs.
    ///
    /// [`push`]: MutBumpVecRev::push
    /// [`extend`]: MutBumpVecRev::extend
    /// [`insert`]: MutBumpVecRev::insert
    /// [`append`]: MutBumpVecRev::append
    /// [`resize`]: MutBumpVecRev::resize
    /// [`resize_with`]: MutBumpVecRev::resize_with
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

    #[doc = include_str!("docs/allocator.md")]
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        self.bump.allocator()
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool> MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP>
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

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + 'a,
{
    error_behavior_generic_methods_allocation_failure! {
        /// Clones elements from `src` range to the end of the vector.
        do panics
        /// Panics if the starting point is greater than the end point or if
        /// the end point is greater than the length of the vector.
        do examples
        /// ```
        /// # use bump_scope::{ Bump, mut_bump_vec_rev };
        /// # let mut bump: Bump = Bump::new();
        /// #
        /// let mut vec = mut_bump_vec_rev![in bump; 0, 1, 2, 3, 4];
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
        impl
        for pub fn extend_from_within_clone
        for pub fn try_extend_from_within_clone
        fn generic_extend_from_within_clone<{R}>(&mut self, src: R)
        where {
            T: Clone,
            R: RangeBounds<usize>,
        } in {
            let range = polyfill::slice::range(src, ..self.len());
            let count = range.len();

            self.generic_reserve(count)?;

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
    }
}

impl<'b, 'a: 'b, T: Debug, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Debug
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_slice(), f)
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Deref
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DerefMut
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, I: SliceIndex<[T]>> Index<I>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self.as_slice(), index)
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, I: SliceIndex<[T]>>
    IndexMut<I> for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self.as_mut_slice(), index)
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'b, 'a: 'b, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Extend<U>
    for MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + 'a,
{
    #[inline]
    fn extend<T: IntoIterator<Item = U>>(&mut self, iter: T) {
        let iter = iter.into_iter();

        self.reserve(iter.size_hint().0);

        for value in iter {
            self.push(value);
        }
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn drop(&mut self) {
        // MutBumpVecRev never actually moves a bump pointer.
        // It may force allocation of a new chunk, but it does not move the pointer within.
        // So we don't need to move the bump pointer when dropping.

        // If we want to reset the bump pointer to a previous chunk, we use a bump scope.
        // We could do it here, by resetting to the last non-empty chunk but that would require a loop.
        // Chunk allocations are supposed to be very rare, so this wouldn't be worth it.

        unsafe {
            self.as_nonnull_slice().as_ptr().drop_in_place();
        }
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'b, 'a: 'b, 't, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Extend<&'t T>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + 'a,
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

impl<'b0, 'a0: 'b0, 'b1, 'a1: 'b1, T, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<MutBumpVecRev<'b1, 'a1, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>
    for MutBumpVecRev<'b0, 'a0, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &MutBumpVecRev<'b1, 'a1, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &MutBumpVecRev<'b1, 'a1, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'b, 'a: 'b, T, U, const N: usize, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<[U; N]> for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<'b, 'a: 'b, T, U, const N: usize, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<&[U; N]> for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<'b, 'a: 'b, T, U, const N: usize, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<&mut [U; N]> for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<'b, 'a: 'b, T, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq<[U]>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<'b, 'a: 'b, T, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq<&[U]>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<'b, 'a: 'b, T, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> PartialEq<&mut [U]>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<'b, 'a: 'b, T, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for [T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'b, 'a: 'b, T, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for &[T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'b, 'a: 'b, T, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for &mut [T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &MutBumpVecRev<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> IntoIterator
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    type Item = T;
    type IntoIter = owned_slice::IntoIter<'b, T>;

    /// Returns an iterator that borrows the `BumpScope` mutably. So you can't use the `BumpScope` while iterating.
    /// The advantage is that the space the items took up is freed.
    ///
    /// If you need to use the `BumpScope` while iterating you can first turn it to a slice with [`MutBumpVecRev::into_boxed_slice`].
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        let this = ManuallyDrop::new(self);
        let start = this.as_nonnull_ptr();
        let slice = nonnull::slice_from_raw_parts(start, this.len);
        unsafe { owned_slice::IntoIter::new(slice) }
    }
}

impl<'c, 'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> IntoIterator
    for &'c MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    type Item = &'c T;
    type IntoIter = slice::Iter<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'c, 'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> IntoIterator
    for &'c mut MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    type Item = &'c mut T;
    type IntoIter = slice::IterMut<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AsRef<Self>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AsMut<Self>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AsRef<[T]>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AsMut<[T]>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Borrow<[T]>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BorrowMut<[T]>
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'b, 'a: 'b, T: Hash, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Hash
    for MutBumpVecRev<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}
