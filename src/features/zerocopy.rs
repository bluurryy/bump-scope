use crate::{
    define_alloc_methods, error_behavior::ErrorBehavior, BaseAllocator, Bump, BumpBox, BumpScope, MinimumAlignment,
    SupportedMinimumAlignment,
};
use core::mem::MaybeUninit;
use zerocopy::FromZeros;

impl<'a, T> BumpBox<'a, MaybeUninit<T>>
where
    T: FromZeros,
{
    /// Initializes `self` by filling it with zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit::<i32>();
    /// let init = uninit.init_zeroed();
    /// assert_eq!(*init, 0);
    /// ```
    #[must_use]
    pub fn init_zeroed(self) -> BumpBox<'a, T> {
        unsafe {
            self.ptr.as_ptr().write_bytes(0, 1);
            self.assume_init()
        }
    }
}

impl<'a, T> BumpBox<'a, [MaybeUninit<T>]>
where
    T: FromZeros,
{
    /// Initializes `self` by filling it with zeroes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit_slice::<i32>(10);
    /// let init = uninit.init_zeroed();
    /// assert_eq!(*init, [0; 10]);
    /// ```
    #[must_use]
    pub fn init_zeroed(self) -> BumpBox<'a, [T]> {
        unsafe {
            let len = self.len();

            self.ptr.as_ptr().cast::<T>().write_bytes(0, len);
            self.assume_init()
        }
    }
}

define_alloc_methods! {
    macro alloc_zeroed_methods

    /// Allocate a zeroed object.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let zero = bump.alloc_zeroed::<i32>();
    /// assert_eq!(*zero, 0);
    /// ```
    for fn alloc_zeroed
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::try_new()?;
    /// let zero = bump.try_alloc_zeroed::<i32>()?;
    /// assert_eq!(*zero, 0);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_zeroed
    use fn generic_alloc_zeroed<{T}>(&self) -> BumpBox<T> | BumpBox<'a, T>
    where {
        T: FromZeros
    };

    /// Allocate a zeroed object slice.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let zeroes = bump.alloc_zeroed_slice::<i32>(3);
    /// assert_eq!(*zeroes, [0; 3]);
    /// ```
    for fn alloc_zeroed_slice
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::try_new()?;
    /// let zeroes = bump.try_alloc_zeroed_slice::<i32>(3)?;
    /// assert_eq!(*zeroes, [0; 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_zeroed_slice
    use fn generic_alloc_zeroed_slice<{T}>(&self, len: usize) -> BumpBox<[T]> | BumpBox<'a, [T]>
    where {
        T: FromZeros
    };
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    alloc_zeroed_methods!(BumpScope);
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    alloc_zeroed_methods!(Bump);
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    pub(crate) fn generic_alloc_zeroed<B: ErrorBehavior, T>(&self) -> Result<BumpBox<'a, T>, B>
    where
        T: FromZeros,
    {
        Ok(self.generic_alloc_uninit::<B, T>()?.init_zeroed())
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_zeroed_slice<B: ErrorBehavior, T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, B>
    where
        T: FromZeros,
    {
        Ok(self.generic_alloc_uninit_slice::<B, T>(len)?.init_zeroed())
    }
}
