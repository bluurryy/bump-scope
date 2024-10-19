mod drain;
mod into_iter;
mod splice;

use crate::{
    bump_down, error_behavior_generic_methods_allocation_failure, owned_slice,
    polyfill::{nonnull, pointer, slice},
    up_align_usize_unchecked, BaseAllocator, BumpBox, BumpScope, ErrorBehavior, FixedBumpVec, GuaranteedAllocatedStats,
    MinimumAlignment, NoDrop, SetLenOnDropByPtr, SizedTypeProperties, Stats, SupportedMinimumAlignment,
};
use allocator_api2::alloc::Allocator;
use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    hash::Hash,
    iter,
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Index, IndexMut, RangeBounds},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::{self, NonNull},
    slice::SliceIndex,
};

#[cfg(not(no_global_oom_handling))]
pub(crate) use drain::Drain;
pub use into_iter::IntoIter;

#[cfg(not(no_global_oom_handling))]
pub use splice::Splice;

/// This is like [`vec!`] but allocates inside a `Bump` or `BumpScope`, returning a [`BumpVec`].
///
/// `$bump` can be a [`Bump`](crate::Bump) or [`BumpScope`] (anything where `$bump.as_scope()` returns a `&BumpScope`).
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
/// - Create an empty [`BumpVec`]:
/// ```
/// # use bump_scope::{ bump_vec, Bump, BumpVec };
/// # let bump: Bump = Bump::new();
/// let vec: BumpVec<i32> = bump_vec![in bump];
/// assert!(vec.is_empty());
/// ```
///
/// - Create a [`BumpVec`] containing a given list of elements:
///
/// ```
/// # use bump_scope::{ bump_vec, Bump };
/// # let bump: Bump = Bump::new();
/// let vec = bump_vec![in bump; 1, 2, 3];
/// assert_eq!(vec[0], 1);
/// assert_eq!(vec[1], 2);
/// assert_eq!(vec[2], 3);
/// ```
///
/// - Create a [`BumpVec`] from a given element and size:
///
/// ```
/// # use bump_scope::{ bump_vec, Bump };
/// # let bump: Bump = Bump::new();
/// let vec = bump_vec![in bump; 1; 3];
/// assert_eq!(vec, [1, 1, 1]);
/// ```
///
/// Note that unlike array expressions this syntax supports all elements
/// which implement [`Clone`] and the number of elements doesn't have to be
/// a constant.
///
/// This will use `clone` to duplicate an expression, so one should be careful
/// using this with types having a nonstandard `Clone` implementation. For
/// example, `bump_vec![in bump; Rc::new(1); 5]` will create a vector of five references
/// to the same boxed integer value, not five references pointing to independently
/// boxed integers.
///
/// Also, note that `bump_vec![in bump; expr; 0]` is allowed, and produces an empty vector.
/// This will still evaluate `expr`, however, and immediately drop the resulting value, so
/// be mindful of side effects.
#[macro_export]
macro_rules! bump_vec {
    [in $bump:expr] => {
        $crate::BumpVec::new_in($bump.as_scope())
    };
    [in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::BumpVec::from_array_in([$($values),*], $bump.as_scope())
    };
    [in $bump:expr; $value:expr; $count:expr] => {
        $crate::BumpVec::from_elem_in($value, $count, $bump.as_scope())
    };
    [try in $bump:expr] => {
        Ok::<_, $crate::allocator_api2::alloc::AllocError>($crate::BumpVec::new_in($bump.as_scope()))
    };
    [try in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::BumpVec::try_from_array_in([$($values),*], $bump.as_scope())
    };
    [try in $bump:expr; $value:expr; $count:expr] => {
        $crate::BumpVec::try_from_elem_in($value, $count, $bump.as_scope())
    };
}

macro_rules! bump_vec_declaration {
    ($($allocator_parameter:tt)*) => {
        /// A bump allocated [`Vec`](alloc::vec::Vec).
        ///
        /// The main difference to `Vec` is that it can be turned into a slice that is live for this bump scope (`'a`).
        /// Such a slice can be live while entering new scopes.
        ///
        /// This would not be possible with `Vec`:
        ///
        /// ```
        /// # use bump_scope::{ Bump, BumpVec };
        /// # let mut bump: Bump = Bump::new();
        /// let bump = bump.as_mut_scope();
        ///
        /// let slice = {
        ///     let mut vec = BumpVec::new_in(&*bump);
        ///
        ///     vec.push(1);
        ///     vec.push(2);
        ///     vec.push(3);
        ///
        ///     vec.into_slice()
        /// };
        ///
        /// bump.scoped(|bump| {
        ///     // allocate more things
        /// });
        ///
        /// assert_eq!(slice, [1, 2, 3]);
        /// ```
        ///
        /// # Examples
        ///
        /// This type can be used to allocate a slice, when `alloc_*` methods are too limiting:
        /// ```
        /// use bump_scope::{ Bump, BumpVec };
        /// let bump: Bump = Bump::new();
        /// let mut vec = BumpVec::new_in(&bump);
        ///
        /// vec.push(1);
        /// vec.push(2);
        /// vec.push(3);
        ///
        /// let slice: &[i32] = vec.into_slice();
        ///
        /// assert_eq!(slice, [1, 2, 3]);
        /// ```
        // `BumpString` and `BumpVec<u8>` have the same repr.
        #[repr(C)]
        pub struct BumpVec<
            'b,
            'a: 'b,
            T,
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        >
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
            pub(crate) fixed: FixedBumpVec<'a, T>,
            pub(crate) bump: &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
        }
    };
}

crate::maybe_default_allocator!(bump_vec_declaration);

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> UnwindSafe
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: UnwindSafe,
    A: UnwindSafe,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> RefUnwindSafe
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: RefUnwindSafe,
    A: RefUnwindSafe,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Deref
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.fixed
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DerefMut
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fixed
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn drop(&mut self) {
        struct DropGuard<'i, 'b, 'a, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>(
            &'i mut BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
        )
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>;

        impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
            for DropGuard<'_, '_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
            fn drop(&mut self) {
                unsafe {
                    let ptr = self.0.fixed.initialized.ptr.cast();
                    let layout = Layout::from_size_align_unchecked(self.0.fixed.capacity * T::SIZE, T::ALIGN);
                    self.0.bump.deallocate(ptr, layout);
                }
            }
        }

        let guard = DropGuard(self);

        // destroy the remaining elements
        guard.0.clear();

        // now `guard` will be dropped and deallocate the memory
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Clone
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            fixed: FixedBumpVec::from_init(self.bump.alloc_slice_clone(self)),
            bump: self.bump,
        }
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Constructs a new empty `BumpVec<T>`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpVec };
    /// # let bump: Bump = Bump::new();
    /// # #[allow(unused_mut)]
    /// let mut vec = BumpVec::<i32>::new_in(&bump);
    /// ```
    #[inline]
    pub fn new_in(bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
        Self {
            fixed: FixedBumpVec::EMPTY,
            bump: bump.into(),
        }
    }

    error_behavior_generic_methods_allocation_failure! {
        /// Constructs a new empty vector with at least the specified capacity
        /// with the provided `BumpScope`.
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
        use fn generic_with_capacity_in(capacity: usize, bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            let bump = bump.into();

            if T::IS_ZST {
                return Ok(Self {
                    fixed: FixedBumpVec::EMPTY,
                    bump,
                });
            }

            if capacity == 0 {
                return Ok(Self {
                    fixed: FixedBumpVec::EMPTY,
                    bump,
                });
            }

            Ok(Self {
                fixed: bump.generic_alloc_fixed_vec(capacity)?,
                bump,
            })
        }

        /// Constructs a new `BumpVec<T>` and pushes `value` `count` times.
        impl
        for fn from_elem_in
        for fn try_from_elem_in
        use fn generic_from_elem_in(value: T, count: usize, bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self
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

        /// Constructs a new `BumpVec<T>` from a `[T; N]`.
        impl
        for fn from_array_in
        for fn try_from_array_in
        use fn generic_from_array_in<{const N: usize}>(array: [T; N], bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>) -> Self {
            #![allow(clippy::needless_pass_by_value)]
            #![allow(clippy::needless_pass_by_ref_mut)]

            let array = ManuallyDrop::new(array);
            let bump = bump.into();

            if T::IS_ZST {
                return Ok(Self {
                    fixed: FixedBumpVec { initialized: unsafe { BumpBox::from_raw(nonnull::slice_from_raw_parts(NonNull::dangling(), N)) }, capacity: usize::MAX },
                    bump,
                });
            }

            if N == 0 {
                return Ok(Self {
                    fixed: FixedBumpVec::EMPTY,
                    bump,
                });
            }

            let mut fixed = bump.generic_alloc_fixed_vec(N)?;

            let src = array.as_ptr();
            let dst = fixed.as_mut_ptr();

            unsafe {
                ptr::copy_nonoverlapping(src, dst, N);
                fixed.set_len(N);
            }

            Ok(Self { fixed, bump })
        }
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Returns the total number of elements the vector can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpVec };
    /// # let bump: Bump = Bump::new();
    /// let vec = BumpVec::<i32>::with_capacity_in(2048, &bump);
    /// assert!(vec.capacity() >= 2048);
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.fixed.capacity()
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
        self.fixed.is_empty()
    }

    /// Splits the collection into two at the given index.
    ///
    /// Returns a newly allocated vector containing the elements in the range
    /// `[at, len)`. After the call, the original vector will be left containing
    /// the elements `[0, at)` with its previous capacity unchanged.
    ///
    /// # Panics
    ///
    /// Panics if `at > len`.
    ///
    /// Panics on allocation failure.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpVec, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in bump; 1, 2, 3];
    /// let vec2 = vec.split_off(1);
    /// assert_eq!(vec, [1]);
    /// assert_eq!(vec2, [2, 3]);
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[inline]
    #[must_use = "use `.truncate()` if you don't need the other half"]
    pub fn split_off(&mut self, at: usize) -> Self
    where
        A: Clone,
    {
        #[cold]
        #[inline(never)]
        fn assert_failed(at: usize, len: usize) -> ! {
            panic!("`at` split index (is {at}) should be <= len (is {len})");
        }

        if at > self.len() {
            assert_failed(at, self.len());
        }

        if at == 0 {
            // the new vector can take over the original buffer and avoid the copy
            return mem::replace(self, BumpVec::with_capacity_in(self.capacity(), self.bump()));
        }

        let other_len = self.len() - at;
        let mut other = BumpVec::with_capacity_in(other_len, self.bump());

        // Unsafely `set_len` and copy items to `other`.
        unsafe {
            self.set_len(at);
            other.set_len(other_len);

            ptr::copy_nonoverlapping(self.as_ptr().add(at), other.as_mut_ptr(), other.len());
        }

        other
    }

    /// Removes the last element from a vector and returns it, or [`None`] if it
    /// is empty.
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        self.fixed.pop()
    }

    /// Clears the vector, removing all values.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in bump; 1, 2, 3];
    /// vec.clear();
    /// assert!(vec.is_empty());
    /// ```
    #[inline(always)]
    pub fn clear(&mut self) {
        self.fixed.clear();
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
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in bump; 1, 2, 3, 4, 5];
    /// vec.truncate(2);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current
    /// length:
    ///
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in bump; 1, 2, 3];
    /// vec.truncate(8);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in bump; 1, 2, 3];
    /// vec.truncate(0);
    /// assert_eq!(vec, []);
    /// ```
    ///
    /// [`clear`]: BumpVec::clear
    /// [`drain`]: BumpVec::drain
    pub fn truncate(&mut self, len: usize) {
        self.fixed.truncate(len);
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
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut v = bump_vec![in bump; 1, 2, 3];
    /// assert_eq!(v.remove(1), 2);
    /// assert_eq!(v, [1, 3]);
    /// ```
    #[track_caller]
    pub fn remove(&mut self, index: usize) -> T {
        self.fixed.remove(index)
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
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut v = bump_vec![in bump; "foo", "bar", "baz", "qux"];
    ///
    /// assert_eq!(v.swap_remove(1), "bar");
    /// assert_eq!(v, ["foo", "qux", "baz"]);
    ///
    /// assert_eq!(v.swap_remove(0), "foo");
    /// assert_eq!(v, ["baz", "qux"]);
    /// ```
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        self.fixed.swap_remove(index)
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    #[must_use]
    #[inline(always)]
    pub const fn as_slice(&self) -> &[T] {
        self.fixed.as_slice()
    }

    /// Extracts a mutable slice containing the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.fixed.as_mut_slice()
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
        self.fixed.as_non_null_slice()
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn unchecked_push(&mut self, value: T) {
        self.fixed.unchecked_push(value);
    }

    /// Appends an element to the back of the collection.
    ///
    /// # Safety
    /// Vector must not be full.
    #[inline(always)]
    pub unsafe fn unchecked_push_with(&mut self, f: impl FnOnce() -> T) {
        self.fixed.unchecked_push_with(f);
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
    /// [`resize`]: BumpVec::resize
    /// [`truncate`]: BumpVec::truncate
    /// [`extend`]: BumpVec::extend
    /// [`clear`]: BumpVec::clear
    /// [`capacity`]: BumpVec::capacity
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.fixed.set_len(new_len);
    }

    #[inline]
    pub(crate) unsafe fn inc_len(&mut self, amount: usize) {
        self.fixed.inc_len(amount);
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    error_behavior_generic_methods_allocation_failure! {
        /// Appends an element to the back of a collection.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ bump_vec, Bump };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; 1, 2];
        /// vec.push(3);
        /// assert_eq!(vec, [1, 2, 3]);
        /// ```
        for fn push
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ bump_vec, Bump };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; 1, 2]?;
        /// vec.try_push(3);
        /// assert_eq!(vec, [1, 2, 3]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_push
        use fn generic_push(&mut self, value: T) {
            self.generic_push_with(|| value)
        }

        /// Appends an element to the back of a collection.
        impl
        for fn push_with
        for fn try_push_with
        use fn generic_push_with(&mut self, f: impl FnOnce() -> T) {
            self.generic_reserve_one()?;
            unsafe {
                self.unchecked_push_with(f);
            }
            Ok(())
        }

        /// Inserts an element at position `index` within the vector, shifting all elements after it to the right.
        do panics
        /// Panics if `index > len`.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ bump_vec, Bump, BumpVec };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; 1, 2, 3];
        /// vec.insert(1, 4);
        /// assert_eq!(vec, [1, 4, 2, 3]);
        /// vec.insert(4, 5);
        /// assert_eq!(vec, [1, 4, 2, 3, 5]);
        /// ```
        for fn insert
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ bump_vec, Bump, BumpVec };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; 1, 2, 3]?;
        /// vec.try_insert(1, 4)?;
        /// assert_eq!(vec, [1, 4, 2, 3]);
        /// vec.try_insert(4, 5)?;
        /// assert_eq!(vec, [1, 4, 2, 3, 5]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_insert
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

        /// Copies and appends all elements in a slice to the `BumpVec`.
        ///
        /// Iterates over the `slice`, copies each element, and then appends
        /// it to this `BumpVec`. The `slice` is traversed in-order.
        ///
        /// Note that this function is same as [`extend`] except that it is
        /// specialized to work with copyable slices instead.
        ///
        /// [`extend`]: BumpVec::extend
        impl
        for fn extend_from_slice_copy
        for fn try_extend_from_slice_copy
        use fn generic_extend_from_slice_copy(&mut self, slice: &[T])
        where {
            T: Copy
        } in {
            unsafe { self.extend_by_copy_nonoverlapping(slice) }
        }

        /// Clones and appends all elements in a slice to the `BumpVec`.
        ///
        /// Iterates over the `slice`, clones each element, and then appends
        /// it to this `BumpVec`. The `slice` is traversed in-order.
        ///
        /// Note that this function is same as [`extend`] except that it is
        /// specialized to work with slices instead.
        ///
        /// [`extend`]: BumpVec::extend
        impl
        for fn extend_from_slice_clone
        for fn try_extend_from_slice_clone
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

        /// Appends all elements in an array to the `BumpVec`.
        ///
        /// Iterates over the `array`, copies each element, and then appends
        /// it to this `BumpVec`. The `array` is traversed in-order.
        ///
        /// Note that this function is same as [`extend`] except that it is
        /// specialized to work with arrays instead.
        ///
        /// [`extend`]: BumpVec::extend
        #[allow(clippy::needless_pass_by_value)]
        impl
        for fn extend_from_array
        for fn try_extend_from_array
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
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; 0, 1, 2, 3, 4];
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
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; 0, 1, 2, 3, 4]?;
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
        do panics
        /// Panics if the starting point is greater than the end point or if
        /// the end point is greater than the length of the vector.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; 0, 1, 2, 3, 4];
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
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; 0, 1, 2, 3, 4]?;
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
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; 1, 2, 3];
        /// vec.extend_zeroed(2);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// ```
        for fn extend_zeroed
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; 1, 2, 3]?;
        /// vec.try_extend_zeroed(2)?;
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_extend_zeroed
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
        /// in the given `BumpVec<T>`. The collection may reserve more space to
        /// speculatively avoid frequent reallocations. After calling `reserve`,
        /// capacity will be greater than or equal to `self.len() + additional`.
        /// Does nothing if capacity is already sufficient.
        do panics
        /// Panics if the new capacity exceeds `isize::MAX` bytes.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; 1];
        /// vec.reserve(10);
        /// assert!(vec.capacity() >= 11);
        /// ```
        for fn reserve
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; 1]?;
        /// vec.try_reserve(10)?;
        /// assert!(vec.capacity() >= 11);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_reserve
        use fn generic_reserve(&mut self, additional: usize) {
            if additional > (self.capacity() - self.len()) {
                self.generic_grow_cold(additional)?;
            }

            Ok(())
        }

        /// Reserves the minimum capacity for at least `additional` more elements to
        /// be inserted in the given `BumpVec<T>`. Unlike [`reserve`], this will not
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
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; 1];
        /// vec.reserve_exact(10);
        /// assert!(vec.capacity() >= 11);
        /// ```
        for fn reserve_exact
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; 1]?;
        /// vec.try_reserve_exact(10)?;
        /// assert!(vec.capacity() >= 11);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_reserve_exact
        use fn generic_reserve_exact(&mut self, additional: usize) {
            if additional > (self.capacity() - self.len()) {
                self.generic_grow_cold_exact(additional)?;
            }

            Ok(())
        }

        /// Resizes the `BumpVec` in-place so that `len` is equal to `new_len`.
        ///
        /// If `new_len` is greater than `len`, the `BumpVec` is extended by the
        /// difference, with each additional slot filled with `value`.
        /// If `new_len` is less than `len`, the `BumpVec` is simply truncated.
        ///
        /// This method requires `T` to implement [`Clone`],
        /// in order to be able to clone the passed value.
        /// If you need more flexibility (or want to rely on [`Default`] instead of
        /// [`Clone`]), use [`resize_with`].
        /// If you only need to resize to a smaller size, use [`truncate`].
        ///
        /// [`resize_with`]: BumpVec::resize_with
        /// [`truncate`]: BumpBox::truncate
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; "hello"];
        /// vec.resize(3, "world");
        /// assert_eq!(vec, ["hello", "world", "world"]);
        /// drop(vec);
        ///
        /// let mut vec = bump_vec![in bump; 1, 2, 3, 4];
        /// vec.resize(2, 0);
        /// assert_eq!(vec, [1, 2]);
        /// ```
        for fn resize
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; "hello"]?;
        /// vec.try_resize(3, "world")?;
        /// assert_eq!(vec, ["hello", "world", "world"]);
        /// drop(vec);
        ///
        /// let mut vec = bump_vec![try in bump; 1, 2, 3, 4]?;
        /// vec.try_resize(2, 0)?;
        /// assert_eq!(vec, [1, 2]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_resize
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

        /// Resizes the `BumpVec` in-place so that `len` is equal to `new_len`.
        ///
        /// If `new_len` is greater than `len`, the `BumpVec` is extended by the
        /// difference, with each additional slot filled with the result of
        /// calling the closure `f`. The return values from `f` will end up
        /// in the `BumpVec` in the order they have been generated.
        ///
        /// If `new_len` is less than `len`, the `BumpVec` is simply truncated.
        ///
        /// This method uses a closure to create new values on every push. If
        /// you'd rather [`Clone`] a given value, use [`BumpVec::resize`]. If you
        /// want to use the [`Default`] trait to generate values, you can
        /// pass [`Default::default`] as the second argument.
        impl
        do examples
        /// ```
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; 1, 2, 3];
        /// vec.resize_with(5, Default::default);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// drop(vec);
        ///
        /// let mut vec = bump_vec![in bump];
        /// let mut p = 1;
        /// vec.resize_with(4, || { p *= 2; p });
        /// assert_eq!(vec, [2, 4, 8, 16]);
        /// ```
        for fn resize_with
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; 1, 2, 3]?;
        /// vec.try_resize_with(5, Default::default);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        /// drop(vec);
        ///
        /// let mut vec = bump_vec![try in bump]?;
        /// let mut p = 1;
        /// vec.try_resize_with(4, || { p *= 2; p })?;
        /// assert_eq!(vec, [2, 4, 8, 16]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_resize_with
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
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// let mut vec = bump_vec![in bump; 1, 2, 3];
        /// vec.resize_zeroed(5);
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        ///
        /// let mut vec = bump_vec![in bump; 1, 2, 3];
        /// vec.resize_zeroed(2);
        /// assert_eq!(vec, [1, 2]);
        /// ```
        for fn resize_zeroed
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::try_new()?;
        /// let mut vec = bump_vec![try in bump; 1, 2, 3]?;
        /// vec.try_resize_zeroed(5)?;
        /// assert_eq!(vec, [1, 2, 3, 0, 0]);
        ///
        /// let mut vec = bump_vec![try in bump; 1, 2, 3]?;
        /// vec.try_resize_zeroed(2)?;
        /// assert_eq!(vec, [1, 2]);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_resize_zeroed
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
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::new();
        /// // needs a scope because of lifetime shenanigans
        /// let bump = bump.as_scope();
        /// let mut slice = bump.alloc_slice_copy(&[4, 5, 6]);
        /// let mut vec = bump_vec![in bump; 1, 2, 3];
        /// vec.append(&mut slice);
        /// assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
        /// assert_eq!(slice, []);
        /// ```
        for fn append
        do examples
        /// ```
        /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
        /// # use bump_scope::{ Bump, bump_vec };
        /// # let bump: Bump = Bump::try_new()?;
        /// // needs a scope because of lifetime shenanigans
        /// let bump = bump.as_scope();
        /// let mut slice = bump.try_alloc_slice_copy(&[4, 5, 6])?;
        /// let mut vec = bump_vec![try in bump; 1, 2, 3]?;
        /// vec.try_append(&mut slice)?;
        /// assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
        /// assert_eq!(slice, []);
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_append
        use fn generic_append(&mut self, other: &mut BumpBox<[T]>) {
            unsafe {
                self.extend_by_copy_nonoverlapping(other.as_slice())?;
                other.set_len(0);
                Ok(())
            }
        }
    }

    /// Creates a splicing iterator that replaces the specified range in the vector
    /// with the given `replace_with` iterator and yields the removed items.
    /// `replace_with` does not need to be the same length as `range`.
    ///
    /// `range` is removed even if the iterator is not consumed until the end.
    ///
    /// It is unspecified how many elements are removed from the vector
    /// if the `Splice` value is leaked.
    ///
    /// The input iterator `replace_with` is only consumed when the `Splice` value is dropped.
    ///
    /// This is optimal if:
    ///
    /// * The tail (elements in the vector after `range`) is empty,
    /// * or `replace_with` yields fewer or equal elements than `range`’s length
    /// * or the lower bound of its `size_hint()` is exact.
    ///
    /// Otherwise, a temporary vector is allocated and the tail is moved twice.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut v = bump_vec![in bump; 1, 2, 3, 4];
    /// let new = [7, 8, 9];
    /// let u = bump.alloc_iter(v.splice(1..3, new));
    /// assert_eq!(v, &[1, 7, 8, 9, 4]);
    /// assert_eq!(u, &[2, 3]);
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[inline]
    pub fn splice<R, I>(
        &mut self,
        range: R,
        replace_with: I,
    ) -> Splice<'_, I::IntoIter, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = T>,
    {
        // Memory safety
        //
        // When the Drain is first created, it shortens the length of
        // the source vector to make sure no uninitialized or moved-from elements
        // are accessible at all if the Drain's destructor never gets to run.
        //
        // Drain will ptr::read out the values to remove.
        // When finished, remaining tail of the vec is copied back to cover
        // the hole, and the vector length is restored to the new length.
        //

        use core::ops::Range;
        let len = self.len();
        let Range { start, end } = slice::range(range, ..len);

        let drain = unsafe {
            // set self.vec length's to start, to be safe in case Drain is leaked
            self.set_len(start);
            let range_slice = slice::from_raw_parts(self.as_ptr().add(start), end - start);

            Drain {
                tail_start: end,
                tail_len: len - end,
                iter: range_slice.iter(),
                vec: NonNull::from(self),
            }
        };

        Splice {
            drain,
            replace_with: replace_with.into_iter(),
        }
    }

    /// Like [`reserve`] but allows you to provide a different `len`.
    ///
    /// This helps with algorithms from the standard library that make use of
    /// `RawVec::reserve` which behaves the same.
    ///
    /// # Safety
    /// TODO
    #[cfg(not(no_global_oom_handling))]
    #[inline(always)]
    pub(crate) unsafe fn buf_reserve(&mut self, len: usize, additional: usize) {
        use crate::infallible;

        if additional > (self.capacity() - len) {
            infallible(self.generic_grow_cold(additional));
        }
    }

    /// Extend the vector by `n` clones of value.
    fn extend_with<B: ErrorBehavior>(&mut self, n: usize, value: T) -> Result<(), B>
    where
        T: Clone,
    {
        self.generic_reserve(n)?;
        unsafe {
            self.fixed.extend_with_unchecked(n, value);
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
            self.generic_grow_cold::<E>(1)?;
        }

        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn generic_grow_cold<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        let required_cap = match self.len().checked_add(additional) {
            Some(required_cap) => required_cap,
            None => return Err(E::capacity_overflow())?,
        };

        if T::IS_ZST {
            return Ok(());
        }

        if self.capacity() == 0 {
            self.fixed = self.bump.generic_alloc_fixed_vec(required_cap)?;
            return Ok(());
        }

        let new_capacity = self.capacity().checked_mul(2).unwrap_or(required_cap).max(required_cap);
        unsafe { self.generic_grow_to(new_capacity) }
    }

    #[cold]
    #[inline(never)]
    fn generic_grow_cold_exact<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        let required_cap = match self.len().checked_add(additional) {
            Some(required_cap) => required_cap,
            None => return Err(E::capacity_overflow())?,
        };

        if T::IS_ZST {
            return Ok(());
        }

        if self.capacity() == 0 {
            self.fixed = self.bump.generic_alloc_fixed_vec(required_cap)?;
            return Ok(());
        }

        unsafe { self.generic_grow_to(required_cap) }
    }

    /// # Safety
    ///
    /// `new_capacity` must be greater than the current capacity.
    unsafe fn generic_grow_to<E: ErrorBehavior>(&mut self, new_capacity: usize) -> Result<(), E> {
        let old_ptr = self.as_non_null_ptr();
        let new_cap = new_capacity;
        let old_size = self.fixed.capacity * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_cap.checked_mul(T::SIZE).ok_or_else(|| E::capacity_overflow())?;

        if UP {
            let is_last = nonnull::byte_add(old_ptr, old_size).cast() == self.bump.chunk.get().pos();

            if is_last {
                let chunk_end = self.bump.chunk.get().content_end();
                let remaining = nonnull::addr(chunk_end).get() - nonnull::addr(old_ptr).get();

                if new_size <= remaining {
                    // There is enough space! We will grow in place. Just need to update the bump pointer.

                    let old_addr = nonnull::addr(old_ptr);
                    let new_end = old_addr.get() + new_size;

                    // Up-aligning a pointer inside a chunks content by `MIN_ALIGN` never overflows.
                    let new_pos = up_align_usize_unchecked(new_end, MIN_ALIGN);

                    self.bump.chunk.get().set_pos_addr(new_pos);
                } else {
                    // The current chunk doesn't have enough space to allocate this layout. We need to allocate in another chunk.
                    let new_ptr = self.bump.do_alloc_slice_in_another_chunk::<E, T>(new_cap)?.cast();
                    nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                    self.fixed.initialized.set_ptr(new_ptr);
                }
            } else {
                let new_ptr = self.bump.do_alloc_slice::<E, T>(new_cap)?.cast();
                nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                self.fixed.initialized.set_ptr(new_ptr);
            }
        } else {
            let is_last = old_ptr.cast() == self.bump.chunk.get().pos();

            if is_last {
                // We may be able to reuse the currently allocated space. Just need to check if the current chunk has enough space for that.
                let additional_size = new_size - old_size;

                let old_addr = nonnull::addr(old_ptr);
                let new_addr = bump_down(old_addr, additional_size, T::ALIGN.max(MIN_ALIGN));

                let very_start = nonnull::addr(self.bump.chunk.get().content_start());

                if new_addr >= very_start.get() {
                    // There is enough space in the current chunk! We will reuse the allocated space.

                    let new_addr = NonZeroUsize::new_unchecked(new_addr);
                    let new_addr_end = new_addr.get() + new_size;

                    let new_ptr = nonnull::with_addr(old_ptr, new_addr);

                    // Check if the regions don't overlap so we may use the faster `copy_nonoverlapping`.
                    if new_addr_end < old_addr.get() {
                        nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                    } else {
                        nonnull::copy::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                    }

                    self.bump.chunk.get().set_pos(new_ptr.cast());
                    self.fixed.initialized.set_ptr(new_ptr);
                } else {
                    // The current chunk doesn't have enough space to allocate this layout. We need to allocate in another chunk.
                    let new_ptr = self.bump.do_alloc_slice_in_another_chunk::<E, T>(new_cap)?.cast();
                    nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                    self.fixed.initialized.set_ptr(new_ptr);
                }
            } else {
                let new_ptr = self.bump.do_alloc_slice::<E, T>(new_cap)?.cast();
                nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                self.fixed.initialized.set_ptr(new_ptr);
            }
        }

        self.fixed.capacity = new_cap;
        Ok(())
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// This will also free space for future bump allocations iff this is the most recent allocation.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, BumpVec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = BumpVec::with_capacity_in(10, &bump);
    /// vec.extend([1, 2, 3]);
    /// assert!(vec.capacity() == 10);
    /// assert_eq!(bump.stats().allocated(), 10 * 4);
    /// vec.shrink_to_fit();
    /// assert!(vec.capacity() == 3);
    /// assert_eq!(bump.stats().allocated(), 3 * 4);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        let old_ptr = self.as_non_null_ptr().cast::<u8>();
        let old_size = self.fixed.capacity * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = self.len() * T::SIZE; // its less than the capacity so this can't overflow

        // Adapted from `Allocator::shrink`.
        unsafe {
            let is_last = if UP {
                old_ptr.as_ptr().add(old_size) == self.bump.chunk.get().pos().as_ptr()
            } else {
                old_ptr == self.bump.chunk.get().pos()
            };

            // if that's not the last allocation, there is nothing we can do
            if !is_last {
                return;
            }

            if UP {
                let end = nonnull::addr(old_ptr).get() + new_size;

                // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(end, MIN_ALIGN);

                self.bump.chunk.get().set_pos_addr(new_pos);
            } else {
                let old_addr = nonnull::addr(old_ptr);
                let old_addr_old_end = NonZeroUsize::new_unchecked(old_addr.get() + old_size);

                let new_addr = bump_down(old_addr_old_end, new_size, T::ALIGN.max(MIN_ALIGN));
                let new_addr = NonZeroUsize::new_unchecked(new_addr);
                let old_addr_new_end = NonZeroUsize::new_unchecked(old_addr.get() + new_size);

                let new_ptr = nonnull::with_addr(old_ptr, new_addr);
                let overlaps = old_addr_new_end > new_addr;

                if overlaps {
                    nonnull::copy(old_ptr, new_ptr, new_size);
                } else {
                    nonnull::copy_nonoverlapping(old_ptr, new_ptr, new_size);
                }

                self.bump.chunk.get().set_pos(new_ptr);
                self.fixed.initialized.set_ptr(new_ptr.cast());
            }

            self.fixed.capacity = self.len();
        }
    }

    /// Turns this `BumpVec<T>` into a `FixedBumpVec<T>`.
    ///
    /// This retains the unused capacity unlike <code>[into_](Self::into_slice)([boxed_](Self::into_boxed_slice))[slice](Self::into_slice)</code>.
    #[must_use]
    #[inline(always)]
    pub fn into_fixed_vec(self) -> FixedBumpVec<'a, T> {
        self.into_parts().0
    }

    /// Turns this `BumpVec<T>` into a `BumpBox<[T]>`.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_slice(mut self) -> BumpBox<'a, [T]> {
        self.shrink_to_fit();
        self.into_fixed_vec().into_boxed_slice()
    }

    /// Turns this `BumpVec<T>` into a `&[T]` that is live for this bump scope.
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

    /// Creates a `BumpVec<T>` from its parts.
    ///
    /// The provided `bump` does not have to be the one the `fixed_vec` was allocated in.
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpVec };
    /// # let bump: Bump = Bump::new();
    /// let mut fixed_vec = bump.alloc_fixed_vec(3);
    /// fixed_vec.push(1);
    /// fixed_vec.push(2);
    /// fixed_vec.push(3);
    /// let mut vec = BumpVec::from_parts(fixed_vec, &bump);
    /// vec.push(4);
    /// vec.push(5);
    /// assert_eq!(vec, [1, 2, 3, 4, 5]);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn from_parts(
        fixed_vec: FixedBumpVec<'a, T>,
        bump: impl Into<&'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>,
    ) -> Self {
        Self {
            fixed: fixed_vec,
            bump: bump.into(),
        }
    }

    /// Turns this `BumpVec<T>` into its parts.
    /// ```
    /// # use bump_scope::{ Bump, BumpVec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = BumpVec::new_in(&bump);
    /// vec.reserve(3);
    /// vec.push(1);
    /// let mut fixed_vec = vec.into_parts().0;
    /// assert_eq!(fixed_vec.capacity(), 3);
    /// assert_eq!(fixed_vec, [1]);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn into_parts(self) -> (FixedBumpVec<'a, T>, &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) {
        let mut this = ManuallyDrop::new(self);
        let bump = this.bump;
        let fixed = mem::take(&mut this.fixed);
        (fixed, bump)
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
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in bump; 1, 2, 3, 4];
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
        self.fixed.retain(f)
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
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut v = bump_vec![in bump; 1, 2, 3];
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
        self.fixed.drain(range)
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
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let some_predicate = |x: &mut i32| { *x == 2 || *x == 3 || *x == 6 };
    /// # let bump: Bump = Bump::new();
    /// # let mut vec = bump_vec![in bump; 1, 2, 3, 4, 5, 6];
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
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut numbers = bump_vec![in bump; 1, 2, 3, 4, 5, 6, 8, 9, 11, 13, 14, 15];
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
        self.fixed.extract_if(filter)
    }

    /// Removes consecutive repeated elements in the vector according to the
    /// [`PartialEq`] trait implementation.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in bump; 1, 2, 2, 3, 2];
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
        self.fixed.dedup();
    }

    /// Removes all but the first of consecutive elements in the vector that resolve to the same
    /// key.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in bump; 10, 20, 21, 30, 20];
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
        self.fixed.dedup_by_key(key);
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
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in bump; "foo", "bar", "Bar", "baz", "bar"];
    ///
    /// vec.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    ///
    /// assert_eq!(vec, ["foo", "bar", "baz", "bar"]);
    /// ```
    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        self.fixed.dedup_by(same_bucket);
    }

    /// Returns the remaining spare capacity of the vector as a slice of
    /// `MaybeUninit<T>`.
    ///
    /// The returned slice can be used to fill the vector with data (e.g. by
    /// reading from a file) before marking the data as initialized using the
    /// [`set_len`] method.
    ///
    /// [`set_len`]: Vec::set_len
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpVec };
    /// # let bump: Bump = Bump::new();
    /// // Allocate vector big enough for 10 elements.
    /// let mut v = BumpVec::with_capacity_in(10, &bump);
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
    /// [`set_len`]: BumpBox::set_len
    ///
    /// Note that this is a low-level API, which should be used with care for
    /// optimization purposes. If you need to append data to a `BumpVec`
    /// you can use [`push`], [`extend`], `extend_from_slice`[`_copy`](BumpVec::extend_from_slice_copy)`/`[`_clone`](BumpVec::extend_from_within_clone),
    /// `extend_from_within`[`_copy`](BumpVec::extend_from_within_copy)`/`[`_clone`](BumpVec::extend_from_within_clone), [`insert`], [`resize`] or
    /// [`resize_with`], depending on your exact needs.
    ///
    /// [`push`]: BumpVec::push
    /// [`extend`]: BumpVec::extend
    /// [`insert`]: BumpVec::insert
    /// [`append`]: BumpVec::append
    /// [`resize`]: BumpVec::resize
    /// [`resize_with`]: BumpVec::resize_with
    #[inline]
    pub fn split_at_spare_mut(&mut self) -> (&mut [T], &mut [MaybeUninit<T>]) {
        let ptr = self.as_mut_ptr();

        // SAFETY:
        // - `ptr` is [guaranteed allocated]o be valid for `self.len` elements
        // - but the allocation extends out to `self.buf.capacity()` elements, possibly
        // uninitialized
        let spare_ptr = unsafe { ptr.add(self.len()) };
        let spare_ptr = spare_ptr.cast::<MaybeUninit<T>>();
        let spare_len = self.capacity() - self.len();

        // SAFETY:
        // - `ptr` is [guaranteed allocated]o be valid for `self.len` elements
        // - `spare_ptr` is pointing one element past the buffer, so it doesn't overlap with `initialized`
        unsafe {
            let initialized = slice::from_raw_parts_mut(ptr, self.len());
            let spare = slice::from_raw_parts_mut(spare_ptr, spare_len);

            (initialized, spare)
        }
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        self.bump.allocator()
    }

    /// Returns a reference to the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn bump(&self) -> &'b BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        self.bump
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, UP> {
        self.bump.stats()
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool> BumpVec<'b, 'a, T, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn guaranteed_allocated_stats(&self) -> GuaranteedAllocatedStats<'a, UP> {
        self.bump.guaranteed_allocated_stats()
    }
}

impl<'b, 'a: 'b, T, const N: usize, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    BumpVec<'b, 'a, [T; N], A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Takes a `BumpVec<[T; N]>` and flattens it into a `BumpVec<T>`.
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
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in bump; [1, 2, 3], [4, 5, 6], [7, 8, 9]];
    /// assert_eq!(vec.pop(), Some([7, 8, 9]));
    ///
    /// let mut flattened = vec.into_flattened();
    /// assert_eq!(flattened.pop(), Some(6));
    /// ```
    #[must_use]
    pub fn into_flattened(self) -> BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        let (fixed, bump) = self.into_parts();
        let fixed = fixed.into_flattened();
        BumpVec { fixed, bump }
    }
}

impl<'b, 'a: 'b, T: Debug, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Debug
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_slice(), f)
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, I: SliceIndex<[T]>> Index<I>
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self.as_slice(), index)
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, I: SliceIndex<[T]>>
    IndexMut<I> for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self.as_mut_slice(), index)
    }
}

#[cfg(not(no_global_oom_handling))]
impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Extend<T>
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + 'a,
{
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
impl<'b, 'a: 'b, 't, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Extend<&'t T>
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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
    PartialEq<BumpVec<'b1, 'a1, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>
    for BumpVec<'b0, 'a0, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &BumpVec<'b1, 'a1, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &BumpVec<'b1, 'a1, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'b, 'a: 'b, T, U, const N: usize, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<[U; N]> for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
    PartialEq<&[U; N]> for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
    PartialEq<&mut [U; N]> for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,

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
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
    PartialEq<BumpVec<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for [T]
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &BumpVec<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &BumpVec<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'b, 'a: 'b, T, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<BumpVec<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for &[T]
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &BumpVec<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &BumpVec<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'b, 'a: 'b, T, U, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A>
    PartialEq<BumpVec<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for &mut [T]
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &BumpVec<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &BumpVec<'b, 'a, U, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> IntoIterator
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Item = T;
    type IntoIter = IntoIter<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            let me = ManuallyDrop::new(self);

            let begin = me.as_non_null_ptr();
            let end = if T::IS_ZST {
                nonnull::wrapping_byte_add(begin, me.len())
            } else {
                nonnull::add(begin, me.len())
            };

            IntoIter {
                buf: begin,
                cap: me.capacity(),

                ptr: begin,
                end,

                bump: me.bump,
                marker: PhantomData,
            }
        }
    }
}

impl<'c, 'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> IntoIterator
    for &'c BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Item = &'c T;
    type IntoIter = slice::Iter<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'c, 'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> IntoIterator
    for &'c mut BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Item = &'c mut T;
    type IntoIter = slice::IterMut<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AsRef<[T]>
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AsMut<[T]>
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Borrow<[T]>
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BorrowMut<[T]>
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'b, 'a: 'b, T: Hash, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> Hash
    for BumpVec<'b, 'a, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

/// Returns [`ErrorKind::OutOfMemory`](std::io::ErrorKind::OutOfMemory) when allocations fail.
#[cfg(feature = "std")]
impl<'b, 'a: 'b, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> std::io::Write
    for BumpVec<'b, 'a, u8, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + 'a,
{
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

        if self.try_reserve(len).is_err() {
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
        if self.try_extend_from_slice_copy(buf).is_err() {
            return Err(std::io::ErrorKind::OutOfMemory.into());
        }

        Ok(())
    }
}
