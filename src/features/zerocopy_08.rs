use core::mem::MaybeUninit;

use zerocopy_08::FromZeros;

use crate::{BumpBox, alloc::AllocError, traits::BumpAllocatorTypedScope};

mod vec_ext;

pub use vec_ext::VecExt;

mod init_zeroed {
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
    /// use bump_scope::{Bump, zerocopy_08::InitZeroed};
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

/// Extension trait for [`BumpAllocatorTypedScope`] that adds the `(try_)alloc_zeroed(_slice)` methods.
pub trait BumpAllocatorTypedScopeExt<'a>: BumpAllocatorTypedScope<'a> {
    /// Allocate a zeroed object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, zerocopy_08::BumpAllocatorTypedScopeExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let zero = bump.as_scope().alloc_zeroed::<i32>();
    /// assert_eq!(*zero, 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed<T>(&self) -> BumpBox<'a, T>
    where
        T: FromZeros,
    {
        self.alloc_uninit().init_zeroed()
    }

    /// Allocate a zeroed object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, zerocopy_08::BumpAllocatorTypedScopeExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let zero = bump.as_scope().try_alloc_zeroed::<i32>()?;
    /// assert_eq!(*zero, 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<'a, T>, AllocError>
    where
        T: FromZeros,
    {
        Ok(self.try_alloc_uninit()?.init_zeroed())
    }

    /// Allocate a zeroed object slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, zerocopy_08::BumpAllocatorTypedScopeExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let zeroes = bump.as_scope().alloc_zeroed_slice::<i32>(3);
    /// assert_eq!(*zeroes, [0; 3]);
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<'a, [T]>
    where
        T: FromZeros,
    {
        self.alloc_uninit_slice(len).init_zeroed()
    }

    /// Allocate a zeroed object slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, zerocopy_08::BumpAllocatorTypedScopeExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let zeroes = bump.as_scope().try_alloc_zeroed_slice::<i32>(3)?;
    /// assert_eq!(*zeroes, [0; 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, AllocError>
    where
        T: FromZeros,
    {
        Ok(self.try_alloc_uninit_slice(len)?.init_zeroed())
    }
}

impl<'a, T> BumpAllocatorTypedScopeExt<'a> for T where T: BumpAllocatorTypedScope<'a> {}
