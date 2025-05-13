use core::mem::MaybeUninit;

use zerocopy::FromZeros;

use crate::{
    alloc::AllocError, error_behavior::ErrorBehavior, BaseAllocator, Bump, BumpBox, BumpScope, MinimumAlignment,
    SupportedMinimumAlignment,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

impl<'a, T: FromZeros> BumpBox<'a, MaybeUninit<T>> {
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
    pub fn init_zeroed(mut self) -> BumpBox<'a, T> {
        unsafe {
            self.as_mut_ptr().write_bytes(0, 1);
            self.assume_init()
        }
    }
}

impl<'a, T: FromZeros> BumpBox<'a, [MaybeUninit<T>]> {
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
    pub fn init_zeroed(mut self) -> BumpBox<'a, [T]> {
        unsafe {
            let len = self.len();
            self.as_mut_ptr().write_bytes(0, len);
            self.assume_init()
        }
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Allocate a zeroed object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let zero = bump.alloc_zeroed::<i32>();
    /// assert_eq!(*zero, 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_zeroed<T>(&self) -> BumpBox<T>
    where
        T: FromZeros,
    {
        self.as_scope().alloc_zeroed()
    }

    /// Allocate a zeroed object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let zero = bump.try_alloc_zeroed::<i32>()?;
    /// assert_eq!(*zero, 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<T>, AllocError>
    where
        T: FromZeros,
    {
        self.as_scope().try_alloc_zeroed()
    }

    /// Allocate a zeroed object slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let zeroes = bump.alloc_zeroed_slice::<i32>(3);
    /// assert_eq!(*zeroes, [0; 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<[T]>
    where
        T: FromZeros,
    {
        self.as_scope().alloc_zeroed_slice(len)
    }

    /// Allocate a zeroed object slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let zeroes = bump.try_alloc_zeroed_slice::<i32>(3)?;
    /// assert_eq!(*zeroes, [0; 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<[T]>, AllocError>
    where
        T: FromZeros,
    {
        self.as_scope().try_alloc_zeroed_slice(len)
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Allocate a zeroed object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let zero = bump.alloc_zeroed::<i32>();
    /// assert_eq!(*zero, 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_zeroed<T>(&self) -> BumpBox<'a, T>
    where
        T: FromZeros,
    {
        panic_on_error(self.generic_alloc_zeroed())
    }

    /// Allocate a zeroed object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let zero = bump.try_alloc_zeroed::<i32>()?;
    /// assert_eq!(*zero, 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<'a, T>, AllocError>
    where
        T: FromZeros,
    {
        self.generic_alloc_zeroed()
    }

    /// Allocate a zeroed object slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let zeroes = bump.alloc_zeroed_slice::<i32>(3);
    /// assert_eq!(*zeroes, [0; 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<'a, [T]>
    where
        T: FromZeros,
    {
        panic_on_error(self.generic_alloc_zeroed_slice(len))
    }

    /// Allocate a zeroed object slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let zeroes = bump.try_alloc_zeroed_slice::<i32>(3)?;
    /// assert_eq!(*zeroes, [0; 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, AllocError>
    where
        T: FromZeros,
    {
        self.generic_alloc_zeroed_slice(len)
    }
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
