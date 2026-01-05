use core::mem::MaybeUninit;

use ::bytemuck::Zeroable;

use crate::{BaseAllocator, Bump, BumpBox, BumpScope, ErrorBehavior, alloc::AllocError, settings::BumpAllocatorSettings};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

mod vec_ext;

pub use vec_ext::VecExt;

mod init_zeroed {
    use super::*;

    pub trait Sealed {}

    impl<T: Zeroable> Sealed for BumpBox<'_, MaybeUninit<T>> {}
    impl<T: Zeroable> Sealed for BumpBox<'_, [MaybeUninit<T>]> {}
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
    /// let bump: Bump = Bump::new();
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

impl<'a, T: Zeroable> InitZeroed<'a> for BumpBox<'a, MaybeUninit<T>> {
    type Output = T;

    #[inline]
    fn init_zeroed(mut self) -> BumpBox<'a, T> {
        unsafe {
            self.as_mut_ptr().write_bytes(0, 1);
            self.assume_init()
        }
    }
}

impl<'a, T: Zeroable> InitZeroed<'a> for BumpBox<'a, [MaybeUninit<T>]> {
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
    use super::*;

    pub trait Sealed {}

    impl<A, S> Sealed for Bump<A, S>
    where
        A: BaseAllocator<S::GuaranteedAllocated>,
        S: BumpAllocatorSettings,
    {
    }
}

mod bump_scope_ext {
    use super::*;

    pub trait Sealed {}

    impl<A, S> Sealed for BumpScope<'_, A, S>
    where
        A: BaseAllocator<S::GuaranteedAllocated>,
        S: BumpAllocatorSettings,
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
    fn alloc_zeroed<T>(&self) -> BumpBox<'_, T>
    where
        T: Zeroable;

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
    fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<'_, T>, AllocError>
    where
        T: Zeroable;

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
    fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<'_, [T]>
    where
        T: Zeroable;

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
    fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<'_, [T]>, AllocError>
    where
        T: Zeroable;
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
        T: Zeroable;

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
        T: Zeroable;

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
        T: Zeroable;

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
        T: Zeroable;
}

impl<A, S> BumpExt for Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed<T>(&self) -> BumpBox<'_, T>
    where
        T: Zeroable,
    {
        self.as_scope().alloc_zeroed()
    }

    #[inline(always)]
    fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<'_, T>, AllocError>
    where
        T: Zeroable,
    {
        self.as_scope().try_alloc_zeroed()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<'_, [T]>
    where
        T: Zeroable,
    {
        self.as_scope().alloc_zeroed_slice(len)
    }

    #[inline(always)]
    fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<'_, [T]>, AllocError>
    where
        T: Zeroable,
    {
        self.as_scope().try_alloc_zeroed_slice(len)
    }
}

impl<'a, A, S> BumpScopeExt<'a> for BumpScope<'a, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed<T>(&self) -> BumpBox<'a, T>
    where
        T: Zeroable,
    {
        panic_on_error(self.generic_alloc_zeroed())
    }

    #[inline(always)]
    fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<'a, T>, AllocError>
    where
        T: Zeroable,
    {
        self.generic_alloc_zeroed()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<'a, [T]>
    where
        T: Zeroable,
    {
        panic_on_error(self.generic_alloc_zeroed_slice(len))
    }

    #[inline(always)]
    fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, AllocError>
    where
        T: Zeroable,
    {
        self.generic_alloc_zeroed_slice(len)
    }
}

trait PrivateBumpScopeExt<'a> {
    fn generic_alloc_zeroed<B: ErrorBehavior, T>(&self) -> Result<BumpBox<'a, T>, B>
    where
        T: Zeroable;

    fn generic_alloc_zeroed_slice<B: ErrorBehavior, T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, B>
    where
        T: Zeroable;
}

impl<'a, A, S> PrivateBumpScopeExt<'a> for BumpScope<'a, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn generic_alloc_zeroed<B: ErrorBehavior, T>(&self) -> Result<BumpBox<'a, T>, B>
    where
        T: Zeroable,
    {
        Ok(self.generic_alloc_uninit::<B, T>()?.init_zeroed())
    }

    #[inline(always)]
    fn generic_alloc_zeroed_slice<B: ErrorBehavior, T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, B>
    where
        T: Zeroable,
    {
        Ok(self.generic_alloc_uninit_slice::<B, T>(len)?.init_zeroed())
    }
}
