use core::alloc::Allocator;

use alloc::{boxed::Box, fmt};

use crate::{
    BoxU, NoDrop, Unmanaged,
    extra::{box_leak, box_replace_allocator},
    traits::BumpAllocatorScope,
};

/// A bump allocation, returned from one of the `alloc_*` methods of [`BumpAllocator`] or [`MutBumpAllocator`].
///
/// This type is meant to be immediately turned into
/// the desired output type using one of its `.into_*()` methods.
///
/// [`BumpAllocator`]: crate::BumpAllocator
/// [`MutBumpAllocator`]: crate::MutBumpAllocator
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, BoxU};
///
/// let bump: Bump = Bump::new();
///
/// let as_ref: &i32 = bump.alloc(123).into_ref();
/// let as_mut: &mut i32 = bump.alloc(123).into_mut();
/// let as_box: Box<i32, &Bump> = bump.alloc(123).into_box();
/// let as_boxu: BoxU<i32> = bump.alloc(123).into_boxu();
/// ```
#[must_use]
#[repr(transparent)]
pub struct BumpAllocation<T: ?Sized, A: Allocator>(pub(crate) Box<T, A>);

impl<T: fmt::Debug + ?Sized, A: Allocator> fmt::Debug for BumpAllocation<T, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl<T: ?Sized, A: Allocator> BumpAllocation<T, A> {
    /// Returns this bump allocation as a `&T`.
    /// This is only available for [`NoDrop`] types so you don't omit dropping a value for which it matters.
    ///
    /// `!NoDrop` types can still be turned into references with [`leak`](BumpAllocation::leak).
    #[must_use]
    pub fn into_ref<'a>(self) -> &'a T
    where
        T: NoDrop,
        A: BumpAllocatorScope<'a>,
    {
        self.leak()
    }

    /// Returns this bump allocation as a `&mut T`.
    /// This is only available for [`NoDrop`] types so you don't omit dropping a value for which it matters.
    ///
    /// `!NoDrop` types can still be turned into references with [`leak`](BumpAllocation::leak).
    #[must_use]
    pub fn into_mut<'a>(self) -> &'a mut T
    where
        T: NoDrop,
        A: BumpAllocatorScope<'a>,
    {
        self.leak()
    }

    /// Returns this bump allocation as a <code>[Box]\<T, A\></code>.
    #[must_use]
    pub fn into_box(self) -> Box<T, A> {
        self.0
    }

    /// Returns this bump allocation as a <code>[BoxU]\<T\></code>.
    ///
    /// The returned `Box` has a zero-sized dummy allocator which doesn't support
    /// allocations. As such `Box` apis that allocate or grow, like
    /// `.clone()` and `.map(...)` will fail.
    ///
    /// The advantage is that this dummy allocator does not take up space in the `Box`.
    pub fn into_boxu<'a>(self) -> BoxU<'a, T>
    where
        A: BumpAllocatorScope<'a>,
    {
        box_replace_allocator(self.0, Unmanaged::VALUE).0
    }

    /// Returns this bump allocation as a raw pointer.
    ///
    /// The pointer will be properly aligned and non-null.
    pub fn into_raw(self) -> *mut T {
        Box::into_raw_with_allocator(self.0).0
    }

    /// Returns this bump allocation as a `&mut T`.
    /// `T` won't be dropped which may leak resources.
    ///
    /// If `T` is [`NoDrop`], prefer to call [`into_mut`] to signify that nothing gets leaked.
    ///
    /// [`into_mut`]: BumpAllocation::into_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, BumpAllocation};
    ///
    /// let bump: Bump = Bump::new();
    ///
    /// let boxed_slice_of_slices: BumpAllocation<[&mut [i32]], _> = bump.alloc_iter_exact((0..3).map(|_| {
    ///     bump.alloc_slice_fill_with(3, Default::default).into_mut()
    /// }));
    ///
    /// // `&mut T` don't implement `NoDrop` currently because a blanket implementation
    /// // for `&mut T` would conflict with the blanket implementation for `T: Copy`
    /// // (see https://github.com/rust-lang/rust/issues/149650)
    /// // so we need to use `leak`
    /// let slice_of_slices: &mut [&mut [i32]] = boxed_slice_of_slices.leak();
    /// # _ = slice_of_slices;
    /// ```
    pub fn leak<'a>(self) -> &'a mut T
    where
        A: BumpAllocatorScope<'a>,
    {
        box_leak(self.0)
    }
}

/// A bump allocation that can **only** be turned into a shared reference.
///
/// The only method you can call on this type is `.into_ref()`.
///
/// This is only used for `CStr` allocation apis.
pub struct IntoRef<T: ?Sized, A> {
    ptr: *const T,
    #[expect(unused)]
    alloc: A,
}

impl<T: ?Sized, A> IntoRef<T, A> {
    pub(crate) unsafe fn new(ptr: *const T, alloc: A) -> Self {
        Self { ptr, alloc }
    }
}

impl<'a, T: ?Sized, A> IntoRef<T, A>
where
    A: BumpAllocatorScope<'a>,
{
    /// Returns this bump allocation as a `&T`.
    pub fn into_ref(self) -> &'a T
    where
        T: NoDrop,
    {
        unsafe { &*self.ptr }
    }
}
