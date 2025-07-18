use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    hash::Hash,
    iter,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut, Index, IndexMut, RangeBounds},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::{self, NonNull},
    slice::SliceIndex,
};

use crate::{
    alloc::AllocError,
    collection_method_allocator_stats,
    destructure::destructure,
    min_non_zero_cap,
    owned_slice::{self, OwnedSlice, TakeOwnedSlice},
    polyfill::{hint::likely, non_null, pointer, slice},
    raw_fixed_bump_vec::RawFixedBumpVec,
    BumpAllocator, BumpAllocatorScope, BumpBox, ErrorBehavior, FixedBumpVec, NoDrop, SizedTypeProperties,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

mod drain;
mod into_iter;
mod splice;

pub use into_iter::IntoIter;

#[cfg(feature = "panic-on-alloc")]
pub(crate) use drain::Drain;

#[cfg(feature = "panic-on-alloc")]
pub use splice::Splice;

/// This is like [`vec!`](alloc_crate::vec!) but allocates inside a bump allocator, returning a [`BumpVec`].
///
/// `$bump` can be any type that implements [`BumpAllocator`].
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
/// # use bump_scope::{Bump, bump_vec, BumpVec};
/// # let bump: Bump = Bump::new();
/// let vec: BumpVec<i32, _> = bump_vec![in &bump];
/// assert!(vec.is_empty());
/// ```
///
/// - Create a [`BumpVec`] containing a given list of elements:
///
/// ```
/// # use bump_scope::{Bump, bump_vec};
/// # let bump: Bump = Bump::new();
/// let vec = bump_vec![in &bump; 1, 2, 3];
/// assert_eq!(vec[0], 1);
/// assert_eq!(vec[1], 2);
/// assert_eq!(vec[2], 3);
/// ```
///
/// - Create a [`BumpVec`] from a given element and size:
///
/// ```
/// # use bump_scope::{Bump, bump_vec};
/// # let bump: Bump = Bump::new();
/// let vec = bump_vec![in &bump; 1; 3];
/// assert_eq!(vec, [1, 1, 1]);
/// ```
///
/// Note that unlike array expressions this syntax supports all elements
/// which implement [`Clone`] and the number of elements doesn't have to be
/// a constant.
///
/// This will use `clone` to duplicate an expression, so one should be careful
/// using this with types having a nonstandard `Clone` implementation. For
/// example, `bump_vec![in &bump; Rc::new(1); 5]` will create a vector of five references
/// to the same boxed integer value, not five references pointing to independently
/// boxed integers.
///
/// Also, note that `bump_vec![in &bump; expr; 0]` is allowed, and produces an empty vector.
/// This will still evaluate `expr`, however, and immediately drop the resulting value, so
/// be mindful of side effects.
#[macro_export]
macro_rules! bump_vec {
    [in $bump:expr] => {
        $crate::BumpVec::new_in($bump)
    };
    [in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::BumpVec::from_owned_slice_in([$($values),*], $bump)
    };
    [in $bump:expr; $value:expr; $count:expr] => {
        $crate::BumpVec::from_elem_in($value, $count, $bump)
    };
    [try in $bump:expr] => {
        Ok::<_, $crate::alloc::AllocError>($crate::BumpVec::new_in($bump))
    };
    [try in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::BumpVec::try_from_owned_slice_in([$($values),*], $bump)
    };
    [try in $bump:expr; $value:expr; $count:expr] => {
        $crate::BumpVec::try_from_elem_in($value, $count, $bump)
    };
}

/// A bump allocated [`Vec`](alloc_crate::vec::Vec).
///
/// The main difference to `Vec` is that it can be turned into a slice that is live for this bump scope (`'a`).
/// Such a slice can be live while entering new scopes.
///
/// This would not be possible with `Vec`:
///
/// ```
/// # use bump_scope::{Bump, BumpVec};
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
///     # _ = bump;
/// });
///
/// assert_eq!(slice, [1, 2, 3]);
/// ```
///
/// # Examples
///
/// This type can be used to allocate a slice, when `alloc_*` methods are too limiting:
/// ```
/// # use bump_scope::{Bump, BumpVec};
/// # let bump: Bump = Bump::new();
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
pub struct BumpVec<T, A: BumpAllocator> {
    fixed: RawFixedBumpVec<T>,
    allocator: A,
}

impl<T: UnwindSafe, A: BumpAllocator + UnwindSafe> UnwindSafe for BumpVec<T, A> {}
impl<T: RefUnwindSafe, A: BumpAllocator + RefUnwindSafe> RefUnwindSafe for BumpVec<T, A> {}

impl<T, A: BumpAllocator> Deref for BumpVec<T, A> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { self.fixed.cook_ref() }
    }
}

impl<T, A: BumpAllocator> DerefMut for BumpVec<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.fixed.cook_mut() }
    }
}

impl<T, A: BumpAllocator> BumpVec<T, A> {
    /// # Safety
    ///
    /// Must only be called from the drop implementation and a call to this function
    /// must be the only thing in that drop implementation.
    #[inline]
    unsafe fn drop_inner(&mut self) {
        struct DropGuard<'a, T, A: BumpAllocator>(&'a mut BumpVec<T, A>);

        impl<T, A: BumpAllocator> Drop for DropGuard<'_, T, A> {
            fn drop(&mut self) {
                // SAFETY:
                // Calling `deallocate` with a dangling pointer is fine because
                // - `layout.size()` will be `0` which will make it have no effect in the allocator
                // - calling deallocate with a pointer not owned by this allocator is explicitly allowed; see `BumpAllocator`
                unsafe {
                    let ptr = self.0.fixed.as_non_null().cast();
                    let layout = Layout::from_size_align_unchecked(self.0.fixed.capacity() * T::SIZE, T::ALIGN);
                    self.0.allocator.deallocate(ptr, layout);
                }
            }
        }

        let guard = DropGuard(self);

        // destroy the remaining elements
        guard.0.clear();

        // now `guard` will be dropped and deallocate the memory
    }
}

#[cfg(feature = "nightly-dropck-eyepatch")]
unsafe impl<#[may_dangle] T, A: BumpAllocator> Drop for BumpVec<T, A> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.drop_inner() }
    }
}

#[cfg(not(feature = "nightly-dropck-eyepatch"))]
impl<T, A: BumpAllocator> Drop for BumpVec<T, A> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.drop_inner() }
    }
}

impl<T, A: BumpAllocator + Default> Default for BumpVec<T, A> {
    fn default() -> Self {
        Self::new_in(A::default())
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<T: Clone, A: BumpAllocator + Clone> Clone for BumpVec<T, A> {
    fn clone(&self) -> Self {
        let allocator = self.allocator.clone();
        let ptr = allocator.allocate_slice::<MaybeUninit<T>>(self.len());
        let slice = non_null::slice_from_raw_parts(ptr, self.len());
        let boxed = unsafe { BumpBox::from_raw(slice) };
        let boxed = boxed.init_clone(self);
        let fixed = FixedBumpVec::from_init(boxed);
        let fixed = unsafe { RawFixedBumpVec::from_cooked(fixed) };
        Self { fixed, allocator }
    }
}

impl<T, A: BumpAllocator> BumpVec<T, A> {
    /// Constructs a new empty `BumpVec<T>`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec = BumpVec::<i32, _>::new_in(&bump);
    /// assert_eq!(vec.len(), 0);
    /// assert_eq!(vec.capacity(), 0);
    /// ```
    #[inline]
    pub const fn new_in(allocator: A) -> Self {
        Self {
            fixed: RawFixedBumpVec::EMPTY,
            allocator,
        }
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
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = BumpVec::<i32, _>::with_capacity_in(10, &bump);
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
    /// // A vector of a zero-sized type will always over-allocate, since no
    /// // allocation is necessary
    /// let vec_units = BumpVec::<(), _>::with_capacity_in(10, &bump);
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
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = BumpVec::<i32, _>::try_with_capacity_in(10, &bump)?;
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
    /// // A vector of a zero-sized type will always over-allocate, since no
    /// // allocation is necessary
    /// let vec_units = BumpVec::<(), _>::try_with_capacity_in(10, &bump)?;
    /// assert_eq!(vec_units.capacity(), usize::MAX);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_capacity_in(capacity: usize, allocator: A) -> Result<Self, AllocError> {
        Self::generic_with_capacity_in(capacity, allocator)
    }

    #[inline]
    pub(crate) fn generic_with_capacity_in<E: ErrorBehavior>(capacity: usize, allocator: A) -> Result<Self, E> {
        if T::IS_ZST || capacity == 0 {
            return Ok(Self {
                fixed: RawFixedBumpVec::EMPTY,
                allocator,
            });
        }

        Ok(Self {
            fixed: unsafe { RawFixedBumpVec::allocate(&allocator, capacity)? },
            allocator,
        })
    }

    /// Constructs a new `BumpVec<T>` and pushes `value` `count` times.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec = BumpVec::from_elem_in("ho", 3, &bump);
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

    /// Constructs a new `BumpVec<T>` and pushes `value` `count` times.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let vec = BumpVec::try_from_elem_in("ho", 3, &bump)?;
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

    /// Constructs a new `BumpVec<T>` from an [`OwnedSlice`].
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// // by value
    /// let a = BumpVec::from_owned_slice_in([1, 2], &bump);
    /// let b = BumpVec::from_owned_slice_in(vec![3, 4], &bump);
    /// let c = BumpVec::from_owned_slice_in(bump.alloc_iter(5..=6), &bump);
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = BumpVec::from_owned_slice_in(&mut other, &bump);
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

    /// Constructs a new `BumpVec<T>` from an [`OwnedSlice`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// // by value
    /// let a = BumpVec::try_from_owned_slice_in([1, 2], &bump)?;
    /// let b = BumpVec::try_from_owned_slice_in(vec![3, 4], &bump)?;
    /// let c = BumpVec::try_from_owned_slice_in(bump.alloc_iter(5..=6), &bump)?;
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = BumpVec::try_from_owned_slice_in(&mut other, &bump)?;
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

    /// Constructs a new `BumpVec<T>` from a `[T; N]`.
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

    /// Constructs a new `BumpVec<T>` from a `[T; N]`.
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

        if T::IS_ZST {
            return Ok(Self {
                fixed: unsafe { RawFixedBumpVec::new_zst(N) },
                allocator,
            });
        }

        if N == 0 {
            return Ok(Self {
                fixed: RawFixedBumpVec::EMPTY,
                allocator,
            });
        }

        let mut fixed = unsafe { RawFixedBumpVec::allocate(&allocator, N)? };

        let src = array.as_ptr();
        let dst = fixed.as_mut_ptr();

        unsafe {
            ptr::copy_nonoverlapping(src, dst, N);
            fixed.set_len(N);
        }

        Ok(Self { fixed, allocator })
    }

    /// Create a new [`BumpVec`] whose elements are taken from an iterator and allocated in the given `bump`.
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
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec = BumpVec::from_iter_in([1, 2, 3], &bump);
    /// assert_eq!(vec, [1, 2, 3]);
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

    /// Create a new [`BumpVec`] whose elements are taken from an iterator and allocated in the given `bump`.
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
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let vec = BumpVec::try_from_iter_in([1, 2, 3], &bump)?;
    /// assert_eq!(vec, [1, 2, 3]);
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

    /// Create a new [`BumpVec`] whose elements are taken from an iterator and allocated in the given `bump`.
    ///
    /// This is just like [`from_iter_in`](Self::from_iter_in) but optimized for an [`ExactSizeIterator`].
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec = BumpVec::from_iter_exact_in([1, 2, 3], &bump);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn from_iter_exact_in<I>(iter: I, allocator: A) -> Self
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        panic_on_error(Self::generic_from_iter_exact_in(iter, allocator))
    }

    /// Create a new [`BumpVec`] whose elements are taken from an iterator and allocated in the given `bump`.
    ///
    /// This is just like [`from_iter_in`](Self::from_iter_in) but optimized for an [`ExactSizeIterator`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let vec = BumpVec::try_from_iter_exact_in([1, 2, 3], &bump)?;
    /// assert_eq!(vec, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_from_iter_exact_in<I>(iter: I, allocator: A) -> Result<Self, AllocError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::generic_from_iter_exact_in(iter, allocator)
    }

    #[inline]
    pub(crate) fn generic_from_iter_exact_in<E: ErrorBehavior, I>(iter: I, allocator: A) -> Result<Self, E>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let mut iter = iter.into_iter();
        let len = iter.len();

        let mut vec = Self::generic_with_capacity_in(len, allocator)?;

        while vec.len() != vec.capacity() {
            match iter.next() {
                // SAFETY: we checked above that `len != capacity`, so there is space
                Some(value) => unsafe { vec.push_unchecked(value) },
                None => break,
            }
        }

        Ok(vec)
    }

    /// Returns the total number of elements the vector can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec = BumpVec::<i32, _>::with_capacity_in(2048, &bump);
    /// assert!(vec.capacity() >= 2048);
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        self.fixed.capacity()
    }

    /// Returns the number of elements in the vector, also referred to
    /// as its 'length'.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let a = bump_vec![in &bump; 1, 2, 3];
    /// assert_eq!(a.len(), 3);
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.fixed.len()
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut v = BumpVec::new_in(&bump);
    /// assert!(v.is_empty());
    ///
    /// v.push(1);
    /// assert!(!v.is_empty());
    /// ```
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.fixed.len() == 0
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
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// # let mut vec = BumpVec::from_owned_slice_in(['a', 'b', 'c', 'd', 'e'], &bump);
    /// # let start = 1;
    /// # let end = 4;
    /// let mut other = BumpVec::new_in(*vec.allocator());
    /// other.append(vec.drain(start..end));
    /// # assert_eq!(vec, ['a', 'e']);
    /// # assert_eq!(other, ['b', 'c', 'd']);
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
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = BumpVec::with_capacity_in(10, &bump);
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
    #[allow(clippy::return_self_not_must_use)]
    pub fn split_off(&mut self, range: impl RangeBounds<usize>) -> Self
    where
        A: Clone,
    {
        let other = unsafe { self.fixed.cook_mut() }.split_off(range);

        Self {
            fixed: unsafe { RawFixedBumpVec::from_cooked(other) },
            allocator: self.allocator.clone(),
        }
    }

    /// Removes the last element from a vector and returns it, or [`None`] if it
    /// is empty.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// assert_eq!(vec.pop(), Some(3));
    /// assert_eq!(vec, [1, 2]);
    /// ```
    ///
    /// # Time complexity
    /// Takes *O*(1) time.
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        unsafe { self.fixed.cook_mut() }.pop()
    }

    /// Removes and returns the last element from a vector if the predicate
    /// returns `true`, or [`None`] if the predicate returns false or the vector
    /// is empty (the predicate will not be called in that case).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2, 3, 4];
    /// let pred = |x: &mut i32| *x % 2 == 0;
    ///
    /// assert_eq!(vec.pop_if(pred), Some(4));
    /// assert_eq!(vec, [1, 2, 3]);
    /// assert_eq!(vec.pop_if(pred), None);
    /// ```
    pub fn pop_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        let last = self.last_mut()?;
        if predicate(last) {
            self.pop()
        } else {
            None
        }
    }

    /// Clears the vector, removing all values.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in &bump; 1, 2, 3, 4, 5];
    /// vec.truncate(2);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current
    /// length:
    ///
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.truncate(8);
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
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
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut v = bump_vec![in &bump; 1, 2, 3];
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
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut v = bump_vec![in &bump; "foo", "bar", "baz", "qux"];
    ///
    /// assert_eq!(v.swap_remove(1), "bar");
    /// assert_eq!(v, ["foo", "qux", "baz"]);
    ///
    /// assert_eq!(v.swap_remove(0), "foo");
    /// assert_eq!(v, ["baz", "qux"]);
    /// ```
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
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// // Allocate vector big enough for 4 elements.
    /// let size = 4;
    /// let mut x: BumpVec<i32, _> = BumpVec::with_capacity_in(size, &bump);
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
    ///     let v = bump_vec![in &bump; 0];
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
        self.fixed.as_non_null()
    }

    /// Returns a `NonNull` pointer to the vector's buffer, or a dangling
    /// `NonNull` pointer valid for zero sized reads if the vector didn't allocate.
    #[doc(hidden)]
    #[deprecated = "renamed to `as_non_null`"]
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_ptr(&self) -> NonNull<T> {
        self.fixed.as_non_null()
    }

    /// Returns a `NonNull` pointer to the vector's buffer, or a dangling
    /// `NonNull` pointer valid for zero sized reads if the vector didn't allocate.
    #[doc(hidden)]
    #[deprecated = "too niche; compute this yourself if needed"]
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_slice(&self) -> NonNull<[T]> {
        #[allow(deprecated)]
        self.fixed.as_non_null_slice()
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

    /// Appends an element to the back of a collection.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{bump_vec, Bump};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2];
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
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{bump_vec, Bump};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; 1, 2]?;
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
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2];
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
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2];
    /// vec.try_push_with(|| 3)?;
    /// assert_eq!(vec, [1, 2, 3]);
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
    /// Panics if the allocation fails.
    ///
    /// Panics if `index > len`.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
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

    /// Copies and appends all elements in a slice to the `BumpVec`.
    ///
    /// Iterates over the `slice`, copies each element, and then appends
    /// it to this `BumpVec`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with copyable slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1];
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

    /// Copies and appends all elements in a slice to the `BumpVec`.
    ///
    /// Iterates over the `slice`, copies each element, and then appends
    /// it to this `BumpVec`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with copyable slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![try in &bump; 1]?;
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

    /// Clones and appends all elements in a slice to the `BumpVec`.
    ///
    /// Iterates over the `slice`, clones each element, and then appends
    /// it to this `BumpVec`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use std::string::String;
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; String::from("a")];
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

    /// Clones and appends all elements in a slice to the `BumpVec`.
    ///
    /// Iterates over the `slice`, clones each element, and then appends
    /// it to this `BumpVec`. The `slice` is traversed in-order.
    ///
    /// Note that this function is same as [`extend`] except that it is
    /// specialized to work with slices instead.
    ///
    /// [`extend`]: Self::extend
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use std::string::String;
    /// # use bump_scope::{ Bump, bump_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![try in &bump; String::from("a")]?;
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
    /// Panics if the allocation fails.
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 0, 1, 2, 3, 4];
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
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; 0, 1, 2, 3, 4]?;
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
    /// Panics if the allocation fails.
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 0, 1, 2, 3, 4];
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
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; 0, 1, 2, 3, 4]?;
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

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `BumpVec<T>`. The collection may reserve more space to
    /// speculatively avoid frequent reallocations. After calling `reserve`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1];
    /// vec.reserve(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve(&mut self, additional: usize) {
        panic_on_error(self.generic_reserve(additional));
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `BumpVec<T>`. The collection may reserve more space to
    /// speculatively avoid frequent reallocations. After calling `reserve`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; 1]?;
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
    /// be inserted in the given `BumpVec<T>`. Unlike [`reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `reserve_exact`, capacity will be greater than or equal to
    /// `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity cannot be relied upon to be precisely
    /// minimal. Prefer [`reserve`] if future insertions are expected.
    ///
    /// [`reserve`]: Self::reserve
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1];
    /// vec.reserve_exact(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve_exact(&mut self, additional: usize) {
        panic_on_error(self.generic_reserve_exact(additional));
    }

    /// Reserves the minimum capacity for at least `additional` more elements to
    /// be inserted in the given `BumpVec<T>`. Unlike [`reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `reserve_exact`, capacity will be greater than or equal to
    /// `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity cannot be relied upon to be precisely
    /// minimal. Prefer [`reserve`] if future insertions are expected.
    ///
    /// [`reserve`]: Self::reserve
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; 1]?;
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
    /// [`resize_with`]: Self::resize_with
    /// [`truncate`]: Self::truncate
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; "hello"];
    /// vec.resize(3, "world");
    /// assert_eq!(vec, ["hello", "world", "world"]);
    /// drop(vec);
    ///
    /// let mut vec = bump_vec![in &bump; 1, 2, 3, 4];
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
    /// [`resize_with`]: Self::resize_with
    /// [`truncate`]: Self::truncate
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; "hello"]?;
    /// vec.try_resize(3, "world")?;
    /// assert_eq!(vec, ["hello", "world", "world"]);
    /// drop(vec);
    ///
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3, 4]?;
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
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.resize_with(5, Default::default);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// drop(vec);
    ///
    /// let mut vec = bump_vec![in &bump];
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
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
    /// vec.try_resize_with(5, Default::default)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// drop(vec);
    ///
    /// let mut vec = bump_vec![try in &bump]?;
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
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = BumpVec::new_in(&bump);
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
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = BumpVec::new_in(&bump);
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

    /// Returns a vector of the same size as `self`, with function `f` applied to each element in order.
    ///
    /// Compared to `from_iter_in(into_iter().map(f), ...)` this method has the advantage that it can reuse the existing allocation.
    ///
    /// # Panics
    /// Panics if the allocation fails. An allocation only occurs when the alignment or size of `U` is greater than that of `T`.
    ///
    /// # Examples
    /// Mapping to a type with an equal alignment and size (no allocation):
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # use core::num::NonZero;
    /// # let bump: Bump = Bump::new();
    /// let a = BumpVec::from_iter_exact_in([0, 1, 2], &bump);
    /// let b = a.map(NonZero::new);
    /// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
    /// ```
    ///
    /// Mapping to a type with a smaller alignment and size (no allocation, capacity may grow):
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec_a: BumpVec<u32, _> = BumpVec::from_iter_in([1, 2, 3, 4], &bump);
    /// assert_eq!(vec_a.capacity(), 4);
    /// assert_eq!(bump.stats().allocated(), 4 * 4);
    ///
    /// let vec_b: BumpVec<u16, _> = vec_a.map(|i| i as u16);
    /// assert_eq!(vec_b.capacity(), 8);
    /// assert_eq!(bump.stats().allocated(), 4 * 4);
    /// ```
    ///
    /// Mapping to a type with a higher alignment or size is equivalent to
    /// calling `from_iter_in(into_iter().map(f), ...)`:
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let vec_a: BumpVec<u16, _> = BumpVec::from_iter_in([1, 2, 3, 4], &bump);
    /// assert_eq!(vec_a.capacity(), 4);
    /// assert_eq!(bump.stats().allocated(), 4 * 2);
    ///
    /// let vec_b: BumpVec<u32, _> = vec_a.map(|i| i as u32);
    /// assert_eq!(vec_b.capacity(), 4);
    /// assert_eq!(bump.stats().allocated(), 4 * 2 + 4 * 4);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> BumpVec<U, A>
    where
        A: Clone,
    {
        panic_on_error(self.generic_map(f))
    }

    /// Returns a vector of the same size as `self`, with function `f` applied to each element in order.
    ///
    /// Compared to `try_from_iter_in(into_iter().map(f), ...)` this method has the advantage that it can reuse the existing allocation.
    ///
    /// # Errors
    /// Errors if the allocation fails. An allocation can only occurs when the alignment or size of `U` is greater than that of `T`.
    ///
    /// # Examples
    /// Mapping to a type with an equal alignment and size (no allocation):
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # use core::num::NonZero;
    /// # let bump: Bump = Bump::try_new()?;
    /// let a = BumpVec::try_from_iter_exact_in([0, 1, 2], &bump)?;
    /// let b = a.try_map(NonZero::new)?;
    /// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Mapping to a type with a smaller alignment and size (no allocation, capacity may grow):
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let vec_a: BumpVec<u32, _> = BumpVec::try_from_iter_in([1, 2, 3, 4], &bump)?;
    /// assert_eq!(vec_a.capacity(), 4);
    /// assert_eq!(bump.stats().allocated(), 4 * 4);
    ///
    /// let vec_b: BumpVec<u16, _> = vec_a.try_map(|i| i as u16)?;
    /// assert_eq!(vec_b.capacity(), 8);
    /// assert_eq!(bump.stats().allocated(), 4 * 4);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Mapping to a type with a higher alignment or size is equivalent to
    /// calling `try_from_iter_in(into_iter().map(f), ...)`:
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::try_new()?;
    /// let vec_a: BumpVec<u16, _> = BumpVec::try_from_iter_in([1, 2, 3, 4], &bump)?;
    /// assert_eq!(vec_a.capacity(), 4);
    /// assert_eq!(bump.stats().allocated(), 4 * 2);
    ///
    /// let vec_b: BumpVec<u32, _> = vec_a.try_map(|i| i as u32)?;
    /// assert_eq!(vec_b.capacity(), 4);
    /// assert_eq!(bump.stats().allocated(), 4 * 2 + 4 * 4);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_map<U>(self, f: impl FnMut(T) -> U) -> Result<BumpVec<U, A>, AllocError>
    where
        A: Clone,
    {
        self.generic_map(f)
    }

    #[inline]
    fn generic_map<B: ErrorBehavior, U>(self, mut f: impl FnMut(T) -> U) -> Result<BumpVec<U, A>, B>
    where
        A: Clone,
    {
        if !T::IS_ZST && !U::IS_ZST && T::ALIGN >= U::ALIGN && T::SIZE >= U::SIZE {
            struct DropGuard<T, U, A: BumpAllocator> {
                ptr: NonNull<T>,
                cap: usize,
                end: *mut T,
                src: *mut T,
                dst: *mut U,
                allocator: A,
            }

            impl<T, U, A: BumpAllocator> DropGuard<T, U, A> {
                fn into_allocator(self) -> A {
                    destructure!(let Self { allocator } = self);
                    allocator
                }
            }

            impl<T, U, A: BumpAllocator> Drop for DropGuard<T, U, A> {
                fn drop(&mut self) {
                    unsafe {
                        // drop `T`s
                        let drop_ptr = self.src.add(1);
                        let drop_len = pointer::offset_from_unsigned(self.end, drop_ptr);
                        ptr::slice_from_raw_parts_mut(drop_ptr, drop_len).drop_in_place();

                        // drop `U`s
                        let drop_ptr = self.ptr.as_ptr().cast::<U>();
                        let drop_len = pointer::offset_from_unsigned(self.dst, drop_ptr);
                        ptr::slice_from_raw_parts_mut(drop_ptr, drop_len).drop_in_place();

                        // deallocate memory block (for additional safety notes see `Self::drop::DropGuard::drop`)
                        let layout = Layout::from_size_align_unchecked(self.cap * T::SIZE, T::ALIGN);
                        self.allocator.deallocate(self.ptr.cast(), layout);
                    }
                }
            }

            destructure!(let Self { fixed, allocator } = self);
            let cap = fixed.capacity();
            let slice = unsafe { fixed.cook() }.into_boxed_slice().into_raw();
            let ptr = slice.cast::<T>();
            let len = slice.len();

            unsafe {
                let mut guard = DropGuard::<T, U, A> {
                    ptr,
                    cap,
                    end: ptr.as_ptr().add(len),
                    src: ptr.as_ptr(),
                    dst: ptr.as_ptr().cast(),
                    allocator,
                };

                while guard.src < guard.end {
                    let src_value = guard.src.read();
                    let dst_value = f(src_value);
                    guard.dst.write(dst_value);
                    guard.src = guard.src.add(1);
                    guard.dst = guard.dst.add(1);
                }

                let allocator = guard.into_allocator();
                let new_cap = (cap * T::SIZE) / U::SIZE;

                Ok(BumpVec {
                    fixed: RawFixedBumpVec::from_raw_parts(non_null::slice_from_raw_parts(ptr.cast(), len), new_cap),
                    allocator,
                })
            }
        } else {
            // fallback
            let allocator = self.allocator.clone();
            Ok(BumpVec::generic_from_iter_exact_in(self.into_iter().map(f), allocator)?)
        }
    }

    /// Returns a vector of the same size as `self`, with function `f` applied to each element in order.
    ///
    /// This function only compiles when `U`s size and alignment is less or equal to `T`'s or if `U` has a size of 0.
    ///
    /// # Examples
    /// Mapping to a type with an equal alignment and size:
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # use core::num::NonZero;
    /// # let bump: Bump = Bump::new();
    /// let a = BumpVec::from_iter_exact_in([0, 1, 2], &bump);
    /// let b = a.map_in_place(NonZero::new);
    /// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
    /// ```
    ///
    /// Mapping to a type with a smaller alignment and size:
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let a: BumpVec<u32, _> = BumpVec::from_iter_exact_in([0, 1, 2], &bump);
    /// assert_eq!(a.capacity(), 3);
    ///
    /// let b: BumpVec<u16, _> = a.map_in_place(|i| i as u16);
    /// assert_eq!(b.capacity(), 6);
    ///
    /// assert_eq!(b, [0, 1, 2]);
    /// ```
    ///
    /// Mapping to a type with a greater alignment or size won't compile:
    /// ```compile_fail,E0080
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let a: BumpVec<u16, _> = BumpVec::from_iter_exact_in([0, 1, 2], &bump);
    /// let b: BumpVec<u32, _> = a.map_in_place(|i| i as u32);
    /// # _ = b;
    /// ```
    ///
    /// Mapping to a type with a greater size won't compile:
    /// ```compile_fail,E0080
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let a: BumpVec<u32, _> = BumpVec::from_iter_exact_in([42], &bump);
    /// let b: BumpVec<[u32; 2], _> = a.map_in_place(|i| [i; 2]);
    /// # _ = b;
    /// ```
    pub fn map_in_place<U>(self, f: impl FnMut(T) -> U) -> BumpVec<U, A> {
        destructure!(let Self { fixed, allocator } = self);

        // `FixedBumpVec::map_in_place` handles dropping `T`s and `U`s on panic.
        // What is left to do is deallocating the memory.

        struct DropGuard<T, A: BumpAllocator> {
            ptr: NonNull<T>,
            cap: usize,
            allocator: A,
        }

        impl<T, A: BumpAllocator> DropGuard<T, A> {
            fn into_allocator(self) -> A {
                destructure!(let Self { allocator } = self);
                allocator
            }
        }

        impl<T, A: BumpAllocator> Drop for DropGuard<T, A> {
            fn drop(&mut self) {
                unsafe {
                    let layout = Layout::from_size_align_unchecked(self.cap * T::SIZE, T::ALIGN);
                    self.allocator.deallocate(self.ptr.cast(), layout);
                }
            }
        }

        let guard = DropGuard::<T, _> {
            ptr: fixed.as_non_null(),
            cap: fixed.capacity(),
            allocator,
        };

        let fixed = unsafe { RawFixedBumpVec::from_cooked(fixed.cook().map_in_place(f)) };
        let allocator = guard.into_allocator();

        BumpVec { fixed, allocator }
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut v = bump_vec![in &bump; 1, 2, 3, 4];
    /// let new = [7, 8, 9];
    /// let u = bump.alloc_iter(v.splice(1..3, new));
    /// assert_eq!(v, [1, 7, 8, 9, 4]);
    /// assert_eq!(u, [2, 3]);
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    #[inline]
    pub fn splice<R, I>(&mut self, range: R, replace_with: I) -> Splice<'_, I::IntoIter, A>
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
    ///
    /// - `len` must be less than or equal to `self.capacity()`
    ///
    /// [`reserve`]: Self::reserve
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) unsafe fn buf_reserve(&mut self, len: usize, additional: usize) {
        if additional > (self.capacity() - len) {
            panic_on_error(self.generic_grow_amortized_buf(len, additional));
        }
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

        let new_cap = self.capacity().checked_mul(2).unwrap_or(required_cap).max(required_cap);
        let new_cap = new_cap.max(min_non_zero_cap(T::SIZE));

        unsafe { self.generic_grow_to(new_cap) }
    }

    /// Like [`reserve`] but allows you to provide a different `len`.
    ///
    /// This is only used for [`buf_reserve`], read its documentation for more.
    ///
    /// # Safety
    ///
    /// - `len` must be less than or equal to `self.capacity()`
    ///
    /// [`reserve`]: Self::reserve
    /// [`buf_reserve`]: Self::buf_reserve
    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    unsafe fn generic_grow_amortized_buf<E: ErrorBehavior>(&mut self, len: usize, additional: usize) -> Result<(), E> {
        if T::IS_ZST {
            // This function is only called after we checked that the current capacity is not
            // sufficient. When `T::IS_ZST` the capacity is `usize::MAX`, so it can't grow.
            return Err(E::capacity_overflow());
        }

        let required_cap = match len.checked_add(additional) {
            Some(required_cap) => required_cap,
            None => return Err(E::capacity_overflow())?,
        };

        // This guarantees exponential growth. The doubling cannot overflow
        // because `capacity <= isize::MAX` and the type of `capacity` is usize;
        let new_cap = (self.capacity() * 2).max(required_cap).max(min_non_zero_cap(T::SIZE));

        self.generic_grow_to(new_cap)
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
        let new_cap = new_capacity;

        if self.capacity() == 0 {
            self.fixed = RawFixedBumpVec::allocate(&self.allocator, new_cap)?;
            return Ok(());
        }

        let old_ptr = self.as_non_null().cast();

        let old_size = self.capacity() * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_cap.checked_mul(T::SIZE).ok_or_else(|| E::capacity_overflow())?;

        let old_layout = Layout::from_size_align_unchecked(old_size, T::ALIGN);
        let new_layout = match Layout::from_size_align(new_size, T::ALIGN) {
            Ok(ok) => ok,
            Err(_) => return Err(E::capacity_overflow()),
        };

        let new_ptr = match self.allocator.grow(old_ptr, old_layout, new_layout) {
            Ok(ok) => ok.cast(),
            Err(_) => return Err(E::allocation(new_layout)),
        };

        self.fixed.set_ptr(new_ptr);
        self.fixed.set_cap(new_cap);

        Ok(())
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// This will also free space for future bump allocations if and only if this is the most recent allocation.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
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
        let Self { fixed, allocator } = self;

        let old_ptr = fixed.as_non_null();
        let old_len = fixed.capacity();
        let new_len = fixed.len();

        unsafe {
            if let Some(new_ptr) = allocator.shrink_slice(old_ptr, old_len, new_len) {
                fixed.set_ptr(new_ptr);
                fixed.set_cap(new_len);
            }
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
            let mut local_len = self.fixed.set_len_on_drop();

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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in &bump; 1, 2, 3, 4];
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut v = bump_vec![in &bump; 1, 2, 3];
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let some_predicate = |x: &mut i32| { *x == 2 || *x == 3 || *x == 6 };
    /// # let bump: Bump = Bump::new();
    /// # let mut vec = bump_vec![in &bump; 1, 2, 3, 4, 5, 6];
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut numbers = bump_vec![in &bump; 1, 2, 3, 4, 5, 6, 8, 9, 11, 13, 14, 15];
    ///
    /// let evens = numbers.extract_if(|x| *x % 2 == 0).collect::<Vec<_>>();
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2, 2, 3, 2];
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 10, 20, 21, 30, 20];
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; "foo", "bar", "Bar", "baz", "bar"];
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
    /// # use bump_scope::{Bump, BumpVec};
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
    /// [`set_len`]: Self::set_len
    ///
    /// Note that this is a low-level API, which should be used with care for
    /// optimization purposes. If you need to append data to a `BumpVec`
    /// you can use [`push`], [`extend`], `extend_from_slice`[`_copy`](BumpVec::extend_from_slice_copy)`/`[`_clone`](BumpVec::extend_from_within_clone),
    /// `extend_from_within`[`_copy`](BumpVec::extend_from_within_copy)`/`[`_clone`](BumpVec::extend_from_within_clone), [`insert`], [`resize`] or
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

    /// Returns a reference to the allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        &self.allocator
    }

    collection_method_allocator_stats!();
}

impl<'a, T, A: BumpAllocatorScope<'a>> BumpVec<T, A> {
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
    /// # use bump_scope::{Bump, FixedBumpVec, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut fixed_vec = FixedBumpVec::with_capacity_in(3, &bump);
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
    pub fn from_parts(vec: FixedBumpVec<'a, T>, allocator: A) -> Self {
        Self {
            fixed: unsafe { RawFixedBumpVec::from_cooked(vec) },
            allocator,
        }
    }

    /// Turns this `BumpVec<T>` into its parts.
    /// ```
    /// # use bump_scope::{Bump, BumpVec};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = BumpVec::new_in(&bump);
    /// vec.reserve(10);
    /// vec.push(1);
    /// let fixed_vec = vec.into_parts().0;
    /// assert_eq!(fixed_vec.capacity(), 10);
    /// assert_eq!(fixed_vec, [1]);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn into_parts(self) -> (FixedBumpVec<'a, T>, A) {
        destructure!(let Self { fixed, allocator } = self);
        (unsafe { fixed.cook() }, allocator)
    }
}

impl<T, const N: usize, A: BumpAllocator> BumpVec<[T; N], A> {
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
    /// # use bump_scope::{Bump, bump_vec};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut vec = bump_vec![in &bump; [1, 2, 3], [4, 5, 6], [7, 8, 9]];
    /// assert_eq!(vec.pop(), Some([7, 8, 9]));
    ///
    /// let mut flattened = vec.into_flattened();
    /// assert_eq!(flattened.pop(), Some(6));
    /// ```
    #[must_use]
    pub fn into_flattened(self) -> BumpVec<T, A> {
        destructure!(let Self { fixed, allocator } = self);
        let fixed = unsafe { RawFixedBumpVec::from_cooked(fixed.cook().into_flattened()) };
        BumpVec { fixed, allocator }
    }
}

impl<T: Debug, A: BumpAllocator> Debug for BumpVec<T, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_slice(), f)
    }
}

impl<T, A: BumpAllocator, I: SliceIndex<[T]>> Index<I> for BumpVec<T, A> {
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self.as_slice(), index)
    }
}

impl<T, A: BumpAllocator, I: SliceIndex<[T]>> IndexMut<I> for BumpVec<T, A> {
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self.as_mut_slice(), index)
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<T, A: BumpAllocator> Extend<T> for BumpVec<T, A> {
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
impl<'t, T: Clone + 't, A: BumpAllocator> Extend<&'t T> for BumpVec<T, A> {
    #[inline]
    fn extend<I: IntoIterator<Item = &'t T>>(&mut self, iter: I) {
        let iter = iter.into_iter();

        self.reserve(iter.size_hint().0);

        for value in iter {
            self.push(value.clone());
        }
    }
}

impl<T, A: BumpAllocator> IntoIterator for BumpVec<T, A> {
    type Item = T;
    type IntoIter = IntoIter<T, A>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            destructure!(let Self { fixed, allocator } = self);

            let (slice, cap) = fixed.into_raw_parts();
            let begin = slice.cast::<T>();

            let end = if T::IS_ZST {
                non_null::wrapping_byte_add(begin, slice.len())
            } else {
                non_null::add(begin, slice.len())
            };

            IntoIter {
                buf: begin,
                cap,

                ptr: begin,
                end,

                allocator,
                marker: PhantomData,
            }
        }
    }
}

impl<'c, T, A: BumpAllocator> IntoIterator for &'c BumpVec<T, A> {
    type Item = &'c T;
    type IntoIter = slice::Iter<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'c, T, A: BumpAllocator> IntoIterator for &'c mut BumpVec<T, A> {
    type Item = &'c mut T;
    type IntoIter = slice::IterMut<'c, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<T, A: BumpAllocator> AsRef<[T]> for BumpVec<T, A> {
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, A: BumpAllocator> AsMut<[T]> for BumpVec<T, A> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T, A: BumpAllocator> Borrow<[T]> for BumpVec<T, A> {
    #[inline(always)]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T, A: BumpAllocator> BorrowMut<[T]> for BumpVec<T, A> {
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T: Hash, A: BumpAllocator> Hash for BumpVec<T, A> {
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

/// Returns [`ErrorKind::OutOfMemory`](std::io::ErrorKind::OutOfMemory) when allocations fail.
#[cfg(feature = "std")]
impl<A: BumpAllocator> std::io::Write for BumpVec<u8, A> {
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

#[cfg(feature = "panic-on-alloc")]
impl<T, A: BumpAllocator + Default> FromIterator<T> for BumpVec<T, A> {
    #[inline]
    #[track_caller]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_iter_in(iter, A::default())
    }
}
