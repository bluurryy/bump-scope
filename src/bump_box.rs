use core::{
    alloc::Layout,
    any::Any,
    borrow::{Borrow, BorrowMut},
    cmp::Ordering,
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    ops::{self, Deref, DerefMut, Index, IndexMut, Range, RangeBounds},
    ptr::{self, NonNull},
    slice::{self, SliceIndex},
    str,
};

#[cfg(feature = "nightly-fn-traits")]
use core::marker::Tuple;

#[cfg(feature = "std")]
use alloc_crate::{string::String, vec::Vec};

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
use alloc_crate::boxed::Box;

use crate::{
    owned_slice, owned_str,
    polyfill::{self, nonnull, pointer, transmute_mut},
    set_len_on_drop_by_ptr::SetLenOnDropByPtr,
    BumpAllocator, FromUtf8Error, NoDrop, SizedTypeProperties,
};

#[cfg(feature = "alloc")]
use crate::{alloc::BoxLike, BumpAllocatorScope};

mod slice_initializer;

pub(crate) use slice_initializer::BumpBoxSliceInitializer;

/// A pointer type that uniquely owns a bump allocation of type `T`. This type is returned whenever a bump allocation is made.
///
/// You can turn a `BumpBox` into a reference with [`into_ref`] and [`into_mut`] and into a [`Box`] with [`into_box`].
///
/// Unlike `Box`, `BumpBox` can not implement `Clone` or free the allocated space as it does not store its allocator.
/// It's essentially just an owned reference.
///
/// ## `BumpBox` has a lot of methods
/// - `BumpBox<[T]>` provides methods from `Vec<T>` like
///   [`pop`](Self::pop),
///   [`clear`](Self::clear),
///   [`truncate`](Self::truncate),
///   [`remove`](Self::remove),
///   [`swap_remove`](Self::swap_remove),
///   [`retain`](Self::retain),
///   [`drain`](Self::drain),
///   [`extract_if`](Self::extract_if),
///   [`dedup`](Self::dedup),
///   slice methods but with owned semantics like
///   [`split_at`](Self::split_at),
///   [`split_first`](Self::split_first),
///   [`split_last`](Self::split_last) and additional methods like
///   [`split_off`](Self::split_off),
///   [`partition`](Self::partition) and [`map_in_place`](Self::map_in_place).
/// - `BumpBox<str>` provide methods from `String` like
///   <code>[from_utf8](Self::from_utf8)([_unchecked](Self::from_utf8_unchecked))</code>,
///   [`into_boxed_bytes`](Self::into_boxed_bytes),
///   [`as_mut_bytes`](Self::as_mut_bytes),
///   [`pop`](Self::pop),
///   [`truncate`](Self::truncate),
///   [`clear`](Self::clear),
///   [`remove`](Self::remove),
///   [`retain`](Self::retain),
///   [`drain`](Self::drain) and
///   [`split_off`](Self::split_off).
/// - `BumpBox<MaybeUninit<T>>` and `BumpBox<[MaybeUninit<T>]>` provide methods like
///   [`init`](Self::init),
///   [`assume_init`](Self::assume_init),
///   [`init_fill`](Self::init_fill),
///   [`init_fill_with`](Self::init_fill_with),
///   [`init_fill_iter`](Self::init_fill_iter),
///   [`init_copy`](Self::init_copy),
///   [`init_clone`](Self::init_clone) and
///   [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed).
///
/// ## No pinning
///
/// There is no way to safely pin a `BumpBox` in the general case.
/// The [*drop guarantee*] of `Pin` requires the value to be dropped before its memory is reused.
/// Preventing reuse of memory is not an option as that's what this crate is all about.
/// So we need to drop the pinned value.
/// But there is no way to ensure that a value is dropped in an async context.
/// <details>
/// <summary>Example of an unsound pin macro implementation.</summary>
///
/// We define a `bump_box_pin` macro that turns a `BumpBox<T>` into a `Pin<&mut T>`. This is only sound in synchronous code.
/// Here the memory `Foo(1)` is allocated at is reused by `Foo(2)` without dropping `Foo(1)` first which violates the drop guarantee.
///
/// ```
/// # use bump_scope::{Bump, BumpBox};
/// # use std::{mem, task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # #[must_use = "futures do nothing unless you `.await` or poll them"]
/// # struct YieldNow(bool);
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
/// # fn yield_now() -> YieldNow {
/// #     YieldNow(false)
/// # }
/// #
/// macro_rules! bump_box_pin {
///     ($name:ident) => {
///         let mut boxed: BumpBox<_> = $name;
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
/// [`into_ref`]: Self::into_ref
/// [`into_mut`]: BumpBox::into_mut
/// [`into_box`]: BumpBox::into_box
/// [`leak`]: BumpBox::leak
/// [`Box`]: alloc_crate::boxed::Box
/// [*drop guarantee*]: https://doc.rust-lang.org/std/pin/index.html#subtle-details-and-the-drop-guarantee
#[repr(transparent)]
pub struct BumpBox<'a, T: ?Sized> {
    ptr: NonNull<T>,

    /// First field marks the lifetime.
    /// Second field marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    marker: PhantomData<(&'a (), T)>,
}

/// Allows unsizing to be performed on `T` of `BumpBox<T>`.
///
/// This macro is required to unsize the pointee of a `BumpBox` on stable rust.
///
/// On nightly and when the feature "nightly-coerce-unsized" is enabled, `BumpBox` implements `CoerceUnsized` so `T` will coerce just like with [`Box`](alloc_crate::boxed::Box).
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpBox, unsize_bump_box};
/// use core::any::Any;
///
/// let bump: Bump = Bump::new();
///
/// let sized_box: BumpBox<[i32; 3]> = bump.alloc([1, 2, 3]);
/// let unsized_box_slice: BumpBox<[i32]> = unsize_bump_box!(sized_box);
///
/// let sized_box: BumpBox<[i32; 3]> = bump.alloc([1, 2, 3]);
/// let unsized_box_dyn: BumpBox<dyn Any> = unsize_bump_box!(sized_box);
/// #
/// # _ = unsized_box_slice;
/// # _ = unsized_box_dyn;
/// ```
/// On nightly with the feature "nightly-coerce-unsized":
#[cfg_attr(feature = "nightly-coerce-unsized", doc = "```")]
#[cfg_attr(not(feature = "nightly-coerce-unsized"), doc = "```ignore")]
/// use bump_scope::{Bump, BumpBox};
/// use core::any::Any;
///
/// let bump: Bump = Bump::new();
///
/// let sized_box: BumpBox<[i32; 3]> = bump.alloc([1, 2, 3]);
/// let unsized_box_slice: BumpBox<[i32]> = sized_box;
///
/// let sized_box: BumpBox<[i32; 3]> = bump.alloc([1, 2, 3]);
/// let unsized_box_dyn: BumpBox<dyn Any> = sized_box;
/// #
/// # _ = unsized_box_slice;
/// # _ = unsized_box_dyn;
/// ```
#[macro_export]
macro_rules! unsize_bump_box {
    ($boxed:expr) => {{
        let (ptr, lt) = $crate::private::bump_box_into_raw_with_lifetime($boxed);
        let ptr: $crate::private::core::ptr::NonNull<_> = ptr;
        unsafe { $crate::private::bump_box_from_raw_with_lifetime(ptr, lt) }
    }};
}

unsafe impl<T: ?Sized + Send> Send for BumpBox<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for BumpBox<'_, T> {}

impl<T> BumpBox<'_, T> {
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

impl<'a, T: ?Sized + NoDrop> BumpBox<'a, T> {
    /// Turns this `BumpBox<T>` into `&T` that is live for this bump scope.
    /// This is only available for [`NoDrop`] types so you don't omit dropping a value for which it matters.
    ///
    /// `!NoDrop` types can still be turned into references via [`leak`](BumpBox::leak).
    #[must_use]
    #[inline(always)]
    pub fn into_ref(self) -> &'a T {
        self.into_mut()
    }

    /// Turns this `BumpBox<T>` into `&mut T` that is live for this bump scope.
    /// This is only available for [`NoDrop`] types so you don't omit dropping a value for which it matters.
    ///
    /// `!NoDrop` types can still be turned into references via [`leak`](BumpBox::leak).
    #[must_use]
    #[inline(always)]
    pub fn into_mut(self) -> &'a mut T {
        Self::leak(self)
    }
}

impl<'a, T: ?Sized> BumpBox<'a, T> {
    #[must_use]
    #[inline(always)]
    pub(crate) const fn ptr(&self) -> NonNull<T> {
        self.ptr
    }

    #[must_use]
    #[inline(always)]
    pub(crate) unsafe fn mut_ptr(&mut self) -> &mut NonNull<T> {
        &mut self.ptr
    }

    /// Turns this `BumpBox<T>` into `Box<T>`. The `bump` allocator is not required to be
    /// the allocator this box was allocated in.
    ///
    /// Unlike `BumpBox`, `Box` implements `Clone` and frees space iff it is the last allocation:
    #[cfg_attr(feature = "allocator-api2-03", doc = "```")]
    #[cfg_attr(not(feature = "allocator-api2-03"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # use allocator_api2_03::boxed::Box;
    /// # let bump: Bump = Bump::new();
    /// let a: Box<_, _> = bump.alloc(3i32).into_box(&bump);
    /// let b = a.clone();
    /// assert_eq!(a, b);
    /// drop(b);
    /// drop(a);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "alloc")]
    pub fn into_box<A, B>(self, allocator: A) -> B
    where
        A: BumpAllocatorScope<'a>,
        B: BoxLike<T = T, A = A>,
    {
        let ptr = BumpBox::into_raw(self).as_ptr();

        // SAFETY: bump might not be the allocator self was allocated with;
        // that's fine though because a `BumpAllocator` allows deallocate calls
        // from allocations that don't belong to it
        unsafe { B::from_raw_in(ptr, allocator) }
    }

    /// Drops this box and frees its memory iff it is the last allocation:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let boxed = bump.alloc(3i32);
    /// assert_eq!(bump.stats().allocated(), 4);
    /// boxed.deallocate_in(&bump);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    pub fn deallocate_in<A: BumpAllocator>(self, bump: A) {
        let layout = Layout::for_value::<T>(&self);
        let ptr = self.into_raw();

        unsafe {
            nonnull::drop_in_place(ptr);
            bump.deallocate(ptr.cast(), layout);
        }
    }

    /// Turns this `BumpBox<T>` into `&mut T` that is live for this bump scope.
    /// `T` won't be dropped which may leak resources.
    ///
    /// If `T` is [`NoDrop`], prefer to call [`into_mut`](BumpBox::into_mut) to signify that nothing gets leaked.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, BumpBox};
    /// # let bump: Bump = Bump::new();
    /// #
    /// let boxed_slice_of_slices: BumpBox<[&mut [i32]]> = bump.alloc_iter_exact((0..3).map(|_| {
    ///     bump.alloc_slice_fill_with(3, Default::default).into_mut()
    /// }));
    ///
    /// // `&mut T` don't implement `NoDrop` even though they should
    /// // (the blanket implementation `impl<T: Copy> NoDrop for T {}` prevents us from implementing it)
    /// // so we need to use `leak`
    /// let slice_of_slices: &mut [&mut [i32]] = BumpBox::leak(boxed_slice_of_slices);
    /// ```
    #[inline(always)]
    #[allow(clippy::must_use_candidate)]
    pub fn leak(boxed: Self) -> &'a mut T {
        unsafe { BumpBox::into_raw(boxed).as_mut() }
    }

    /// Consumes the `BumpBox`, returning a wrapped raw pointer.
    ///
    /// The pointer will be properly aligned and non-null. It is only valid for the lifetime `'a`.
    ///
    /// After calling this function, the caller is responsible for dropping the
    /// value previously managed by the `BumpBox`. The easiest way to do this is to `p.drop_in_place()`.
    ///
    /// You can turn this pointer back into a `BumpBox` with [`BumpBox::from_raw`].
    ///
    /// # Examples
    /// Manually dropping `T`:
    /// ```
    /// use bump_scope::{Bump, BumpBox};
    /// let bump: Bump = Bump::new();
    /// let x = bump.alloc(String::from("Hello"));
    /// let p = BumpBox::into_raw(x);
    /// unsafe { p.as_ptr().drop_in_place() }
    /// ```
    #[inline(always)]
    #[must_use = "use `leak` if you don't make use of the pointer"]
    #[allow(clippy::needless_pass_by_value)]
    pub fn into_raw(self) -> NonNull<T> {
        ManuallyDrop::new(self).ptr
    }

    /// Constructs a `BumpBox` from a raw pointer.
    ///
    /// After calling this function, the pointed to value is owned by the resulting `BumpBox`.
    /// Specifically, the `BumpBox` destructor will call the destructor of `T`.
    /// For this to be safe, the pointer must point to a valid `T` for the lifetime of `'a`.
    ///
    /// # Safety
    /// - `ptr` must point to a valid value for the lifetime `'a`
    ///
    /// # Examples
    ///
    /// Recreate a `BumpBox` which was previously converted to a raw pointer
    /// using [`BumpBox::into_raw`]:
    /// ```
    /// use bump_scope::{Bump, BumpBox};
    /// use core::ptr::NonNull;
    ///
    /// unsafe fn from_raw_in<'a, T>(ptr: NonNull<T>, bump: &'a Bump) -> BumpBox<'a, T> {
    ///     BumpBox::from_raw(ptr)
    /// }
    ///
    /// let bump: Bump = Bump::new();
    /// let x = bump.alloc(String::from("Hello"));
    /// let ptr = BumpBox::into_raw(x);
    /// let x = unsafe { from_raw_in(ptr, &bump) };
    /// assert_eq!(x.as_str(), "Hello");
    /// drop(x);
    /// ```
    /// Manually create a `BumpBox` from scratch by using the bump allocator:
    /// ```
    /// use bump_scope::{Bump, BumpBox};
    /// use core::alloc::Layout;
    /// use core::ptr::NonNull;
    ///
    /// unsafe fn from_raw_in<'a, T>(ptr: NonNull<T>, bump: &'a Bump) -> BumpBox<'a, T> {
    ///     BumpBox::from_raw(ptr)
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

impl<'a, T> BumpBox<'a, T> {
    /// Consumes the `BumpBox`, returning the wrapped value.
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

    /// Converts a `BumpBox<T>` into a `BumpBox<[T]>`
    ///
    /// This conversion happens in place.
    #[must_use]
    #[inline(always)]
    pub fn into_boxed_slice(self) -> BumpBox<'a, [T]> {
        unsafe {
            let ptr = self.into_raw();
            let ptr = nonnull::slice_from_raw_parts(ptr, 1);
            BumpBox::from_raw(ptr)
        }
    }
}

impl<'a> BumpBox<'a, str> {
    /// Empty str.
    pub const EMPTY_STR: Self = unsafe { BumpBox::from_utf8_unchecked(BumpBox::<[u8]>::EMPTY) };

    /// Converts a `BumpBox<[u8]>` to a `BumpBox<str>`.
    ///
    /// A string ([`BumpBox<str>`]) is made of bytes ([`u8`]), and a vector of bytes
    /// ([`BumpBox<[u8]>`]) is made of bytes, so this function converts between the
    /// two. Not all byte slices are valid `BumpBox<str>`s, however: `BumpBox<str>`
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
    /// If you need a [`&str`] instead of a `BumpBox<str>`, consider
    /// [`str::from_utf8`].
    ///
    /// The inverse of this method is [`into_boxed_bytes`].
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
    /// # use bump_scope::{Bump, BumpBox};
    /// # let bump: Bump = Bump::new();
    /// // some bytes, in a vector
    /// let sparkle_heart = bump.alloc_slice_copy(&[240, 159, 146, 150]);
    ///
    /// // We know these bytes are valid, so we'll use `unwrap()`.
    /// let sparkle_heart = BumpBox::from_utf8(sparkle_heart).unwrap();
    ///
    /// assert_eq!(sparkle_heart, "💖");
    /// ```
    ///
    /// Incorrect bytes:
    /// ```
    /// # use bump_scope::{Bump, BumpBox};
    /// # let bump: Bump = Bump::new();
    /// // some invalid bytes, in a vector
    /// let sparkle_heart = bump.alloc_slice_copy(&[0, 159, 146, 150]);
    ///
    /// assert!(BumpBox::from_utf8(sparkle_heart).is_err());
    /// ```
    ///
    /// [`from_utf8_unchecked`]: Self::from_utf8_unchecked
    /// [`&str`]: prim@str "&str"
    /// [`into_boxed_bytes`]: Self::into_boxed_bytes
    pub const fn from_utf8(bytes: BumpBox<'a, [u8]>) -> Result<Self, FromUtf8Error<BumpBox<'a, [u8]>>> {
        match str::from_utf8(bytes.as_slice()) {
            // SAFETY: `BumpBox<[u8]>` and `BumpBox<str>` have the same representation;
            // only the invariant that the bytes are utf8 is different.
            Ok(_) => Ok(unsafe { mem::transmute(bytes) }),
            Err(error) => Err(FromUtf8Error::new(error, bytes)),
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
    /// # use bump_scope::{Bump, BumpBox};
    /// # let bump: Bump = Bump::new();
    /// // some bytes, in a vector
    /// let sparkle_heart = bump.alloc_slice_copy(&[240, 159, 146, 150]);
    ///
    /// let sparkle_heart = unsafe {
    ///     BumpBox::from_utf8_unchecked(sparkle_heart)
    /// };
    ///
    /// assert_eq!(sparkle_heart, "💖");
    /// ```
    #[must_use]
    pub const unsafe fn from_utf8_unchecked(bytes: BumpBox<'a, [u8]>) -> Self {
        debug_assert!(str::from_utf8(bytes.as_slice()).is_ok());

        // SAFETY: `BumpBox<[u8]>` and `BumpBox<str>` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        mem::transmute(bytes)
    }

    /// Converts a `BumpBox<str>` into a `BumpBox<[u8]>`.
    #[inline]
    #[must_use]
    pub fn into_boxed_bytes(self) -> BumpBox<'a, [u8]> {
        BumpBox {
            ptr: nonnull::str_bytes(self.ptr),
            marker: PhantomData,
        }
    }

    /// Returns a mutable reference to the bytes of this string.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned `&mut BumpBox<[u8]>` allows writing
    /// bytes which are not valid UTF-8. If this constraint is violated, using
    /// the original `BumpBox<str>` after dropping the `&mut BumpBox<[u8]>` may violate memory
    /// safety, as `BumpBox<str>`s must be valid UTF-8.
    #[must_use]
    #[inline(always)]
    pub unsafe fn as_mut_bytes(&mut self) -> &mut BumpBox<'a, [u8]> {
        // SAFETY: `BumpBox<[u8]>` and `BumpBox<str>` have the same representation;
        // only the invariant that the bytes are utf8 is different.
        transmute_mut(self)
    }

    /// Returns the number of bytes in the string, also referred to
    /// as its 'length'.
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        nonnull::str_len(self.ptr)
    }

    /// Returns `true` if the slice contains no elements.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[track_caller]
    pub(crate) fn assert_char_boundary(&self, index: usize) {
        #[cold]
        #[track_caller]
        #[inline(never)]
        fn assert_failed() {
            panic!("index is not on a char boundary")
        }

        if !self.is_char_boundary(index) {
            assert_failed();
        }
    }

    /// Splits the string into two by removing the specified range.
    ///
    /// This method does not allocate and does not change the order of the elements.
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut string = bump.alloc_str("foobarbazqux");
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
        let ptr = nonnull::as_non_null_ptr(nonnull::str_bytes(self.ptr));

        if end == len {
            self.assert_char_boundary(start);

            let lhs = nonnull::slice_from_raw_parts(ptr, start);
            let rhs = nonnull::slice_from_raw_parts(unsafe { nonnull::add(ptr, start) }, len - start);

            self.ptr = nonnull::str_from_utf8(lhs);
            return unsafe { BumpBox::from_raw(nonnull::str_from_utf8(rhs)) };
        }

        if start == 0 {
            self.assert_char_boundary(end);

            let lhs = nonnull::slice_from_raw_parts(ptr, end);
            let rhs = nonnull::slice_from_raw_parts(unsafe { nonnull::add(ptr, end) }, len - end);

            self.ptr = nonnull::str_from_utf8(rhs);
            return unsafe { BumpBox::from_raw(nonnull::str_from_utf8(lhs)) };
        }

        if start == end {
            return BumpBox::EMPTY_STR;
        }

        self.assert_char_boundary(start);
        self.assert_char_boundary(end);

        let head_len = start;
        let tail_len = len - end;

        let range_len = end - start;
        let remaining_len = len - range_len;

        unsafe {
            if head_len < tail_len {
                // move the range of elements to split off to the start
                self.as_mut_bytes().get_unchecked_mut(..end).rotate_right(range_len);

                let lhs = nonnull::slice_from_raw_parts(ptr, range_len);
                let rhs = nonnull::slice_from_raw_parts(nonnull::add(ptr, range_len), remaining_len);

                let lhs = nonnull::str_from_utf8(lhs);
                let rhs = nonnull::str_from_utf8(rhs);

                self.ptr = rhs;

                BumpBox::from_raw(lhs)
            } else {
                // move the range of elements to split off to the end
                self.as_mut_bytes().get_unchecked_mut(start..).rotate_left(range_len);

                let lhs = nonnull::slice_from_raw_parts(ptr, remaining_len);
                let rhs = nonnull::slice_from_raw_parts(nonnull::add(ptr, remaining_len), range_len);

                let lhs = nonnull::str_from_utf8(lhs);
                let rhs = nonnull::str_from_utf8(rhs);

                self.ptr = lhs;

                BumpBox::from_raw(rhs)
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_str("abč");
    ///
    /// assert_eq!(s.pop(), Some('č'));
    /// assert_eq!(s.pop(), Some('b'));
    /// assert_eq!(s.pop(), Some('a'));
    ///
    /// assert_eq!(s.pop(), None);
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<char> {
        let ch = self.chars().next_back()?;
        let new_len = self.len() - ch.len_utf8();
        unsafe {
            self.set_len(new_len);
        }
        Some(ch)
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_str("hello");
    ///
    /// s.truncate(2);
    ///
    /// assert_eq!(s, "he");
    /// ```
    #[inline]
    pub fn truncate(&mut self, new_len: usize) {
        if new_len <= self.len() {
            assert!(self.is_char_boundary(new_len));
            unsafe { self.as_mut_bytes().truncate(new_len) }
        }
    }

    /// Truncates this string, removing all contents.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec};
    /// # let bump: Bump = Bump::new();
    /// let mut str = bump.alloc_str("hello");
    /// str.clear();
    /// assert!(str.is_empty());
    /// ```
    #[inline(always)]
    pub fn clear(&mut self) {
        unsafe {
            self.set_len(0);
        }
    }

    /// Returns a raw pointer to the str, or a dangling raw pointer
    /// valid for zero sized reads.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const u8 {
        // We shadow the str method of the same name to avoid going through
        // `deref`, which creates an intermediate reference.
        self.ptr.as_ptr().cast()
    }

    /// Returns an unsafe mutable pointer to str, or a dangling
    /// raw pointer valid for zero sized reads.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        // We shadow the str method of the same name to avoid going through
        // `deref_mut`, which creates an intermediate reference.
        self.ptr.as_ptr().cast()
    }

    #[inline(always)]
    pub(crate) unsafe fn set_ptr(&mut self, new_ptr: NonNull<u8>) {
        self.ptr = nonnull::str_from_utf8(nonnull::slice_from_raw_parts(new_ptr, self.len()));
    }

    /// Forces the length of the string to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type. Normally changing the length of a boxed slice
    /// is done using one of the safe operations instead, such as
    /// [`truncate`] or [`clear`].
    ///
    /// [`truncate`]: Self::truncate
    /// [`clear`]: Self::clear
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to the `capacity` (capacity is not tracked by this type).
    /// - `new_len` must lie on a `char` boundary
    /// - The elements at `old_len..new_len` must be initialized.
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        let ptr = self.ptr.cast::<u8>();
        let bytes = nonnull::slice_from_raw_parts(ptr, new_len);
        self.ptr = nonnull::str_from_utf8(bytes);
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_str("abç");
    ///
    /// assert_eq!(s.remove(0), 'a');
    /// assert_eq!(s.remove(1), 'ç');
    /// assert_eq!(s.remove(0), 'b');
    /// ```
    #[inline]
    pub fn remove(&mut self, idx: usize) -> char {
        let ch = match self[idx..].chars().next() {
            Some(ch) => ch,
            None => panic!("cannot remove a char from the end of a string"),
        };

        let next = idx + ch.len_utf8();
        let len = self.len();
        unsafe {
            ptr::copy(self.as_ptr().add(next), self.as_mut_ptr().add(idx), len - next);
            self.set_len(len - (next - idx));
        }
        ch
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_str("f_o_ob_ar");
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_str("abcde");
    /// let keep = [false, true, true, false, true];
    /// let mut iter = keep.iter();
    /// s.retain(|_| *iter.next().unwrap());
    /// assert_eq!(s, "bce");
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(char) -> bool,
    {
        struct SetLenOnDrop<'b, 'a> {
            s: &'b mut BumpBox<'a, str>,
            idx: usize,
            del_bytes: usize,
        }

        impl Drop for SetLenOnDrop<'_, '_> {
            fn drop(&mut self) {
                let new_len = self.idx - self.del_bytes;
                debug_assert!(new_len <= self.s.len());
                unsafe { self.s.set_len(new_len) };
            }
        }

        let len = self.len();
        let mut guard = SetLenOnDrop {
            s: self,
            idx: 0,
            del_bytes: 0,
        };

        while guard.idx < len {
            let ch =
                // SAFETY: `guard.idx` is positive-or-zero and less that len so the `get_unchecked`
                // is in bound. `self` is valid UTF-8 like string and the returned slice starts at
                // a unicode code point so the `Chars` always return one character.
                unsafe { guard.s.get_unchecked(guard.idx..len).chars().next().unwrap_unchecked() };
            let ch_len = ch.len_utf8();

            if !f(ch) {
                guard.del_bytes += ch_len;
            } else if guard.del_bytes > 0 {
                // SAFETY: `guard.idx` is in bound and `guard.del_bytes` represent the number of
                // bytes that are erased from the string so the resulting `guard.idx -
                // guard.del_bytes` always represent a valid unicode code point.
                //
                // `guard.del_bytes` >= `ch.len_utf8()`, so taking a slice with `ch.len_utf8()` len
                // is safe.
                ch.encode_utf8(unsafe {
                    slice::from_raw_parts_mut(guard.s.as_mut_ptr().add(guard.idx - guard.del_bytes), ch.len_utf8())
                });
            }

            // Point idx to the next char
            guard.idx += ch_len;
        }

        drop(guard);
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut s = bump.alloc_str("α is alpha, β is beta");
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
        // Memory safety
        //
        // The String version of Drain does not have the memory safety issues
        // of the vector version. The data is just plain bytes.
        // Because the range removal happens in Drop, if the Drain iterator is leaked,
        // the removal will not happen.
        let Range { start, end } = polyfill::slice::range(range, ..self.len());
        assert!(self.is_char_boundary(start));
        assert!(self.is_char_boundary(end));

        // Take out two simultaneous borrows. The &mut String won't be accessed
        // until iteration is over, in Drop.
        let self_ptr = unsafe { NonNull::new_unchecked(self as *mut _) };
        // SAFETY: `slice::range` and `is_char_boundary` do the appropriate bounds checks.
        let chars_iter = unsafe { self.get_unchecked(start..end) }.chars();

        owned_str::Drain {
            start,
            end,
            iter: chars_iter,
            string: self_ptr,
        }
    }
}

impl<'a, T: Sized> BumpBox<'a, MaybeUninit<T>> {
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
    pub fn init(mut self, value: T) -> BumpBox<'a, T> {
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
    pub unsafe fn assume_init(self) -> BumpBox<'a, T> {
        let ptr = BumpBox::into_raw(self);
        BumpBox::from_raw(ptr.cast())
    }
}

impl<'a, T: Sized> BumpBox<'a, [MaybeUninit<T>]> {
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
    pub fn init_fill(self, value: T) -> BumpBox<'a, [T]>
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
                BumpBox::default()
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
    pub fn init_fill_with(self, mut f: impl FnMut() -> T) -> BumpBox<'a, [T]> {
        let mut initializer = self.initializer();

        while !initializer.is_full() {
            initializer.push_with(&mut f);
        }

        initializer.into_init()
    }

    /// Initializes `self` by filling it with elements returned from an iterator.
    ///
    /// # Panics
    ///
    /// This function will panic if the iterator runs out of items before the slice is filled.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let buf = bump.alloc_uninit_slice(5);
    /// let buf = buf.init_fill_iter(['a', 'b'].iter().copied().cycle());
    /// assert_eq!(buf, ['a', 'b', 'a', 'b', 'a']);
    /// ```
    #[must_use]
    #[inline]
    pub fn init_fill_iter(self, mut iter: impl Iterator<Item = T>) -> BumpBox<'a, [T]> {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn iter_ran_out() -> ! {
            panic!("iterator ran out of items to fill the slice with");
        }

        let mut initializer = self.initializer();

        while !initializer.is_full() {
            match iter.next() {
                Some(item) => {
                    initializer.push(item);
                }
                None => iter_ran_out(),
            }
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
    pub fn init_copy(mut self, slice: &[T]) -> BumpBox<'a, [T]>
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
    pub fn init_clone(self, slice: &[T]) -> BumpBox<'a, [T]>
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
    pub(crate) fn initializer(self) -> BumpBoxSliceInitializer<'a, T> {
        BumpBoxSliceInitializer::new(self)
    }

    /// # Safety
    ///
    /// It is up to the caller to guarantee that each `MaybeUninit<T>` really is in an initialized state. Calling this when the content is not yet fully initialized causes immediate undefined behavior.
    ///
    /// See [`MaybeUninit::assume_init`].
    #[must_use]
    #[inline(always)]
    pub unsafe fn assume_init(self) -> BumpBox<'a, [T]> {
        let ptr = BumpBox::into_raw(self);
        let ptr = NonNull::new_unchecked(ptr.as_ptr() as _);
        BumpBox::from_raw(ptr)
    }
}

impl<'a, T> BumpBox<'a, [T]> {
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
        BumpBox::uninit_zst_slice(slice.len()).init_clone(slice)
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
            BumpBox::EMPTY
        } else {
            for _ in 1..len {
                mem::forget(value.clone());
            }

            mem::forget(value);
            unsafe { BumpBox::zst_slice_from_len(len) }
        }
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn zst_slice_fill_with(len: usize, f: impl FnMut() -> T) -> Self {
        assert!(T::IS_ZST);
        BumpBox::uninit_zst_slice(len).init_fill_with(f)
    }

    /// Creates `T` values from nothing!
    #[must_use]
    #[inline(always)]
    pub(crate) unsafe fn zst_slice_from_len(len: usize) -> Self {
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
    /// # use bump_scope::{Bump, mut_bump_vec};
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
    /// # use bump_scope::{Bump, mut_bump_vec};
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
    /// # use bump_scope::{Bump, mut_bump_vec};
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
    /// # use bump_scope::{Bump, mut_bump_vec};
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let mut slice = bump.alloc_slice_copy(&[1, 2, 3]);
    /// slice.truncate(0);
    /// assert_eq!(slice, []);
    /// ```
    ///
    /// [`clear`]: Self::clear
    /// [`drain`]: Self::drain
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
    /// See the example below for how this guarantee can be used.
    ///
    /// # Examples
    ///
    /// Due to the aliasing guarantee, the following code is legal:
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// unsafe {
    ///     let mut v = bump.alloc_slice_copy(&[0]);
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
        self.ptr.cast()
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

    #[inline(always)]
    pub(crate) unsafe fn set_ptr(&mut self, new_ptr: NonNull<T>) {
        nonnull::set_ptr(&mut self.ptr, new_ptr);
    }

    /// Forces the length of the slice to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type. Normally changing the length of a boxed slice
    /// is done using one of the safe operations instead, such as
    /// [`truncate`] or [`clear`].
    ///
    /// [`truncate`]: Self::truncate
    /// [`clear`]: Self::clear
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to the `capacity` (capacity is not tracked by this type).
    /// - The elements at `old_len..new_len` must be initialized.
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

    #[inline(always)]
    pub(crate) unsafe fn set_len_on_drop(&mut self) -> SetLenOnDropByPtr<T> {
        SetLenOnDropByPtr::new(&mut self.ptr)
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
    ///
    /// ```
    /// # use bump_scope::{Bump, mut_bump_vec};
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
    ///
    /// ```
    /// # use bump_scope::Bump;
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
    /// Splits the vector into two by removing the specified range.
    ///
    /// This method does not allocate and does not change the order of the elements.
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
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6, 7, 8]);
    ///
    /// let front = slice.split_off(..2);
    /// assert_eq!(front, [1, 2]);
    /// assert_eq!(slice, [3, 4, 5, 6, 7, 8]);
    ///
    /// let back = slice.split_off(4..);
    /// assert_eq!(back, [7, 8]);
    /// assert_eq!(slice, [3, 4, 5, 6]);
    ///
    /// let middle = slice.split_off(1..3);
    /// assert_eq!(middle, [4, 5]);
    /// assert_eq!(slice, [3, 6]);
    ///
    /// let rest = slice.split_off(..);
    /// assert_eq!(rest, [3, 6]);
    /// assert_eq!(slice, []);
    /// ```
    #[inline]
    #[allow(clippy::return_self_not_must_use)]
    pub fn split_off(&mut self, range: impl RangeBounds<usize>) -> Self {
        let len = self.len();
        let ops::Range { start, end } = polyfill::slice::range(range, ..len);
        let ptr = nonnull::as_non_null_ptr(self.ptr);

        if T::IS_ZST {
            let range_len = end - start;
            let remaining_len = len - range_len;

            unsafe {
                self.set_len(remaining_len);
                return BumpBox::zst_slice_from_len(range_len);
            }
        }

        if end == len {
            let lhs = nonnull::slice_from_raw_parts(ptr, start);
            let rhs = nonnull::slice_from_raw_parts(unsafe { nonnull::add(ptr, start) }, len - start);

            self.ptr = lhs;
            return unsafe { BumpBox::from_raw(rhs) };
        }

        if start == 0 {
            let lhs = nonnull::slice_from_raw_parts(ptr, end);
            let rhs = nonnull::slice_from_raw_parts(unsafe { nonnull::add(ptr, end) }, len - end);

            self.ptr = rhs;
            return unsafe { BumpBox::from_raw(lhs) };
        }

        if start == end {
            return BumpBox::EMPTY;
        }

        let head_len = start;
        let tail_len = len - end;

        let range_len = end - start;
        let remaining_len = len - range_len;

        unsafe {
            if head_len < tail_len {
                // move the range of elements to split off to the start
                self.as_mut_slice().get_unchecked_mut(..end).rotate_right(range_len);

                let lhs = nonnull::slice_from_raw_parts(ptr, range_len);
                let rhs = nonnull::slice_from_raw_parts(unsafe { nonnull::add(ptr, range_len) }, remaining_len);

                self.ptr = rhs;

                BumpBox::from_raw(lhs)
            } else {
                // move the range of elements to split off to the end
                self.as_mut_slice().get_unchecked_mut(start..).rotate_left(range_len);

                let lhs = nonnull::slice_from_raw_parts(ptr, remaining_len);
                let rhs = nonnull::slice_from_raw_parts(unsafe { nonnull::add(ptr, remaining_len) }, range_len);

                self.ptr = lhs;

                BumpBox::from_raw(rhs)
            }
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
    pub fn split_at(self, at: usize) -> (Self, Self) {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(at: usize, len: usize) -> ! {
            panic!("`at` split index (is {at}) should be <= len (is {len})");
        }

        if at > self.len() {
            assert_failed(at, self.len());
        }

        // SAFETY: `[ptr; mid]` and `[mid; len]` are inside `self`, which
        // fulfills the requirements of `split_at_unchecked`.
        unsafe { self.split_at_unchecked(at) }
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
    /// [`split_at`]: Self::split_at
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
    /// This does consume the `BumpBox`. You can create a new empty one with [`BumpBox::default`](BumpBox::default).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
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
    pub fn split_first(self) -> Option<(BumpBox<'a, T>, BumpBox<'a, [T]>)> {
        let this = ManuallyDrop::new(self);

        if this.is_empty() {
            return None;
        }

        unsafe {
            let ptr = this.ptr.cast::<T>();
            let len = this.len();

            Some((
                BumpBox::from_raw(ptr),
                BumpBox::from_raw(nonnull::slice_from_raw_parts(nonnull::add(ptr, 1), len - 1)),
            ))
        }
    }

    /// Returns the last and all the rest of the elements of the slice, or `None` if it is empty.
    ///
    /// This does consume the `BumpBox`. You can create a new empty one with [`BumpBox::default`](BumpBox::default).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
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
    pub fn split_last(self) -> Option<(BumpBox<'a, T>, BumpBox<'a, [T]>)> {
        let this = ManuallyDrop::new(self);

        if this.is_empty() {
            return None;
        }

        unsafe {
            let ptr = this.ptr.cast::<T>();
            let len_minus_one = this.len() - 1;

            Some((
                BumpBox::from_raw(nonnull::add(ptr, len_minus_one)),
                BumpBox::from_raw(nonnull::slice_from_raw_parts(ptr, len_minus_one)),
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
            v: &'b mut BumpBox<'a, [T]>,
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
    pub fn drain<R>(&mut self, range: R) -> owned_slice::Drain<'_, T>
    where
        R: RangeBounds<usize>,
    {
        owned_slice::Drain::new(self, range)
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
    ///
    /// [`retain`]: Self::retain
    pub fn extract_if<F>(&mut self, filter: F) -> owned_slice::ExtractIf<T, F>
    where
        F: FnMut(&mut T) -> bool,
    {
        owned_slice::ExtractIf::new(self, filter)
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
            boxed: &'b mut BumpBox<'a, [T]>,
        }

        impl<T> Drop for FillGapOnDrop<'_, '_, T> {
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

    /// Consumes `self`, creating two boxed slices from it.
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
    /// let slice = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6, 7]);
    ///
    /// let (even, odd) = slice.partition(|n| n % 2 == 0);
    ///
    /// assert!(even.iter().all(|n| n % 2 == 0));
    /// assert!(odd.iter().all(|n| n % 2 != 0));
    /// ```
    pub fn partition<F>(mut self, f: F) -> (Self, Self)
    where
        F: FnMut(&T) -> bool,
    {
        let index = polyfill::iter::partition_in_place(self.iter_mut(), f);
        self.split_at(index)
    }

    /// Returns a boxed slice of the same size as `self`, with function `f` applied to each element in order.
    ///
    /// This function only compiles when `U`s size and alignment is less or equal to `T`'s or if `U` has a size of 0.
    ///
    /// # Examples
    /// Mapping to a type with an equal alignment and size:
    /// ```
    /// # use bump_scope::{Bump};
    /// # use core::num::NonZero;
    /// # let bump: Bump = Bump::new();
    /// let a = bump.alloc_slice_copy(&[0, 1, 2]);
    /// let b = a.map_in_place(NonZero::new);
    /// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
    /// ```
    ///
    /// Mapping to a type with a smaller alignment and size:
    /// ```
    /// # use bump_scope::{Bump, BumpBox};
    /// # let bump: Bump = Bump::new();
    /// let a: BumpBox<[u32]> = bump.alloc_slice_copy(&[0, 1, 2]);
    /// let b: BumpBox<[u16]> = a.map_in_place(|i| i as u16);
    /// assert_eq!(b, [0, 1, 2]);
    /// ```
    ///
    /// Mapping to a type with a higher alignment or size won't compile:
    /// ```compile_fail,E0080
    /// # use bump_scope::{Bump, BumpBox};
    /// # let bump: Bump = Bump::new();
    /// let a: BumpBox<[u16]> = bump.alloc_slice_copy(&[0, 1, 2]);
    /// let b: BumpBox<[u32]> = a.map_in_place(|i| i as u32);
    /// # _ = b;
    /// ```
    pub fn map_in_place<U>(self, mut f: impl FnMut(T) -> U) -> BumpBox<'a, [U]> {
        assert_in_place_mappable!(T, U);

        if U::IS_ZST {
            return BumpBox::uninit_zst_slice(self.len()).init_fill_iter(self.into_iter().map(f));
        }

        struct DropGuard<T, U> {
            ptr: NonNull<T>,
            end: *mut T,
            src: *mut T,
            dst: *mut U,
        }

        impl<T, U> Drop for DropGuard<T, U> {
            fn drop(&mut self) {
                unsafe {
                    // drop `T`s
                    let drop_ptr = self.src.add(1);
                    let drop_len = pointer::offset_from_unsigned(self.end, drop_ptr);
                    ptr::slice_from_raw_parts_mut(drop_ptr, drop_len).drop_in_place();

                    // drop `U`s
                    let drop_ptr = self.ptr.cast::<U>().as_ptr();
                    let drop_len = pointer::offset_from_unsigned(self.dst, drop_ptr);
                    ptr::slice_from_raw_parts_mut(drop_ptr, drop_len).drop_in_place();
                }
            }
        }

        let slice = self.into_raw();
        let ptr = slice.cast::<T>();
        let len = slice.len();

        unsafe {
            let mut guard = DropGuard::<T, U> {
                ptr,
                end: ptr.as_ptr().add(len),
                src: ptr.as_ptr(),
                dst: ptr.as_ptr().cast::<U>(),
            };

            while guard.src < guard.end {
                let src_value = guard.src.read();
                let dst_value = f(src_value);
                guard.dst.write(dst_value);
                guard.src = guard.src.add(1);
                guard.dst = guard.dst.add(1);
            }

            mem::forget(guard);

            BumpBox::from_raw(nonnull::slice_from_raw_parts(ptr.cast(), len))
        }
    }
}

impl<'a, T, const N: usize> BumpBox<'a, [T; N]> {
    /// Converts this `BumpBox<[T; N]>` into a `BumpBox<[T]>`.
    ///
    /// ```
    /// # use bump_scope::{Bump, BumpBox};
    /// # let bump: Bump = Bump::new();
    /// // explicit types are just for demonstration
    /// let array: BumpBox<[i32; 3]> = bump.alloc([1, 2, 3]);
    /// let slice: BumpBox<[i32]> = array.into_unsized();
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn into_unsized(self) -> BumpBox<'a, [T]> {
        let ptr = nonnull::as_non_null_ptr(self.into_raw());
        let slice = nonnull::slice_from_raw_parts(ptr, N);
        unsafe { BumpBox::from_raw(slice) }
    }
}

impl<'a, T, const N: usize> BumpBox<'a, [[T; N]]> {
    /// Takes a `BumpBox<[[T; N]]>` and flattens it into a `BumpBox<[T]>`.
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
    pub fn into_flattened(self) -> BumpBox<'a, [T]> {
        let ptr = self.into_raw();
        let len = ptr.len();

        let new_len = if T::IS_ZST {
            len.checked_mul(N).expect("slice len overflow")
        } else {
            // SAFETY:
            // - `len * N` cannot overflow because the allocation is already in
            // the address space.
            // - Each `[T; N]` has `N` valid elements, so there are `len * N`
            // valid elements in the allocation.
            unsafe { polyfill::usize::unchecked_mul(len, N) }
        };

        unsafe { BumpBox::from_raw(nonnull::slice_from_raw_parts(ptr.cast(), new_len)) }
    }
}

impl<'a> BumpBox<'a, dyn Any> {
    /// Attempt to downcast the box to a concrete type.
    #[inline(always)]
    #[allow(clippy::missing_errors_doc)]
    pub fn downcast<T: Any>(self) -> Result<BumpBox<'a, T>, Self> {
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
    pub unsafe fn downcast_unchecked<T: Any>(self) -> BumpBox<'a, T> {
        debug_assert!(self.is::<T>());
        BumpBox::from_raw(BumpBox::into_raw(self).cast())
    }
}

impl<'a> BumpBox<'a, dyn Any + Send> {
    /// Attempt to downcast the box to a concrete type.
    #[allow(clippy::missing_errors_doc)]
    #[inline(always)]
    pub fn downcast<T: Any>(self) -> Result<BumpBox<'a, T>, Self> {
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
    pub unsafe fn downcast_unchecked<T: Any>(self) -> BumpBox<'a, T> {
        debug_assert!(self.is::<T>());
        BumpBox::from_raw(BumpBox::into_raw(self).cast())
    }
}

impl<'a> BumpBox<'a, dyn Any + Send + Sync> {
    /// Attempt to downcast the box to a concrete type.
    #[allow(clippy::missing_errors_doc)]
    #[inline(always)]
    pub fn downcast<T: Any>(self) -> Result<BumpBox<'a, T>, Self> {
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
    pub unsafe fn downcast_unchecked<T: Any>(self) -> BumpBox<'a, T> {
        debug_assert!(self.is::<T>());
        BumpBox::from_raw(BumpBox::into_raw(self).cast())
    }
}

// just like std's Rc and Arc this implements Unpin,
// at time of writing Box does not implement Unpin unconditionally, but that is probably an oversight
// see https://github.com/rust-lang/rust/pull/118634
impl<T: ?Sized> Unpin for BumpBox<'_, T> {}

#[cfg(feature = "nightly-coerce-unsized")]
impl<'a, T, U> core::ops::CoerceUnsized<BumpBox<'a, U>> for BumpBox<'a, T>
where
    T: ?Sized + core::marker::Unsize<U>,
    U: ?Sized,
{
}

impl<T: ?Sized> Drop for BumpBox<'_, T> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { self.ptr.as_ptr().drop_in_place() }
    }
}

impl<T> NoDrop for BumpBox<'_, T> where T: NoDrop {}

impl<T: ?Sized> Deref for BumpBox<'_, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for BumpBox<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T: ?Sized> AsRef<T> for BumpBox<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> AsMut<T> for BumpBox<'_, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized> Borrow<T> for BumpBox<'_, T> {
    #[inline(always)]
    fn borrow(&self) -> &T {
        self
    }
}

impl<T: ?Sized> BorrowMut<T> for BumpBox<'_, T> {
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized + Debug> Debug for BumpBox<'_, T> {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: ?Sized + Display> Display for BumpBox<'_, T> {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        T::fmt(self, f)
    }
}

impl<T> Default for BumpBox<'_, [T]> {
    #[inline(always)]
    fn default() -> Self {
        unsafe { Self::from_raw(NonNull::from(&mut [])) }
    }
}

impl Default for BumpBox<'_, str> {
    #[inline(always)]
    fn default() -> Self {
        unsafe { Self::from_raw(NonNull::from(core::str::from_utf8_unchecked_mut(&mut []))) }
    }
}

impl<'b, T: ?Sized + PartialEq> PartialEq<BumpBox<'b, T>> for BumpBox<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &BumpBox<'b, T>) -> bool {
        T::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &BumpBox<'b, T>) -> bool {
        T::ne(self, other)
    }
}

impl<T: ?Sized + PartialEq> PartialEq<T> for BumpBox<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &T) -> bool {
        T::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &T) -> bool {
        T::ne(self, other)
    }
}

impl<T: ?Sized + PartialEq> PartialEq<&T> for BumpBox<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &&T) -> bool {
        T::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &&T) -> bool {
        T::ne(self, other)
    }
}

impl<T: ?Sized + PartialEq> PartialEq<&mut T> for BumpBox<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &&mut T) -> bool {
        T::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &&mut T) -> bool {
        T::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<BumpBox<'a, [U]>> for [T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &BumpBox<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &BumpBox<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<BumpBox<'a, [U]>> for &[T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &BumpBox<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &BumpBox<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<'a, T, U> PartialEq<BumpBox<'a, [U]>> for &mut [T]
where
    T: PartialEq<U>,
{
    #[inline(always)]
    fn eq(&self, other: &BumpBox<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &BumpBox<'a, [U]>) -> bool {
        <[T] as PartialEq<[U]>>::ne(self, other)
    }
}

impl<T: PartialEq, const N: usize> PartialEq<[T; N]> for BumpBox<'_, [T]> {
    #[inline(always)]
    fn eq(&self, other: &[T; N]) -> bool {
        <[T]>::eq(self, other)
    }

    #[inline(always)]
    fn ne(&self, other: &[T; N]) -> bool {
        <[T]>::ne(self, other)
    }
}

impl<'b, T: ?Sized + PartialOrd> PartialOrd<BumpBox<'b, T>> for BumpBox<'_, T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &BumpBox<'b, T>) -> Option<Ordering> {
        T::partial_cmp(self, other)
    }
    #[inline(always)]
    fn lt(&self, other: &BumpBox<'b, T>) -> bool {
        T::lt(self, other)
    }
    #[inline(always)]
    fn le(&self, other: &BumpBox<'b, T>) -> bool {
        T::le(self, other)
    }
    #[inline(always)]
    fn ge(&self, other: &BumpBox<'b, T>) -> bool {
        T::ge(self, other)
    }
    #[inline(always)]
    fn gt(&self, other: &BumpBox<'b, T>) -> bool {
        T::gt(self, other)
    }
}

impl<'a, T: ?Sized + Ord> Ord for BumpBox<'a, T> {
    #[inline(always)]
    fn cmp(&self, other: &BumpBox<'a, T>) -> Ordering {
        T::cmp(self, other)
    }
}

impl<T: ?Sized + Eq> Eq for BumpBox<'_, T> {}

impl<T: ?Sized + Hash> Hash for BumpBox<'_, T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        T::hash(self, state);
    }
}

impl<T: ?Sized + Hasher> Hasher for BumpBox<'_, T> {
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
impl<'a> Extend<BumpBox<'a, str>> for alloc_crate::string::String {
    #[inline(always)]
    fn extend<T: IntoIterator<Item = BumpBox<'a, str>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl<'a, T> IntoIterator for BumpBox<'a, [T]> {
    type Item = T;
    type IntoIter = owned_slice::IntoIter<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        let this = ManuallyDrop::new(self);
        unsafe { owned_slice::IntoIter::new(this.ptr) }
    }
}

impl<'b, T> IntoIterator for &'b BumpBox<'_, [T]> {
    type Item = &'b T;
    type IntoIter = slice::Iter<'b, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        <[T]>::iter(self)
    }
}

impl<'b, T> IntoIterator for &'b mut BumpBox<'_, [T]> {
    type Item = &'b mut T;
    type IntoIter = slice::IterMut<'b, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        <[T]>::iter_mut(self)
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for BumpBox<'_, [T]> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        <[T]>::index(self, index)
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for BumpBox<'_, [T]> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        <[T]>::index_mut(self, index)
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized + std::io::Read> std::io::Read for BumpBox<'_, T> {
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
impl<T: ?Sized + std::io::Write> std::io::Write for BumpBox<'_, T> {
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
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        T::write_fmt(self, fmt)
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized + std::io::Seek> std::io::Seek for BumpBox<'_, T> {
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
impl<T: ?Sized + std::io::BufRead> std::io::BufRead for BumpBox<'_, T> {
    #[inline(always)]
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        T::fill_buf(self)
    }

    #[inline(always)]
    fn consume(&mut self, amt: usize) {
        T::consume(self, amt);
    }

    #[inline(always)]
    fn read_until(&mut self, byte: u8, buf: &mut std::vec::Vec<u8>) -> std::io::Result<usize> {
        T::read_until(self, byte, buf)
    }

    #[inline(always)]
    fn read_line(&mut self, buf: &mut std::string::String) -> std::io::Result<usize> {
        T::read_line(self, buf)
    }
}

#[cfg(feature = "nightly-fn-traits")]
impl<Args: Tuple, F: FnOnce<Args> + ?Sized> FnOnce<Args> for BumpBox<'_, F> {
    type Output = <F as FnOnce<Args>>::Output;

    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        use alloc_crate::{
            alloc::{AllocError, Allocator},
            boxed::Box,
        };

        struct NoopAllocator;

        // SAFETY:
        // An allocator that always fails allocation and does nothing on deallocation
        // satisfies the safety invariants.
        unsafe impl Allocator for NoopAllocator {
            fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
                Err(AllocError)
            }

            unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
        }

        let ptr = self.into_raw().as_ptr();

        // SAFETY:
        // The allocator will only be used to call `deallocate` which does nothing
        // just like the `BumpBox` would do no deallocation on drop.
        //
        // The `Box` or its allocator do not encode the lifetime of the `BumpBox` but
        // it's fine here because it only lives in this function.
        unsafe {
            let boxed = Box::from_raw_in(ptr, NoopAllocator);
            <Box<F, NoopAllocator> as FnOnce<Args>>::call_once(boxed, args)
        }
    }
}

#[cfg(feature = "nightly-fn-traits")]
impl<Args: Tuple, F: FnMut<Args> + ?Sized> FnMut<Args> for BumpBox<'_, F> {
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        <F as FnMut<Args>>::call_mut(self, args)
    }
}

#[cfg(feature = "nightly-fn-traits")]
impl<Args: Tuple, F: Fn<Args> + ?Sized> Fn<Args> for BumpBox<'_, F> {
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        <F as Fn<Args>>::call(self, args)
    }
}

#[inline(always)]
fn as_uninit_slice<T>(slice: &[T]) -> &[MaybeUninit<T>] {
    unsafe { &*(slice as *const _ as *const [MaybeUninit<T>]) }
}

macro_rules! assert_in_place_mappable {
    ($src:ty, $dst:ty) => {
        #[allow(unused_variables)]
        let _assert = AssertInPlaceMappable::<$src, $dst>::ASSERT;
    };
}

// False positive; i need `pub(self)` to forward declare it.
// Useless attribute is needed for msrv clippy.
#[allow(clippy::useless_attribute)]
#[allow(clippy::needless_pub_self)]
pub(self) use assert_in_place_mappable;

struct AssertInPlaceMappable<Src, Dst>(PhantomData<(Src, Dst)>);

impl<Src, Dst> AssertInPlaceMappable<Src, Dst> {
    #[allow(dead_code)]
    const ASSERT: () = assert!(
        Dst::IS_ZST || (Src::ALIGN >= Dst::ALIGN && Src::SIZE >= Dst::SIZE),
        "`map_in_place` only compiles when `U`s size and alignment is less or equal to `T`'s or if `U` has a size of 0"
    );
}

const _: () = {
    #[repr(align(1024))]
    struct AlignedZst;
    assert_in_place_mappable!(u32, u32);
    assert_in_place_mappable!(u32, Option<core::num::NonZeroU32>);
    assert_in_place_mappable!(u32, [u8; 4]);
    assert_in_place_mappable!(u32, [u16; 2]);
    assert_in_place_mappable!(u32, ());
    assert_in_place_mappable!(u32, AlignedZst);
    assert_in_place_mappable!((), ());
    assert_in_place_mappable!((), AlignedZst);
};
