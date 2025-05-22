use core::mem::MaybeUninit;

use ::bytemuck::Zeroable as FromZeros;

use crate::{
    alloc::AllocError, BaseAllocator, Bump, BumpBox, BumpScope, ErrorBehavior, MinimumAlignment, SupportedMinimumAlignment,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

mod vec_ext;

pub use vec_ext::VecExt;

mod init_zeroed {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub trait Sealed {}

    impl<T: FromZeros> Sealed for BumpBox<'_, MaybeUninit<T>> {}
    impl<T: FromZeros> Sealed for BumpBox<'_, [MaybeUninit<T>]> {}
}

/// Extension trait for [`BumpBox`] that adds the `init_zeroed` method.
pub trait InitZeroed<'a>: init_zeroed::Sealed {
    /// The initialized type.
    type Output: ?Sized;

    /// Initializes `self` by filling it with zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, bytemuck::InitZeroed};
    /// let mut bump: Bump = Bump::new();
    ///
    /// // single value
    /// let uninit = bump.alloc_uninit::<i32>();
    /// let init = uninit.init_zeroed();
    /// assert_eq!(*init, 0);
    ///
    /// // slice
    /// let uninit = bump.alloc_uninit_slice::<i32>(10);
    /// let init = uninit.init_zeroed();
    /// assert_eq!(*init, [0; 10]);
    /// ```
    #[must_use]
    fn init_zeroed(self) -> BumpBox<'a, Self::Output>;
}

impl<'a, T: FromZeros> InitZeroed<'a> for BumpBox<'a, MaybeUninit<T>> {
    type Output = T;

    #[inline]
    fn init_zeroed(mut self) -> BumpBox<'a, T> {
        unsafe {
            self.as_mut_ptr().write_bytes(0, 1);
            self.assume_init()
        }
    }
}

impl<'a, T: FromZeros> InitZeroed<'a> for BumpBox<'a, [MaybeUninit<T>]> {
    type Output = [T];

    #[inline]
    fn init_zeroed(mut self) -> BumpBox<'a, [T]> {
        unsafe {
            let len = self.len();
            self.as_mut_ptr().write_bytes(0, len);
            self.assume_init()
        }
    }
}

mod bump_ext {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub trait Sealed {}

    impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Sealed
        for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        A: BaseAllocator<GUARANTEED_ALLOCATED>,
    {
    }
}

mod bump_scope_ext {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub trait Sealed {}

    impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Sealed
        for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        A: BaseAllocator<GUARANTEED_ALLOCATED>,
    {
    }
}

/// Extension trait for [`Bump`] that adds the `(try_)alloc_zeroed(_slice)` methods.
pub trait BumpExt: bump_ext::Sealed {
    /// Allocate a zeroed object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bytemuck::BumpExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let zero = bump.alloc_zeroed::<i32>();
    /// assert_eq!(*zero, 0);
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed<T>(&self) -> BumpBox<T>
    where
        T: FromZeros;

    /// Allocate a zeroed object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bytemuck::BumpExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let zero = bump.try_alloc_zeroed::<i32>()?;
    /// assert_eq!(*zero, 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<T>, AllocError>
    where
        T: FromZeros;

    /// Allocate a zeroed object slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bytemuck::BumpExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let zeroes = bump.alloc_zeroed_slice::<i32>(3);
    /// assert_eq!(*zeroes, [0; 3]);
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<[T]>
    where
        T: FromZeros;

    /// Allocate a zeroed object slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bytemuck::BumpExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let zeroes = bump.try_alloc_zeroed_slice::<i32>(3)?;
    /// assert_eq!(*zeroes, [0; 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<[T]>, AllocError>
    where
        T: FromZeros;
}

/// Extension trait for [`BumpScope`] that adds the `(try_)alloc_zeroed(_slice)` methods.
pub trait BumpScopeExt<'a>: bump_scope_ext::Sealed {
    /// Allocate a zeroed object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bytemuck::BumpScopeExt};
    /// let mut bump: Bump = Bump::new();
    ///
    /// bump.scoped(|bump| {
    ///     let zero = bump.alloc_zeroed::<i32>();
    ///     assert_eq!(*zero, 0);
    /// });
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed<T>(&self) -> BumpBox<'a, T>
    where
        T: FromZeros;

    /// Allocate a zeroed object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, alloc::AllocError, bytemuck::BumpScopeExt};
    /// let mut bump: Bump = Bump::try_new()?;
    ///
    /// bump.scoped(|bump| -> Result<(), AllocError> {
    ///     let zero = bump.try_alloc_zeroed::<i32>()?;
    ///     assert_eq!(*zero, 0);
    ///     Ok(())
    /// })?;
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<'a, T>, AllocError>
    where
        T: FromZeros;

    /// Allocate a zeroed object slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bytemuck::BumpScopeExt};
    /// let mut bump: Bump = Bump::new();
    ///
    /// bump.scoped(|bump| {
    ///     let zeroes = bump.alloc_zeroed_slice::<i32>(3);
    ///     assert_eq!(*zeroes, [0; 3]);
    /// });
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<'a, [T]>
    where
        T: FromZeros;

    /// Allocate a zeroed object slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, alloc::AllocError, bytemuck::BumpScopeExt};
    /// let mut bump: Bump = Bump::try_new()?;
    ///
    /// bump.scoped(|bump| -> Result<(), AllocError>  {
    ///     let zeroes = bump.try_alloc_zeroed_slice::<i32>(3)?;
    ///     assert_eq!(*zeroes, [0; 3]);
    ///     Ok(())
    /// })?;
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, AllocError>
    where
        T: FromZeros;
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpExt
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed<T>(&self) -> BumpBox<T>
    where
        T: FromZeros,
    {
        self.as_scope().alloc_zeroed()
    }

    #[inline(always)]
    fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<T>, AllocError>
    where
        T: FromZeros,
    {
        self.as_scope().try_alloc_zeroed()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<[T]>
    where
        T: FromZeros,
    {
        self.as_scope().alloc_zeroed_slice(len)
    }

    #[inline(always)]
    fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<[T]>, AllocError>
    where
        T: FromZeros,
    {
        self.as_scope().try_alloc_zeroed_slice(len)
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpScopeExt<'a>
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed<T>(&self) -> BumpBox<'a, T>
    where
        T: FromZeros,
    {
        panic_on_error(self.generic_alloc_zeroed())
    }

    #[inline(always)]
    fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<'a, T>, AllocError>
    where
        T: FromZeros,
    {
        self.generic_alloc_zeroed()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<'a, [T]>
    where
        T: FromZeros,
    {
        panic_on_error(self.generic_alloc_zeroed_slice(len))
    }

    #[inline(always)]
    fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, AllocError>
    where
        T: FromZeros,
    {
        self.generic_alloc_zeroed_slice(len)
    }
}

trait PrivateBumpScopeExt<'a> {
    fn generic_alloc_zeroed<B: ErrorBehavior, T>(&self) -> Result<BumpBox<'a, T>, B>
    where
        T: FromZeros;

    fn generic_alloc_zeroed_slice<B: ErrorBehavior, T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, B>
    where
        T: FromZeros;
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> PrivateBumpScopeExt<'a>
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn generic_alloc_zeroed<B: ErrorBehavior, T>(&self) -> Result<BumpBox<'a, T>, B>
    where
        T: FromZeros,
    {
        Ok(self.generic_alloc_uninit::<B, T>()?.init_zeroed())
    }

    #[inline(always)]
    fn generic_alloc_zeroed_slice<B: ErrorBehavior, T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, B>
    where
        T: FromZeros,
    {
        Ok(self.generic_alloc_uninit_slice::<B, T>(len)?.init_zeroed())
    }
}
