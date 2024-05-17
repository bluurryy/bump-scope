use core::{
    any::Any,
    borrow::{Borrow, BorrowMut},
    cmp::Ordering,
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut, Index, IndexMut, RangeBounds},
    ptr::{self, NonNull},
    slice::{self, SliceIndex},
};

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
use allocator_api2::boxed::Box as StdBox;

mod slice_initializer;
pub(crate) use slice_initializer::BoxSliceInitializer;

use crate::{
    polyfill::{self, nonnull},
    Drain, ExtractIf, FixedString, FixedVec, FromUtf8Error, IntoIter, NoDrop, SizedTypeProperties,
};

#[cfg(feature = "alloc")]
use crate::{BumpAllocator, WithLifetime};

/// A pointer type that uniquely owns a bump allocation of type `T`. This type is returned whenever a bump allocation is made.
///
/// You can turn a `Box` into a reference with [`into_ref`] and [`into_mut`] and into a [`Box`] with [`into_box`].
///
/// Unlike `alloc::boxed::Box`, `Box` can not implement `Clone` or free the allocated space as it does not store its allocator.
///
/// ## Certain `Box` types have additional methods
/// - `Box<MaybeUninit<T>>` and `Box<[MaybeUninit<T>]>` provide methods to initialize the value(s).
/// - `Box<[T]>` provides some methods from `Vec<T>` and `[T]` like `pop`, `remove`, `split_at`, ...
/// - `Box<str>` and `Box<[u8]>` provide methods to convert between the two.
///
/// ## No pinning
///
/// There is no way to safely pin a `Box` in the general case.
/// The [*drop guarantee*] of `Pin` requires the value to be dropped before its memory is reused.
/// Preventing reuse of memory is not an option as that's what this crate is all about.
/// So we need to drop the pinned value.
/// But there is no way to ensure that a value is dropped in an async context.
/// <details>
/// <summary>Example of an unsound pin macro implementation.</summary>
///
/// We define a `bump_box_pin` macro that turns a `Box<T>` into a `Pin<&mut T>`. This is only sound in synchronous code.
/// Here the memory `Foo(1)` is allocated at is reused by `Foo(2)` without dropping `Foo(1)` first which violates the drop guarantee.
///
/// ```
/// # use bump_scope::{ Bump, Box };
/// # use std::{ mem, task::{ Context, Poll }, pin::Pin, future::Future };
/// #
/// # #[must_use = "futures do nothing unless you `.await` or poll them"]
/// # pub struct YieldNow(bool);
/// #
/// # impl Future for YieldNow {
/// #     type Output = ();
/// #
/// #     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
/// #         if !self.0 {
/// #             self.0 = true;
/// #             cx.waker().wake_by_ref();
/// #             Poll::Pending
/// #         } else {
/// #             Poll::Ready(())
/// #         }
/// #     }
/// # }
/// #
/// # pub fn yield_now() -> YieldNow {
/// #     YieldNow(false)
/// # }
/// #
/// macro_rules! bump_box_pin {
///     ($name:ident) => {
///         let mut boxed: Box<_> = $name;
///         let $name = unsafe { Pin::new_unchecked(&mut *boxed) };
///     };
/// }
///
/// struct Foo(i32);
///
/// impl Drop for Foo {
///     fn drop(&mut self) {
///         println!("dropped Foo({}) at {:?}", self.0, self as *const Foo);
///     }
/// }
///
/// fn use_pinned(_foo: Pin<&mut Foo>) {}
///
/// fn violate_drop_guarantee(cx: &mut Context) {
///     let mut bump: Bump = Bump::new();
///
///     let mut future = Box::pin(async {
///         let foo = bump.alloc(Foo(1));
///         println!("created Foo({}) at {:?}", foo.0, &*foo as *const Foo);
///         bump_box_pin!(foo);
///         println!("pinned Foo({}) at {:?}", foo.0, &*foo as *const Foo);
///         yield_now().await;
///         use_pinned(foo);
///     });
///
///     assert_eq!(future.as_mut().poll(cx), Poll::Pending);
///     mem::forget(future);
///
///     bump.reset();
///     let foo = bump.alloc(Foo(2));
///     println!("created Foo({}) at {:?}", foo.0, &*foo as *const Foo);
/// }
/// ```
/// This will print something like:
/// ```text
/// created Foo(1) at 0x78a4f4000d30
/// pinned  Foo(1) at 0x78a4f4000d30
/// created Foo(2) at 0x78a4f4000d30
/// dropped Foo(2) at 0x78a4f4000d30
/// ```
/// </details>
///
/// [`into_ref`]: Box::into_ref
/// [`into_mut`]: Box::into_mut
/// [`into_box`]: Box::into_box
/// [`leak`]: Box::leak
/// [`Box`]: allocator_api2::boxed::Box
/// [*drop guarantee*]: https://doc.rust-lang.org/std/pin/index.html#subtle-details-and-the-drop-guarantee
#[repr(transparent)]
pub struct Box<'a, T: ?Sized> {
    pub(crate) ptr: NonNull<T>,

    /// First field marks the lifetime.
    /// Second field marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    marker: PhantomData<(&'a (), T)>,
}

unsafe impl<'a, T: ?Sized + Send> Send for Box<'a, T> {}
unsafe impl<'a, T: ?Sized + Sync> Sync for Box<'a, T> {}

impl<'a, T> Box<'a, T> {
    #[must_use]
    #[inline(always)]
    pub(crate) fn zst(value: T) -> Self {
        assert!(T::IS_ZST);
        mem::forget(value);
        Self {
            ptr: NonNull::dangling(),
            marker: PhantomData,
        }
    }
}

impl<'a, T: ?Sized + NoDrop> Box<'a, T> {
    /// Turns this `Box<T>` into `&T` that is live for the entire bump scope.
    /// This is only available for [`NoDrop`] types so you don't omit dropping a value for which it matters.
    ///
    /// `!NoDrop` types can still be turned into references via [`leak`](Box::leak).
    #[must_use]
    #[inline(always)]
    pub fn into_ref(self) -> &'a T {
        self.into_mut()
    }

    /// Turns this `Box<T>` into `&mut T` that is live for the entire bump scope.
    /// This is only available for [`NoDrop`] types so you don't omit dropping a value for which it matters.
    ///
    /// `!NoDrop` types can still be turned into references via [`leak`](Box::leak).
    #[must_use]
    #[inline(always)]
    pub fn into_mut(self) -> &'a mut T {
        Self::leak(self)
    }
}

impl<'a, T: ?Sized> Box<'a, T> {
    /// Turns this `Box<T>` into `Box<T>`. The `bump` allocator is not required to be
    /// the allocator this box was allocated in.
    ///
    /// Unlike `Box`, `Box` implements `Clone` and frees space iff it is the last allocation:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let a = bump.alloc(3i32).into_box(&bump);
    /// let b = a.clone();
    /// assert_eq!(a, b);
    /// drop(b);
    /// drop(a);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "alloc")]
    pub fn into_box<A: BumpAllocator>(self, bump: A) -> StdBox<T, WithLifetime<'a, A>> {
        let ptr = Box::into_raw(self).as_ptr();

        // SAFETY: bump might not be the allocator self was allocated with;
        // that's fine though because a `BumpAllocator` allows deallocate calls
        // from allocations that don't belong to it
        unsafe { StdBox::from_raw_in(ptr, WithLifetime::new(bump)) }
    }

    /// Turns this `Box<T>` into `&mut T` that is live for the entire bump scope.
    /// `T` won't be dropped which may leak resources.
    ///
    /// If `T` is [`NoDrop`], prefer to call [`into_mut`](Box::into_mut) to signify that nothing gets leaked.
    #[inline(always)]
    #[allow(clippy::must_use_candidate)]
    pub fn leak(boxed: Self) -> &'a mut T {
        unsafe { Box::into_raw(boxed).as_mut() }
    }
}

impl<'a, T> Box<'a, T> {
    /// Consumes the `Box`, returning the wrapped value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let c = bump.alloc(5);
    /// assert_eq!(c.into_inner(), 5);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn into_inner(self) -> T {
        unsafe { self.into_raw().as_ptr().read() }
    }

    /// Converts a `Box<T>` into a `Box<[T]>`
    ///
    /// This conversion happens in place.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_slice(self) -> Box<'a, [T]> {
        unsafe {
            let ptr = self.into_raw();
            let ptr = nonnull::slice_from_raw_parts(ptr, 1);
            Box::from_raw(ptr)
        }
    }
}

impl<'a> Box<'a, [u8]> {
    /// Converts a slice of bytes to a string slice.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the slice is not UTF-8 with a description as to why the
    /// provided bytes are not UTF-8. The vector you moved in is also included.
    #[inline]
    pub const fn into_boxed_str(self) -> Result<Box<'a, str>, FromUtf8Error<Self>> {
        match core::str::from_utf8(self.as_slice()) {
            Ok(_) => Ok(unsafe { self.into_boxed_str_unchecked() }),
            Err(error) => Err(FromUtf8Error { error, bytes: self }),
        }
    }

    /// Converts a slice of bytes to a string slice without checking
    /// that the string contains valid UTF-8.
    ///
    /// See the safe version, [`into_boxed_str`](Self::into_boxed_str), for more information.
    ///
    /// # Safety
    ///
    /// The bytes passed in must be valid UTF-8.
    #[inline]
    #[must_use]
    pub const unsafe fn into_boxed_str_unchecked(self) -> Box<'a, str> {
        let ptr = self.ptr.as_ptr();
        let _ = ManuallyDrop::new(self);

        Box {
            ptr: NonNull::new_unchecked(ptr as *mut str),
            marker: PhantomData,
        }
    }
}

impl<'a> Box<'a, str> {
    /// Empty str.
    pub const EMPTY_STR: Self = unsafe { Box::<[u8]>::EMPTY.into_boxed_str_unchecked() };

    /// Converts a `Box<str>` into a `Box<[u8]>`.
    #[inline]
    #[must_use]
    pub fn into_boxed_bytes(self) -> Box<'a, [u8]> {
        Box {
            ptr: unsafe { NonNull::new_unchecked(self.ptr.as_ptr() as *mut [u8]) },
            marker: PhantomData,
        }
    }
}

impl<'a, T: Sized> Box<'a, MaybeUninit<T>> {
    /// Initializes `self` with `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit();
    /// let init = uninit.init(1);
    /// assert_eq!(*init, 1);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn init(mut self, value: T) -> Box<'a, T> {
        self.as_mut().write(value);
        unsafe { self.assume_init() }
    }

    /// # Safety
    ///
    /// It is up to the caller to guarantee that the `MaybeUninit<T>` really is in an initialized state. Calling this when the content is not yet fully initialized causes immediate undefined behavior.
    ///
    /// See [`MaybeUninit::assume_init`].
    #[must_use]
    #[inline(always)]
    pub unsafe fn assume_init(self) -> Box<'a, T> {
        let ptr = Box::into_raw(self);
        Box::from_raw(ptr.cast())
    }
}

impl<'a, T: Sized> Box<'a, [MaybeUninit<T>]> {
    #[must_use]
    #[inline(always)]
    pub(crate) fn uninit_zst_slice(len: usize) -> Self {
        assert!(T::IS_ZST);
        Self {
            ptr: nonnull::slice_from_raw_parts(NonNull::dangling(), len),
            marker: PhantomData,
        }
    }

    /// Initializes `self` by filling it with elements by cloning `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let buf = bump.alloc_uninit_slice(10);
    /// let buf = buf.init_fill(1);
    /// assert_eq!(buf, [1; 10]);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn init_fill(self, value: T) -> Box<'a, [T]>
    where
        T: Clone,
    {
        unsafe {
            let len = self.len();

            if len != 0 {
                let mut initializer = self.initializer();

                for _ in 0..(len - 1) {
                    initializer.push_with_unchecked(|| value.clone());
                }

                initializer.push_unchecked(value);
                initializer.into_init_unchecked()
            } else {
                Box::default()
            }
        }
    }

    /// Initializes `self` by filling it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`init_fill`](Self::init_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let buf = bump.alloc_uninit_slice(10);
    /// let buf = buf.init_fill_with(Default::default);
    /// assert_eq!(buf, [0; 10]);
    /// ```
    #[must_use]
    #[inline]
    pub fn init_fill_with(self, mut f: impl FnMut() -> T) -> Box<'a, [T]> {
        let mut initializer = self.initializer();

        while !initializer.is_full() {
            initializer.push_with(&mut f);
        }

        initializer.into_init()
    }

    /// Initializes `self` by copying the elements from `slice` into `self`.
    ///
    /// The length of `slice` must be the same as `self`.
    ///
    /// # Panics
    ///
    /// This function will panic if the two slices have different lengths.
    #[must_use]
    #[inline]
    pub fn init_copy(mut self, slice: &[T]) -> Box<'a, [T]>
    where
        T: Copy,
    {
        self.copy_from_slice(as_uninit_slice(slice));
        unsafe { self.assume_init() }
    }

    /// Initializes `self` by cloning the elements from `slice` into `self`.
    ///
    /// The length of `slice` must be the same as `self`.
    ///
    /// # Panics
    ///
    /// This function will panic if the two slices have different lengths.
    #[must_use]
    #[inline]
    pub fn init_clone(self, slice: &[T]) -> Box<'a, [T]>
    where
        T: Clone,
    {
        assert_eq!(slice.len(), self.len());

        let mut initializer = self.initializer();

        // SAFETY: we asserted that the lengths are the same
        unsafe {
            for value in slice {
                initializer.push_unchecked(value.clone());
            }

            initializer.into_init_unchecked()
        }
    }

    #[must_use]
    #[inline]
    pub(crate) fn initializer(self) -> BoxSliceInitializer<'a, T> {
        BoxSliceInitializer::new(self)
    }

    /// # Safety
    ///
    /// It is up to the caller to guarantee that each `MaybeUninit<T>` really is in an initialized state. Calling this when the content is not yet fully initialized causes immediate undefined behavior.
    ///
    /// See [`MaybeUninit::assume_init`].
    #[must_use]
    #[inline(always)]
    pub unsafe fn assume_init(self) -> Box<'a, [T]> {
        let ptr = Box::into_raw(self);
        let ptr = NonNull::new_unchecked(ptr.as_ptr() as _);
        Box::from_raw(ptr)
    }

    #[deprecated = "use `FixedVec::from_uninit` instead"]
    /// Turns this `Box<[MaybeUninit<T>]>` into a `FixedVec<T>` with a length of `0`.
    #[inline]
    #[must_use]
    pub fn into_fixed_vec(self) -> FixedVec<'a, T> {
        FixedVec::from_uninit(self)
    }
}

impl<'a> Box<'a, [MaybeUninit<u8>]> {
    #[deprecated = "use `FixedString::from_uninit` instead"]
    /// Turns this `Box<[MaybeUninit<u8>]>` into a `FixedString` with a length of `0`.
    #[inline]
    #[must_use]
    pub fn into_fixed_string(self) -> FixedString<'a> {
        FixedString::from_uninit(self)
    }
}

impl<'a, T> Box<'a, [T]> {
    /// Empty slice.
    pub const EMPTY: Self = Self {
        ptr: nonnull::slice_from_raw_parts(NonNull::dangling(), 0),
        marker: PhantomData,
    };

    #[must_use]
    #[inline(always)]
    pub(crate) fn zst_slice_clone(slice: &[T]) -> Self
    where
        T: Clone,
    {
        assert!(T::IS_ZST);
        Box::uninit_zst_slice(slice.len()).init_clone(slice)
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn zst_slice_fill(len: usize, value: T) -> Self
    where
        T: Clone,
    {
        assert!(T::IS_ZST);
        if len == 0 {
            drop(value);
            Box::EMPTY
        } else {
            for _ in 1..len {
                mem::forget(value.clone());
            }

            mem::forget(value);
            unsafe { Box::zst_slice_from_len(len) }
        }
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn zst_slice_fill_with(len: usize, mut f: impl FnMut() -> T) -> Self {
        assert!(T::IS_ZST);
        for _ in 0..len {
            mem::forget(f());
        }
        unsafe { Box::zst_slice_from_len(len) }
    }

    /// Creates `T` values from nothing!
    #[must_use]
    #[inline(always)]
    unsafe fn zst_slice_from_len(len: usize) -> Self {
        assert!(T::IS_ZST);
        Self {
            ptr: nonnull::slice_from_raw_parts(NonNull::dangling(), len),
            marker: PhantomData,
        }
    }

    /// Returns the number of elements in the slice, also referred to
    /// as its 'length'.
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.ptr.len()
    }

    /// Returns `true` if the slice contains no elements.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Removes the last element from a slice and returns it, or [`None`] if it
    /// is empty.
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            unsafe {
                self.set_len(self.len() - 1);
                let ptr = self.as_ptr().add(self.len());
                Some(ptr.read())
            }
        }
    }

    /// Clears the slice, removing all values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_vec };
    /// # let bump: Bump = Bump::new();
    /// let mut slice = bump.alloc_slice_copy(&[1, 2, 3]);
    /// slice.clear();
    /// assert!(slice.is_empty());
    /// ```
    #[inline(always)]
    pub fn clear(&mut self) {
        let elems: *mut [T] = self.ptr.as_ptr();

        // SAFETY:
        // - Setting `self.len` before calling `drop_in_place` means that,
        //   if an element's `Drop` impl panics, the vector's `Drop` impl will
        //   do nothing (leaking the rest of the elements) instead of dropping
        //   some twice.
        unsafe {
            self.set_len(0);
            ptr::drop_in_place(elems);
        }
    }

    /// Shortens the slice, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// If `len` is greater than the slice's current length, this has no
    /// effect.
    /// <!--
    /// The [`drain`] method can emulate `truncate`, but causes the excess
    /// elements to be returned instead of dropped.
    /// -->
    /// Note that this method has no effect on the allocated capacity
    /// of the vector.
    ///
    /// # Examples
    ///
    /// Truncating a five element vector to two elements:
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5]);
    /// slice.truncate(2);
    /// assert_eq!(slice, [1, 2]);
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the slice's current
    /// length:
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut slice = bump.alloc_slice_copy(&[1, 2, 3]);
    /// slice.truncate(8);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_vec };
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut slice = bump.alloc_slice_copy(&[1, 2, 3]);
    /// slice.truncate(0);
    /// assert_eq!(slice, []);
    /// ```
    ///
    /// [`clear`]: Box::clear
    /// [`drain`]: Box::drain
    pub fn truncate(&mut self, len: usize) {
        unsafe { nonnull::truncate(&mut self.ptr, len) }
    }

    /// Extracts a slice containing the entire boxed slice.
    ///
    /// Equivalent to `&s[..]`.
    #[must_use]
    #[inline(always)]
    pub const fn as_slice(&self) -> &[T] {
        unsafe { &*(self.ptr.as_ptr() as *const _) }
    }

    /// Extracts a mutable slice containing the entire boxed slice.
    ///
    /// Equivalent to `&mut s[..]`.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { self.ptr.as_mut() }
    }

    /// Returns a raw pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        // We shadow the slice method of the same name to avoid going through
        // `deref`, which creates an intermediate reference.
        self.ptr.as_ptr().cast()
    }

    /// Returns an unsafe mutable pointer to slice, or a dangling
    /// raw pointer valid for zero sized reads.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        // We shadow the slice method of the same name to avoid going through
        // `deref_mut`, which creates an intermediate reference.
        self.ptr.as_ptr().cast()
    }

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_ptr(&self) -> NonNull<T> {
        self.ptr.cast()
    }

    /// Returns a raw nonnull pointer to the slice, or a dangling raw pointer
    /// valid for zero sized reads.
    #[must_use]
    #[inline(always)]
    pub fn as_non_null_slice(&self) -> NonNull<[T]> {
        self.ptr
    }

    /// Forces the length of the slice to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type. Normally changing the length of a boxed slice
    /// is done using one of the safe operations instead, such as
    /// [`truncate`] or [`clear`].
    ///
    /// [`truncate`]: Box::truncate
    /// [`clear`]: Box::clear
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to the [`capacity`] (capacity is not tracked by this type).
    /// - The elements at `old_len..new_len` must be initialized.
    ///
    /// [`capacity`]: crate::MutVec::capacity
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        nonnull::set_len(&mut self.ptr, new_len);
    }

    #[inline]
    pub(crate) unsafe fn inc_len(&mut self, amount: usize) {
        self.set_len(self.len() + amount);
    }

    #[inline]
    pub(crate) unsafe fn dec_len(&mut self, amount: usize) {
        self.set_len(self.len() - amount);
    }

    #[inline]
    pub(crate) unsafe fn set_ptr(&mut self, ptr: NonNull<T>) {
        let len = self.ptr.len();
        self.ptr = nonnull::slice_from_raw_parts(ptr, len);
    }

    /// Removes and returns the element at position `index` within the slice,
    /// shifting all elements after it to the left.
    ///
    /// Note: Because this shifts over the remaining elements, it has a
    /// worst-case performance of *O*(*n*). If you don't need the order of elements
    /// to be preserved, use [`swap_remove`] instead.
    ///
    /// [`swap_remove`]: Box::swap_remove
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_vec };
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

    /// Removes an element from the slice and returns it.
    ///
    /// The removed element is replaced by the last element of the slice.
    ///
    /// This does not preserve ordering, but is *O*(1).
    /// If you need to preserve the element order, use [`remove`] instead.
    ///
    /// [`remove`]: Box::remove
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, mut_vec };
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

    /// Divides one slice into two at an index.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding
    /// the index `mid` itself) and the second will contain all
    /// indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// # Panics
    ///
    /// Panics if `mid > len`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    ///
    /// {
    ///    let (left, right) = v.split_at(0);
    ///    assert_eq!(left, []);
    ///    assert_eq!(right, [1, 2, 3, 4, 5, 6]);
    /// }
    ///
    /// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    ///
    /// {
    ///     let (left, right) = v.split_at(2);
    ///     assert_eq!(left, [1, 2]);
    ///     assert_eq!(right, [3, 4, 5, 6]);
    /// }
    ///
    /// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    ///
    /// {
    ///     let (left, right) = v.split_at(6);
    ///     assert_eq!(left, [1, 2, 3, 4, 5, 6]);
    ///     assert_eq!(right, []);
    /// }
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn split_at(self, mid: usize) -> (Self, Self) {
        assert!(mid <= self.len());
        // SAFETY: `[ptr; mid]` and `[mid; len]` are inside `self`, which
        // fulfills the requirements of `split_at_unchecked`.
        unsafe { self.split_at_unchecked(mid) }
    }

    /// Divides one slice into two at an index, without doing bounds checking.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding
    /// the index `mid` itself) and the second will contain all
    /// indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// For a safe alternative see [`split_at`].
    ///
    /// # Safety
    ///
    /// Calling this method with an out-of-bounds index is *[undefined behavior]*
    /// even if the resulting reference is not used. The caller has to ensure that
    /// `0 <= mid <= self.len()`.
    ///
    /// [`split_at`]: slice::split_at
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    ///
    /// unsafe {
    ///    let (left, right) = v.split_at_unchecked(0);
    ///    assert_eq!(left, []);
    ///    assert_eq!(right, [1, 2, 3, 4, 5, 6]);
    /// }
    ///
    /// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    ///
    /// unsafe {
    ///     let (left, right) = v.split_at_unchecked(2);
    ///     assert_eq!(left, [1, 2]);
    ///     assert_eq!(right, [3, 4, 5, 6]);
    /// }
    ///
    /// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    ///
    /// unsafe {
    ///     let (left, right) = v.split_at_unchecked(6);
    ///     assert_eq!(left, [1, 2, 3, 4, 5, 6]);
    ///     assert_eq!(right, []);
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub unsafe fn split_at_unchecked(self, mid: usize) -> (Self, Self) {
        let this = ManuallyDrop::new(self);

        let len = this.len();
        let ptr = this.ptr.cast::<T>();

        debug_assert!(
            mid <= len,
            "slice::split_at_unchecked requires the index to be within the slice"
        );

        (
            Self::from_raw(nonnull::slice_from_raw_parts(ptr, mid)),
            Self::from_raw(nonnull::slice_from_raw_parts(nonnull::add(ptr, mid), len - mid)),
        )
    }

    /// Returns the first and all the rest of the elements of the slice, or `None` if it is empty.
    ///
    /// This does consume the `Box`. You can create a new empty one with [`Box::default`](Box::default).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let x = bump.alloc_slice_copy(&[0, 1, 2]);
    ///
    /// if let Some((first, elements)) = x.split_first() {
    ///     assert_eq!(&*first, &0);
    ///     assert_eq!(&*elements, &[1, 2]);
    /// }
    /// # ; // load bearing semicolon
    /// ```
    #[inline]
    #[must_use]
    pub fn split_first(self) -> Option<(Box<'a, T>, Box<'a, [T]>)> {
        let this = ManuallyDrop::new(self);

        if this.is_empty() {
            return None;
        }

        unsafe {
            let ptr = this.ptr.cast::<T>();
            let len = this.len();

            Some((
                Box::from_raw(ptr),
                Box::from_raw(nonnull::slice_from_raw_parts(nonnull::add(ptr, 1), len - 1)),
            ))
        }
    }

    /// Returns the last and all the rest of the elements of the slice, or `None` if it is empty.
    ///
    /// This does consume the `Box`. You can create a new empty one with [`Box::default`](Box::default).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump };
    /// # let bump: Bump = Bump::new();
    /// #
    /// let x = bump.alloc_slice_copy(&[0, 1, 2]);
    ///
    /// if let Some((last, elements)) = x.split_last() {
    ///     assert_eq!(&*last, &2);
    ///     assert_eq!(&*elements, &[0, 1]);
    /// }
    /// # ; // load bearing semicolon
    /// ```
    #[inline]
    #[must_use]
    pub fn split_last(self) -> Option<(Box<'a, T>, Box<'a, [T]>)> {
        let this = ManuallyDrop::new(self);

        if this.is_empty() {
            return None;
        }

        unsafe {
            let ptr = this.ptr.cast::<T>();
            let len_minus_one = this.len() - 1;

            Some((
                Box::from_raw(nonnull::add(ptr, len_minus_one)),
                Box::from_raw(nonnull::slice_from_raw_parts(ptr, len_minus_one)),
            ))
        }
    }

    /// Merges two contiguous slices into one.
    ///
    /// # Panics
    ///
    /// Panics if `self` and `other` are not contiguous.
    /// Panics if `T` is a zero-sized type and adding the lengths overflows.
    ///
    /// # Examples
    ///
    /// Split and merge back together.
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    ///
    /// let (left, right) = v.split_at(3);
    /// assert_eq!(left, [1, 2, 3]);
    /// assert_eq!(right, [4, 5, 6]);
    ///
    /// let merged = left.merge(right);
    /// assert_eq!(merged, [1, 2, 3, 4, 5, 6]);
    /// ```
    #[inline]
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed_zst() -> ! {
            panic!("adding the lengths overflowed");
        }

        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed() -> ! {
            panic!("the two slices are not contiguous");
        }

        if T::IS_ZST {
            let len = match self.len().checked_add(other.len()) {
                Some(len) => len,
                None => assert_failed_zst(),
            };

            let _ = self.into_raw();
            let _ = other.into_raw();

            unsafe { Self::zst_slice_from_len(len) }
        } else {
            if self.as_ptr_range().end != other.as_ptr() {
                assert_failed();
            }

            let lhs = self.into_raw();
            let rhs = other.into_raw();

            let ptr = nonnull::as_non_null_ptr(lhs);

            // This can't overflow.
            // - Two slices can only be contiguous if they are part of the same chunk.
            // - The size of a chunk is representable as `usize`.
            let len = lhs.len() + rhs.len();

            let slice = nonnull::slice_from_raw_parts(ptr, len);

            unsafe { Self::from_raw(slice) }
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut slice = bump.alloc_slice_copy(&[1, 2, 3, 4]);
    ///
    /// slice.retain(|x| if *x <= 3 {
    ///     *x += 1;
    ///     true
    /// } else {
    ///     false
    /// });
    ///
    /// assert_eq!(slice, [2, 3, 4]);
    /// ```
    #[allow(clippy::pedantic)]
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let original_len = self.len();
        // Avoid double drop if the drop guard is not executed,
        // since we may make some holes during the process.
        unsafe { self.set_len(0) };

        // Vec: [Kept, Kept, Hole, Hole, Hole, Hole, Unchecked, Unchecked]
        //      |<-              processed len   ->| ^- next to check
        //                  |<-  deleted cnt     ->|
        //      |<-              original_len                          ->|
        // Kept: Elements which predicate returns true on.
        // Hole: Moved or dropped element slot.
        // Unchecked: Unchecked valid elements.
        //
        // This drop guard will be invoked when predicate or `drop` of element panicked.
        // It shifts unchecked elements to cover holes and `set_len` to the correct length.
        // In cases when predicate and `drop` never panic, it will be optimized out.
        struct BackshiftOnDrop<'b, 'a, T> {
            v: &'b mut Box<'a, [T]>,
            processed_len: usize,
            deleted_cnt: usize,
            original_len: usize,
        }

        impl<T> Drop for BackshiftOnDrop<'_, '_, T> {
            fn drop(&mut self) {
                if self.deleted_cnt > 0 {
                    // SAFETY: Trailing unchecked items must be valid since we never touch them.
                    unsafe {
                        ptr::copy(
                            self.v.as_ptr().add(self.processed_len),
                            self.v.as_mut_ptr().add(self.processed_len - self.deleted_cnt),
                            self.original_len - self.processed_len,
                        );
                    }
                }
                // SAFETY: After filling holes, all items are in contiguous memory.
                unsafe {
                    self.v.set_len(self.original_len - self.deleted_cnt);
                }
            }
        }

        let mut g = BackshiftOnDrop {
            v: self,
            processed_len: 0,
            deleted_cnt: 0,
            original_len,
        };

        fn process_loop<F, T, const DELETED: bool>(original_len: usize, f: &mut F, g: &mut BackshiftOnDrop<'_, '_, T>)
        where
            F: FnMut(&mut T) -> bool,
        {
            while g.processed_len != original_len {
                // SAFETY: Unchecked element must be valid.
                let cur = unsafe { &mut *g.v.ptr.as_ptr().cast::<T>().add(g.processed_len) };
                if !f(cur) {
                    // Advance early to avoid double drop if `drop_in_place` panicked.
                    g.processed_len += 1;
                    g.deleted_cnt += 1;
                    // SAFETY: We never touch this element again after dropped.
                    unsafe { ptr::drop_in_place(cur) };
                    // We already advanced the counter.
                    if DELETED {
                        continue;
                    } else {
                        break;
                    }
                }
                if DELETED {
                    // SAFETY: `deleted_cnt` > 0, so the hole slot must not overlap with current element.
                    // We use copy for move, and never touch this element again.
                    unsafe {
                        let hole_slot = g.v.ptr.as_ptr().cast::<T>().add(g.processed_len - g.deleted_cnt);
                        ptr::copy_nonoverlapping(cur, hole_slot, 1);
                    }
                }
                g.processed_len += 1;
            }
        }

        // Stage 1: Nothing was deleted.
        process_loop::<F, T, false>(original_len, &mut f, &mut g);

        // Stage 2: Some elements were deleted.
        process_loop::<F, T, true>(original_len, &mut f, &mut g);

        // All item are processed. This can be optimized to `set_len` by LLVM.
        drop(g);
    }

    /// Removes the specified range from the slice in bulk, returning all
    /// removed elements as an iterator. If the iterator is dropped before
    /// being fully consumed, it drops the remaining removed elements.
    ///
    /// The returned iterator keeps a mutable borrow on the slice to optimize
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
    /// [`mem::forget`], for example), the vector may have lost and leaked
    /// elements arbitrarily, including elements outside the range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut v = bump.alloc_slice_copy(&[1, 2, 3]);
    /// let u = bump.alloc_iter(v.drain(1..));
    /// assert_eq!(v, [1]);
    /// assert_eq!(u, [2, 3]);
    ///
    /// // A full range clears the slice, like `clear()` does
    /// v.drain(..);
    /// assert_eq!(v, []);
    /// ```
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
    where
        R: RangeBounds<usize>,
    {
        Drain::new(self, range)
    }

    /// Creates an iterator which uses a closure to determine if an element should be removed.
    ///
    /// If the closure returns true, then the element is removed and yielded.
    /// If the closure returns false, the element will remain in the slice and will not be yielded
    /// by the iterator.
    ///
    /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped without iterating
    /// or the iteration short-circuits, then the remaining elements will be retained.
    /// Use [`retain`] with a negated predicate if you do not need the returned iterator.
    ///
    /// [`retain`]: Vec::retain
    ///
    /// Using this method is equivalent to the following code:
    ///
    /// ```
    /// # let some_predicate = |x: &mut i32| { *x == 2 || *x == 3 || *x == 6 };
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// # let mut slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
    /// let mut i = 0;
    /// while i < slice.len() {
    ///     if some_predicate(&mut slice[i]) {
    ///         let val = slice.remove(i);
    ///         // your code here
    ///     } else {
    ///         i += 1;
    ///     }
    /// }
    ///
    /// # assert_eq!(slice, [1, 4, 5]);
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
    /// let mut numbers = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6, 8, 9, 11, 13, 14, 15]);
    ///
    /// let evens = bump.alloc_iter(numbers.extract_if(|x| *x % 2 == 0));
    /// let odds = numbers;
    ///
    /// assert_eq!(evens, [2, 4, 6, 8, 14]);
    /// assert_eq!(odds, [1, 3, 5, 9, 11, 13, 15]);
    /// ```
    pub fn extract_if<F>(&mut self, filter: F) -> ExtractIf<T, F>
    where
        F: FnMut(&mut T) -> bool,
    {
        ExtractIf::new(self, filter)
    }

    /// Removes consecutive repeated elements in the slice according to the
    /// [`PartialEq`] trait implementation.
    ///
    /// If the slice is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut slice = bump.alloc_slice_copy(&[1, 2, 2, 3, 2]);
    ///
    /// slice.dedup();
    ///
    /// assert_eq!(slice, [1, 2, 3, 2]);
    /// ```
    #[inline]
    pub fn dedup(&mut self)
    where
        T: PartialEq,
    {
        self.dedup_by(|a, b| a == b);
    }

    /// Removes all but the first of consecutive elements in the slice that resolve to the same
    /// key.
    ///
    /// If the slice is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut slice = bump.alloc_slice_copy(&[10, 20, 21, 30, 20]);
    ///
    /// slice.dedup_by_key(|i| *i / 10);
    ///
    /// assert_eq!(slice, [10, 20, 30, 20]);
    /// ```
    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, mut key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.dedup_by(|a, b| key(a) == key(b));
    }

    /// Removes all but the first of consecutive elements in the vector satisfying a given equality
    /// relation.
    ///
    /// The `same_bucket` function is passed references to two elements from the vector and
    /// must determine if the elements compare equal. The elements are passed in opposite order
    /// from their order in the slice, so if `same_bucket(a, b)` returns `true`, `a` is removed.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut slice = bump.alloc_slice_copy(&["foo", "bar", "Bar", "baz", "bar"]);
    ///
    /// slice.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    ///
    /// assert_eq!(slice, ["foo", "bar", "baz", "bar"]);
    /// ```
    pub fn dedup_by<F>(&mut self, mut same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        let len = self.len();
        if len <= 1 {
            return;
        }

        /* INVARIANT: vec.len() > read >= write > write-1 >= 0 */
        struct FillGapOnDrop<'b, 'a, T> {
            /* Offset of the element we want to check if it is duplicate */
            read: usize,

            /* Offset of the place where we want to place the non-duplicate
             * when we find it. */
            write: usize,

            /* The Vec that would need correction if `same_bucket` panicked */
            boxed: &'b mut Box<'a, [T]>,
        }

        impl<'b, 'a, T> Drop for FillGapOnDrop<'b, 'a, T> {
            fn drop(&mut self) {
                /* This code gets executed when `same_bucket` panics */

                /* SAFETY: invariant guarantees that `read - write`
                 * and `len - read` never overflow and that the copy is always
                 * in-bounds. */
                unsafe {
                    let ptr = self.boxed.as_mut_ptr();
                    let len = self.boxed.len();

                    /* How many items were left when `same_bucket` panicked.
                     * Basically vec[read..].len() */
                    let items_left = len.wrapping_sub(self.read);

                    /* Pointer to first item in vec[write..write+items_left] slice */
                    let dropped_ptr = ptr.add(self.write);
                    /* Pointer to first item in vec[read..] slice */
                    let valid_ptr = ptr.add(self.read);

                    /* Copy `vec[read..]` to `vec[write..write+items_left]`.
                     * The slices can overlap, so `copy_nonoverlapping` cannot be used */
                    ptr::copy(valid_ptr, dropped_ptr, items_left);

                    /* How many items have been already dropped
                     * Basically vec[read..write].len() */
                    let dropped = self.read.wrapping_sub(self.write);

                    self.boxed.set_len(len - dropped);
                }
            }
        }

        let mut gap = FillGapOnDrop {
            read: 1,
            write: 1,
            boxed: self,
        };
        let ptr = gap.boxed.as_mut_ptr();

        /* Drop items while going through Vec, it should be more efficient than
         * doing slice partition_dedup + truncate */

        /* SAFETY: Because of the invariant, read_ptr, prev_ptr and write_ptr
         * are always in-bounds and read_ptr never aliases prev_ptr */
        unsafe {
            while gap.read < len {
                let read_ptr = ptr.add(gap.read);
                let prev_ptr = ptr.add(gap.write.wrapping_sub(1));

                if same_bucket(&mut *read_ptr, &mut *prev_ptr) {
                    // Increase `gap.read` now since the drop may panic.
                    gap.read += 1;
                    /* We have found duplicate, drop it in-place */
                    ptr::drop_in_place(read_ptr);
                } else {
                    let write_ptr = ptr.add(gap.write);

                    /* Because `read_ptr` can be equal to `write_ptr`, we either
                     * have to use `copy` or conditional `copy_nonoverlapping`.
                     * Looks like the first option is faster. */
                    ptr::copy(read_ptr, write_ptr, 1);

                    /* We have filled that place, so go further */
                    gap.write += 1;
                    gap.read += 1;
                }
            }

            /* Technically we could let `gap` clean up with its Drop, but
             * when `same_bucket` is [guaranteed allocated]o not panic, this bloats a little
             * the codegen, so we just do it manually */
            gap.boxed.set_len(gap.write);
            mem::forget(gap);
        }
    }

    /// Consumes an iterator, creating two collections from it.
    ///
    /// The predicate passed to `partition()` can return `true`, or `false`.
    /// `partition()` returns a pair, all of the elements for which it returned
    /// `true`, and all of the elements for which it returned `false`.
    ///
    /// See also [`is_partitioned()`] and [`partition_in_place()`].
    ///
    /// [`is_partitioned()`]: Iterator::is_partitioned
    /// [`partition_in_place()`]: Iterator::partition_in_place
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_slice_copy(&[1, 2, 3]);
    ///
    /// let (even, odd) = slice.partition(|n| n % 2 == 0);
    ///
    /// assert_eq!(even, vec![2]);
    /// assert_eq!(odd, vec![1, 3]);
    /// ```
    #[cfg(test)]
    pub fn partition<F>(mut self, f: F) -> (Box<'a, [T]>, Box<'a, [T]>)
    where
        F: FnMut(&T) -> bool,
    {
        let index = self.iter_mut().partition_in_place(f);
        self.split_at(index)
    }
}

impl<'a, T, const N: usize> Box<'a, [[T; N]]> {
    /// Takes a `Box<[[T; N]]>` and flattens it into a `Box<[T]>`.
    ///
    /// # Panics
    ///
    /// Panics if the length of the resulting slice would overflow a `usize`.
    ///
    /// This is only possible when flattening a slice of arrays of zero-sized
    /// types, and thus tends to be irrelevant in practice. If
    /// `size_of::<T>() > 0`, this will never panic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let mut slice = bump.alloc_slice_copy(&[[1, 2, 3], [4, 5, 6], [7, 8, 9]]);
    /// assert_eq!(slice.pop(), Some([7, 8, 9]));
    ///
    /// let mut flattened = slice.into_flattened();
    /// assert_eq!(flattened.pop(), Some(6));
    /// ```
    #[must_use]
    pub fn into_flattened(self) -> Box<'a, [T]> {
        let ptr = self.as_non_null_ptr();
        let len = self.len();

        let new_len = if T::IS_ZST {
            len.checked_mul(N).expect("vec len overflow")
        } else {
            // SAFETY:
            // - `len * N` cannot overflow because the allocation is already in
            // the address space.
            // - Each `[T; N]` has `N` valid elements, so there are `len * N`
            // valid elements in the allocation.
            unsafe { polyfill::usize::unchecked_mul(len, N) }
        };

        unsafe { Box::from_raw(nonnull::slice_from_raw_parts(ptr.cast(), new_len)) }
    }
}

impl<'a, T: ?Sized> Box<'a, T> {
    /// Consumes the `Box`, returning a wrapped raw pointer.
    ///
    /// The pointer will be properly aligned and non-null. It is only valid for the lifetime `'a`.
    ///
    /// After calling this function, the caller is responsible for dropping the
    /// value previously managed by the `Box`. The easiest way to do this is to `p.drop_in_place()`.
    ///
    /// You can turn this pointer back into a `Box` with [`Box::from_raw`].
    ///
    /// # Examples
    /// Manually dropping `T`:
    /// ```
    /// use bump_scope::{ Bump, Box };
    /// let bump: Bump = Bump::new();
    /// let x = bump.alloc(String::from("Hello"));
    /// let p = Box::into_raw(x);
    /// unsafe { p.as_ptr().drop_in_place() }
    /// ```
    #[inline(always)]
    #[must_use = "use `leak` if you don't make use of the pointer"]
    #[allow(clippy::needless_pass_by_value)]
    pub fn into_raw(self) -> NonNull<T> {
        ManuallyDrop::new(self).ptr
    }

    /// Constructs a `Box` from a raw pointer.
    ///
    /// After calling this function, the pointed to value is owned by the resulting `Box`.
    /// Specifically, the `Box` destructor will call the destructor of `T`.
    /// For this to be safe, the pointer must point to a valid `T` for the lifetime of `'a`.
    ///
    /// # Safety
    /// - `ptr` must point to a valid value for the lifetime `'a`
    ///
    /// # Examples
    ///
    /// Recreate a `Box` which was previously converted to a raw pointer
    /// using [`Box::into_raw`]:
    /// ```
    /// use bump_scope::{ Bump, Box };
    /// use core::ptr::NonNull;
    ///
    /// unsafe fn from_raw_in<'a, T>(ptr: NonNull<T>, bump: &'a Bump) -> Box<'a, T> {
    ///     Box::from_raw(ptr)
    /// }
    ///
    /// let bump: Bump = Bump::new();
    /// let x = bump.alloc(String::from("Hello"));
    /// let ptr = Box::into_raw(x);
    /// let x = unsafe { from_raw_in(ptr, &bump) };
    /// assert_eq!(x.as_str(), "Hello");
    /// drop(x);
    /// ```
    /// Manually create a `Box` from scratch by using the bump allocator:
    /// ```
    /// use bump_scope::{ Bump, Box };
    /// use core::alloc::Layout;
    /// use core::ptr::NonNull;
    ///
    /// unsafe fn from_raw_in<'a, T>(ptr: NonNull<T>, bump: &'a Bump) -> Box<'a, T> {
    ///     Box::from_raw(ptr)
    /// }
    ///
    /// let bump: Bump = Bump::new();
    ///
    /// let five = unsafe {
    ///     let ptr = bump.alloc_layout(Layout::new::<i32>());
    ///     // In general .write is required to avoid attempting to destruct
    ///     // the (uninitialized) previous contents of `ptr`, though for this
    ///     // simple example `*ptr = 5` would have worked as well.
    ///     ptr.as_ptr().write(5);
    ///     from_raw_in(ptr, &bump)
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// ```
    #[must_use]
    #[inline(always)]
    pub unsafe fn from_raw(ptr: NonNull<T>) -> Self {
        Self {
            ptr,
            marker: PhantomData,
        }
    }
}

impl<'a> Box<'a, dyn Any> {
    /// Attempt to downcast the box to a concrete type.
    #[inline(always)]
    #[allow(clippy::missing_errors_doc)]
    pub fn downcast<T: Any>(self) -> Result<Box<'a, T>, Self> {
        if self.is::<T>() {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }

    /// Downcasts the box to a concrete type.
    ///
    /// For a safe alternative see [`downcast`].
    ///
    /// # Safety
    ///
    /// The contained value must be of type `T`. Calling this method
    /// with the incorrect type is *undefined behavior*.
    ///
    /// [`downcast`]: Self::downcast
    #[must_use]
    #[inline(always)]
    pub unsafe fn downcast_unchecked<T: Any>(self) -> Box<'a, T> {
        Box::from_raw(Box::into_raw(self).cast())
    }
}

impl<'a> Box<'a, dyn Any + Send> {
    /// Attempt to downcast the box to a concrete type.
    #[allow(clippy::missing_errors_doc)]
    #[inline(always)]
    pub fn downcast<T: Any>(self) -> Result<Box<'a, T>, Self> {
        if self.is::<T>() {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }

    /// Downcasts the box to a concrete type.
    ///
    /// For a safe alternative see [`downcast`].
    ///
    /// # Safety
    ///
    /// The contained value must be of type `T`. Calling this method
    /// with the incorrect type is *undefined behavior*.
    ///
    /// [`downcast`]: Self::downcast
    #[must_use]
    #[inline(always)]
    pub unsafe fn downcast_unchecked<T: Any>(self) -> Box<'a, T> {
        Box::from_raw(Box::into_raw(self).cast())
    }
}

impl<'a> Box<'a, dyn Any + Send + Sync> {
    /// Attempt to downcast the box to a concrete type.
    #[allow(clippy::missing_errors_doc)]
    #[inline(always)]
    pub fn downcast<T: Any>(self) -> Result<Box<'a, T>, Self> {
        if self.is::<T>() {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }

    /// Downcasts the box to a concrete type.
    ///
    /// For a safe alternative see [`downcast`].
    ///
    /// # Safety
    ///
    /// The contained value must be of type `T`. Calling this method
    /// with the incorrect type is *undefined behavior*.
    ///
    /// [`downcast`]: Self::downcast
    #[must_use]
    #[inline(always)]
    pub unsafe fn downcast_unchecked<T: Any>(self) -> Box<'a, T> {
        Box::from_raw(Box::into_raw(self).cast())
    }
}

// just like std's Rc and Arc this implements Unpin,
// at time of writing Box does not implement Unpin unconditionally, but that is probably an oversight
// see https://github.com/rust-lang/rust/pull/118634
impl<'a, T: ?Sized> Unpin for Box<'a, T> {}

#[cfg(feature = "nightly-coerce-unsized")]
impl<'a, T, U> core::ops::CoerceUnsized<Box<'a, U>> for Box<'a, T>
where
    T: ?Sized + core::marker::Unsize<U>,
    U: ?Sized,
{
}

impl<'a, T: ?Sized> Drop for Box<'a, T> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { self.ptr.as_ptr().drop_in_place() }
    }
}

impl<'a, T: ?Sized> Deref for Box<'a, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<'a, T: ?Sized> DerefMut for Box<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T: ?Sized> AsRef<T> for Box<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> AsMut<T> for Box<'_, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized> Borrow<T> for Box<'_, T> {
    #[inline(always)]
    fn borrow(&self) -> &T {
        self
    }
}

impl<T: ?Sized> BorrowMut<T> for Box<'_, T> {
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}

impl<'a, T: ?Sized + Debug> Debug for Box<'a, T> {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        T::fmt(self, f)
    }
}

impl<'a, T: ?Sized + Display> Display for Box<'a, T> {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        T::fmt(self, f)
    }
}

impl<'a, T> Default for Box<'a, [T]> {
    #[inline(always)]
    fn default() -> Self {
        unsafe { Self::from_raw(NonNull::from(&mut [])) }
    }
}

impl<'a> Default for Box<'a, str> {
    #[inline(always)]
    fn default() -> Self {
        unsafe { Self::from_raw(NonNull::from(core::str::from_utf8_unchecked_mut(&mut []))) }
    }
}

impl<'b, 'a, T: ?Sized + PartialEq> PartialEq<Box<'b, T>> for Box<'a, T> {
    #[inline(always)]
    fn eq(&self, other: &Box<'b, T>) -> bool {
        T::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &Box<'b, T>) -> bool {
        T::ne(self, other)
    }
}

impl<T: ?Sized + PartialEq> PartialEq<T> for Box<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &T) -> bool {
        T::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &T) -> bool {
        T::ne(self, other)
    }
}

impl<T: ?Sized + PartialEq> PartialEq<&T> for Box<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &&T) -> bool {
        T::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &&T) -> bool {
        T::ne(self, other)
    }
}

impl<T: ?Sized + PartialEq> PartialEq<&mut T> for Box<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &&mut T) -> bool {
        T::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &&mut T) -> bool {
        T::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<Box<'a, [U]>> for [T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &Box<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &Box<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<Box<'a, [U]>> for &[T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &Box<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &Box<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<Box<'a, [U]>> for &mut [T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &Box<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &Box<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<T: PartialEq, const N: usize> PartialEq<[T; N]> for Box<'_, [T]> {
    #[inline(always)]
    fn eq(&self, other: &[T; N]) -> bool {
        <[T]>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &[T; N]) -> bool {
        <[T]>::ne(self, other)
    }
}

impl<'b, 'a, T: ?Sized + PartialOrd> PartialOrd<Box<'b, T>> for Box<'a, T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Box<'b, T>) -> Option<Ordering> {
        T::partial_cmp(self, other)
    }
    #[inline(always)]
    fn lt(&self, other: &Box<'b, T>) -> bool {
        T::lt(self, other)
    }
    #[inline(always)]
    fn le(&self, other: &Box<'b, T>) -> bool {
        T::le(self, other)
    }
    #[inline(always)]
    fn ge(&self, other: &Box<'b, T>) -> bool {
        T::ge(self, other)
    }
    #[inline(always)]
    fn gt(&self, other: &Box<'b, T>) -> bool {
        T::gt(self, other)
    }
}

impl<'a, T: ?Sized + Ord> Ord for Box<'a, T> {
    #[inline(always)]
    fn cmp(&self, other: &Box<'a, T>) -> Ordering {
        T::cmp(self, other)
    }
}

impl<'a, T: ?Sized + Eq> Eq for Box<'a, T> {}

impl<'a, T: ?Sized + Hash> Hash for Box<'a, T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        T::hash(self, state);
    }
}

impl<'a, T: ?Sized + Hasher> Hasher for Box<'a, T> {
    #[inline(always)]
    fn finish(&self) -> u64 {
        T::finish(self)
    }
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        T::write(self, bytes);
    }
    #[inline(always)]
    fn write_u8(&mut self, i: u8) {
        T::write_u8(self, i);
    }
    #[inline(always)]
    fn write_u16(&mut self, i: u16) {
        T::write_u16(self, i);
    }
    #[inline(always)]
    fn write_u32(&mut self, i: u32) {
        T::write_u32(self, i);
    }
    #[inline(always)]
    fn write_u64(&mut self, i: u64) {
        T::write_u64(self, i);
    }
    #[inline(always)]
    fn write_u128(&mut self, i: u128) {
        T::write_u128(self, i);
    }
    #[inline(always)]
    fn write_usize(&mut self, i: usize) {
        T::write_usize(self, i);
    }
    #[inline(always)]
    fn write_i8(&mut self, i: i8) {
        T::write_i8(self, i);
    }
    #[inline(always)]
    fn write_i16(&mut self, i: i16) {
        T::write_i16(self, i);
    }
    #[inline(always)]
    fn write_i32(&mut self, i: i32) {
        T::write_i32(self, i);
    }
    #[inline(always)]
    fn write_i64(&mut self, i: i64) {
        T::write_i64(self, i);
    }
    #[inline(always)]
    fn write_i128(&mut self, i: i128) {
        T::write_i128(self, i);
    }
    #[inline(always)]
    fn write_isize(&mut self, i: isize) {
        T::write_isize(self, i);
    }
}

#[cfg(feature = "alloc")]
impl<'a> Extend<Box<'a, str>> for alloc::string::String {
    #[inline(always)]
    fn extend<T: IntoIterator<Item = Box<'a, str>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl<'a, T> IntoIterator for Box<'a, [T]> {
    type Item = T;
    type IntoIter = IntoIter<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        let this = ManuallyDrop::new(self);
        unsafe { IntoIter::new(this.ptr) }
    }
}

impl<'b, 'a, T> IntoIterator for &'b Box<'a, [T]> {
    type Item = &'b T;
    type IntoIter = slice::Iter<'b, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        <[T]>::iter(self)
    }
}

impl<'b, 'a, T> IntoIterator for &'b mut Box<'a, [T]> {
    type Item = &'b mut T;
    type IntoIter = slice::IterMut<'b, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        <[T]>::iter_mut(self)
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for Box<'_, [T]> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        <[T]>::index(self, index)
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for Box<'_, [T]> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        <[T]>::index_mut(self, index)
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized + std::io::Read> std::io::Read for Box<'_, T> {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        T::read(self, buf)
    }

    #[inline(always)]
    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
        T::read_vectored(self, bufs)
    }

    #[inline(always)]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        T::read_to_end(self, buf)
    }

    #[inline(always)]
    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        T::read_to_string(self, buf)
    }

    #[inline(always)]
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        T::read_exact(self, buf)
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized + std::io::Write> std::io::Write for Box<'_, T> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        T::write(self, buf)
    }

    #[inline(always)]
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        T::write_vectored(self, bufs)
    }

    #[inline(always)]
    fn flush(&mut self) -> std::io::Result<()> {
        T::flush(self)
    }

    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        T::write_all(self, buf)
    }

    #[inline(always)]
    fn write_fmt(&mut self, fmt: alloc::fmt::Arguments<'_>) -> std::io::Result<()> {
        T::write_fmt(self, fmt)
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized + std::io::Seek> std::io::Seek for Box<'_, T> {
    #[inline(always)]
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        T::seek(self, pos)
    }

    #[inline(always)]
    fn stream_position(&mut self) -> std::io::Result<u64> {
        T::stream_position(self)
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized + std::io::BufRead> std::io::BufRead for Box<'_, T> {
    #[inline(always)]
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        T::fill_buf(self)
    }

    #[inline(always)]
    fn consume(&mut self, amt: usize) {
        T::consume(self, amt);
    }

    #[inline(always)]
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        T::read_until(self, byte, buf)
    }

    #[inline(always)]
    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize> {
        T::read_line(self, buf)
    }
}

#[inline(always)]
fn as_uninit_slice<T>(slice: &[T]) -> &[MaybeUninit<T>] {
    unsafe { &*(slice as *const _ as *const [MaybeUninit<T>]) }
}
