macro_rules! bytemuck_or_zerocopy {
    (
        mod $mod:ident
        trait $trait:ident
    ) => {
        /// Contains extension traits.
        pub mod $mod {
            use core::mem::MaybeUninit;

            use ::$mod::$trait;

            use crate::{
                BumpBox, BumpVec, ErrorBehavior, FixedBumpVec, MutBumpVec, MutBumpVecRev,
                alloc::AllocError,
                traits::{BumpAllocatorTyped, BumpAllocatorTypedScope, MutBumpAllocatorTyped},
            };

            #[cfg(feature = "panic-on-alloc")]
            use crate::panic_on_error;

            mod init_zeroed {
                use super::*;

                pub trait Sealed {}

                impl<T: $trait> Sealed for BumpBox<'_, MaybeUninit<T>> {}
                impl<T: $trait> Sealed for BumpBox<'_, [MaybeUninit<T>]> {}
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
                #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::InitZeroed};")]
                ///
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

            impl<'a, T: $trait> InitZeroed<'a> for BumpBox<'a, MaybeUninit<T>> {
                type Output = T;

                #[inline]
                fn init_zeroed(mut self) -> BumpBox<'a, T> {
                    unsafe {
                        self.as_mut_ptr().write_bytes(0, 1);
                        self.assume_init()
                    }
                }
            }

            impl<'a, T: $trait> InitZeroed<'a> for BumpBox<'a, [MaybeUninit<T>]> {
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
                #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::BumpAllocatorTypedScopeExt};")]
                /// let bump: Bump = Bump::new();
                ///
                /// let zero = bump.as_scope().alloc_zeroed::<i32>();
                /// assert_eq!(*zero, 0);
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn alloc_zeroed<T>(&self) -> BumpBox<'a, T>
                where
                    T: $trait,
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
                #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::BumpAllocatorTypedScopeExt};")]
                /// let bump: Bump = Bump::try_new()?;
                ///
                /// let zero = bump.as_scope().try_alloc_zeroed::<i32>()?;
                /// assert_eq!(*zero, 0);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_alloc_zeroed<T>(&self) -> Result<BumpBox<'a, T>, AllocError>
                where
                    T: $trait,
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
                #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::BumpAllocatorTypedScopeExt};")]
                /// let bump: Bump = Bump::new();
                ///
                /// let zeroes = bump.as_scope().alloc_zeroed_slice::<i32>(3);
                /// assert_eq!(*zeroes, [0; 3]);
                /// ```
                #[cfg(feature = "panic-on-alloc")]
                fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpBox<'a, [T]>
                where
                    T: $trait,
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
                #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::BumpAllocatorTypedScopeExt};")]
                /// let bump: Bump = Bump::try_new()?;
                ///
                /// let zeroes = bump.as_scope().try_alloc_zeroed_slice::<i32>(3)?;
                /// assert_eq!(*zeroes, [0; 3]);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpBox<'a, [T]>, AllocError>
                where
                    T: $trait,
                {
                    Ok(self.try_alloc_uninit_slice(len)?.init_zeroed())
                }
            }

            impl<'a, T> BumpAllocatorTypedScopeExt<'a> for T where T: BumpAllocatorTypedScope<'a> {}

            mod vec_ext {
                use super::*;

                pub trait Sealed {}

                impl<T> Sealed for FixedBumpVec<'_, T> {}
                impl<T, A: BumpAllocatorTyped> Sealed for BumpVec<T, A> {}
                impl<T, A: MutBumpAllocatorTyped> Sealed for MutBumpVec<T, A> {}
                impl<T, A: MutBumpAllocatorTyped> Sealed for MutBumpVecRev<T, A> {}
            }

            /// Extension trait for this crate's vector types.
            pub trait VecExt: vec_ext::Sealed {
                /// The element type of this vector.
                type T;

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::new();
                ///
                /// let mut vec = bump_vec![in &bump; 1, 2, 3];
                /// vec.extend_zeroed(2);
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// ```
                #[cfg(feature = "panic-on-alloc")]
                fn extend_zeroed(&mut self, additional: usize)
                where
                    Self::T: $trait;

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::try_new()?;
                ///
                /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
                /// vec.try_extend_zeroed(2)?;
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
                where
                    Self::T: $trait;

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::new();
                ///
                /// let mut vec = bump_vec![in &bump; 1, 2, 3];
                /// vec.resize_zeroed(5);
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                ///
                /// let mut vec = bump_vec![in &bump; 1, 2, 3];
                /// vec.resize_zeroed(2);
                /// assert_eq!(vec, [1, 2]);
                /// ```
                #[cfg(feature = "panic-on-alloc")]
                fn resize_zeroed(&mut self, new_len: usize)
                where
                    Self::T: $trait;

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::try_new()?;
                ///
                /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
                /// vec.try_resize_zeroed(5)?;
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                ///
                /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
                /// vec.try_resize_zeroed(2)?;
                /// assert_eq!(vec, [1, 2]);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
                where
                    Self::T: $trait;
            }

            impl<T> VecExt for FixedBumpVec<'_, T> {
                type T = T;

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Panics
                /// Panics if the vector does not have enough capacity.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, FixedBumpVec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::new();
                ///
                /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
                /// vec.extend_from_slice_copy(&[1, 2, 3]);
                /// vec.extend_zeroed(2);
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn extend_zeroed(&mut self, additional: usize)
                where
                    Self::T: $trait,
                {
                    panic_on_error(self.generic_extend_zeroed(additional));
                }

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Errors
                /// Errors if the vector does not have enough capacity.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, FixedBumpVec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::try_new()?;
                ///
                /// let mut vec = FixedBumpVec::try_with_capacity_in(5, &bump)?;
                /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
                /// vec.try_extend_zeroed(2)?;
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
                where
                    Self::T: $trait,
                {
                    self.generic_extend_zeroed(additional)
                }

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Panics
                /// Panics if the vector does not have enough capacity.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, FixedBumpVec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::new();
                ///
                /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
                /// vec.extend_from_slice_copy(&[1, 2, 3]);
                /// vec.resize_zeroed(5);
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                ///
                /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
                /// vec.extend_from_slice_copy(&[1, 2, 3]);
                /// vec.resize_zeroed(2);
                /// assert_eq!(vec, [1, 2]);
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn resize_zeroed(&mut self, new_len: usize)
                where
                    T: $trait,
                {
                    panic_on_error(self.generic_resize_zeroed(new_len));
                }

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Errors
                /// Errors if the vector does not have enough capacity.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, FixedBumpVec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::try_new()?;
                ///
                /// let mut vec = FixedBumpVec::try_with_capacity_in(5, &bump)?;
                /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
                /// vec.try_resize_zeroed(5)?;
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                ///
                /// let mut vec = FixedBumpVec::try_with_capacity_in(5, &bump)?;
                /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
                /// vec.try_resize_zeroed(2)?;
                /// assert_eq!(vec, [1, 2]);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
                where
                    Self::T: $trait,
                {
                    self.generic_resize_zeroed(new_len)
                }
            }

            impl<T, A: BumpAllocatorTyped> VecExt for BumpVec<T, A> {
                type T = T;

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::new();
                ///
                /// let mut vec = bump_vec![in &bump; 1, 2, 3];
                /// vec.extend_zeroed(2);
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn extend_zeroed(&mut self, additional: usize)
                where
                    Self::T: $trait,
                {
                    panic_on_error(self.generic_extend_zeroed(additional));
                }

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::try_new()?;
                ///
                /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
                /// vec.try_extend_zeroed(2)?;
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
                where
                    Self::T: $trait,
                {
                    self.generic_extend_zeroed(additional)
                }

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::new();
                ///
                /// let mut vec = bump_vec![in &bump; 1, 2, 3];
                /// vec.resize_zeroed(5);
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                ///
                /// let mut vec = bump_vec![in &bump; 1, 2, 3];
                /// vec.resize_zeroed(2);
                /// assert_eq!(vec, [1, 2]);
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn resize_zeroed(&mut self, new_len: usize)
                where
                    Self::T: $trait,
                {
                    panic_on_error(self.generic_resize_zeroed(new_len));
                }

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let bump: Bump = Bump::try_new()?;
                ///
                /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
                /// vec.try_resize_zeroed(5)?;
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                ///
                /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
                /// vec.try_resize_zeroed(2)?;
                /// assert_eq!(vec, [1, 2]);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
                where
                    T: $trait,
                {
                    self.generic_resize_zeroed(new_len)
                }
            }

            impl<T, A: MutBumpAllocatorTyped> VecExt for MutBumpVec<T, A> {
                type T = T;

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, mut_bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let mut bump: Bump = Bump::new();
                ///
                /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
                /// vec.extend_zeroed(2);
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn extend_zeroed(&mut self, additional: usize)
                where
                    Self::T: $trait,
                {
                    panic_on_error(self.generic_extend_zeroed(additional));
                }

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, mut_bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let mut bump: Bump = Bump::try_new()?;
                ///
                /// let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
                /// vec.try_extend_zeroed(2)?;
                /// assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
                where
                    Self::T: $trait,
                {
                    self.generic_extend_zeroed(additional)
                }

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, mut_bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let mut bump: Bump = Bump::new();
                ///
                /// {
                ///     let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
                ///     vec.resize_zeroed(5);
                ///     assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// }
                ///
                /// {
                ///    let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
                ///    vec.resize_zeroed(2);
                ///    assert_eq!(vec, [1, 2]);
                /// }
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn resize_zeroed(&mut self, new_len: usize)
                where
                    Self::T: $trait,
                {
                    panic_on_error(self.generic_resize_zeroed(new_len));
                }

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, mut_bump_vec, ", stringify!($mod), "::VecExt};")]
                ///
                /// let mut bump: Bump = Bump::try_new()?;
                ///
                /// {
                ///     let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
                ///     vec.try_resize_zeroed(5)?;
                ///     assert_eq!(vec, [1, 2, 3, 0, 0]);
                /// }
                ///
                /// {
                ///    let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
                ///    vec.try_resize_zeroed(2)?;
                ///    assert_eq!(vec, [1, 2]);
                /// }
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
                where
                    Self::T: $trait,
                {
                    self.generic_resize_zeroed(new_len)
                }
            }

            impl<T, A: MutBumpAllocatorTyped> VecExt for MutBumpVecRev<T, A> {
                type T = T;

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, mut_bump_vec_rev, ", stringify!($mod), "::VecExt};")]
                ///
                /// let mut bump: Bump = Bump::new();
                ///
                /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
                /// vec.extend_zeroed(2);
                /// assert_eq!(vec, [0, 0, 1, 2, 3]);
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn extend_zeroed(&mut self, additional: usize)
                where
                    T: $trait,
                {
                    panic_on_error(self.generic_extend_zeroed(additional));
                }

                /// Extends this vector by pushing `additional` new items onto the end.
                /// The new items are initialized with zeroes.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, mut_bump_vec_rev, ", stringify!($mod), "::VecExt};")]
                ///
                /// let mut bump: Bump = Bump::try_new()?;
                ///
                /// let mut vec = mut_bump_vec_rev![try in &mut bump; 1, 2, 3]?;
                /// vec.try_extend_zeroed(2)?;
                /// assert_eq!(vec, [0, 0, 1, 2, 3]);
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
                where
                    T: $trait,
                {
                    self.generic_extend_zeroed(additional)
                }

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, mut_bump_vec_rev, ", stringify!($mod), "::VecExt};")]
                ///
                /// let mut bump: Bump = Bump::new();
                ///
                /// {
                ///     let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
                ///     vec.resize_zeroed(5);
                ///     assert_eq!(vec, [0, 0, 1, 2, 3]);
                /// }
                ///
                /// {
                ///     let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
                ///     vec.resize_zeroed(2);
                ///     assert_eq!(vec, [2, 3]);
                /// }
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn resize_zeroed(&mut self, new_len: usize)
                where
                    T: $trait,
                {
                    panic_on_error(self.generic_resize_zeroed(new_len));
                }

                /// Resizes this vector in-place so that `len` is equal to `new_len`.
                ///
                /// If `new_len` is greater than `len`, the vector is extended by the
                /// difference, with each additional slot filled with `value`.
                /// If `new_len` is less than `len`, the vector is simply truncated.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, mut_bump_vec_rev, ", stringify!($mod), "::VecExt};")]
                ///
                /// let mut bump: Bump = Bump::try_new()?;
                ///
                /// {
                ///     let mut vec = mut_bump_vec_rev![try in &mut bump; 1, 2, 3]?;
                ///     vec.try_resize_zeroed(5)?;
                ///     assert_eq!(vec, [0, 0, 1, 2, 3]);
                /// }
                ///
                /// {
                ///     let mut vec = mut_bump_vec_rev![try in &mut bump; 1, 2, 3]?;
                ///     vec.try_resize_zeroed(2)?;
                ///     assert_eq!(vec, [2, 3]);
                /// }
                /// # Ok::<(), bump_scope::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
                where
                    T: $trait,
                {
                    self.generic_resize_zeroed(new_len)
                }
            }

            trait PrivateVecExt {
                fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E>;
                fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E>;
            }

            impl<T: $trait> PrivateVecExt for FixedBumpVec<'_, T> {
                #[inline]
                fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
                    self.generic_reserve(additional)?;

                    unsafe {
                        let ptr = self.as_mut_ptr();
                        let len = self.len();

                        ptr.add(len).write_bytes(0, additional);
                        self.set_len(len + additional);
                    }

                    Ok(())
                }

                #[inline]
                fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E> {
                    let len = self.len();

                    if new_len > len {
                        self.generic_extend_zeroed(new_len - len)
                    } else {
                        self.truncate(new_len);
                        Ok(())
                    }
                }
            }

            impl<T: $trait, A: BumpAllocatorTyped> PrivateVecExt for BumpVec<T, A> {
                #[inline]
                fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
                    self.generic_reserve(additional)?;

                    unsafe {
                        let ptr = self.as_mut_ptr();
                        let len = self.len();

                        ptr.add(len).write_bytes(0, additional);
                        self.set_len(len + additional);
                    }

                    Ok(())
                }

                #[inline]
                fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E> {
                    let len = self.len();

                    if new_len > len {
                        self.generic_extend_zeroed(new_len - len)
                    } else {
                        self.truncate(new_len);
                        Ok(())
                    }
                }
            }

            impl<T: $trait, A: MutBumpAllocatorTyped> PrivateVecExt for MutBumpVec<T, A> {
                #[inline]
                fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E>
                where
                    T: $trait,
                {
                    self.generic_reserve(additional)?;

                    unsafe {
                        let ptr = self.as_mut_ptr();
                        let len = self.len();

                        ptr.add(len).write_bytes(0, additional);
                        self.set_len(len + additional);
                    }

                    Ok(())
                }

                #[inline]
                fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E>
                where
                    T: $trait,
                {
                    let len = self.len();

                    if new_len > len {
                        self.generic_extend_zeroed(new_len - len)
                    } else {
                        self.truncate(new_len);
                        Ok(())
                    }
                }
            }

            impl<T: $trait, A: MutBumpAllocatorTyped> PrivateVecExt for MutBumpVecRev<T, A> {
                #[inline]
                fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
                    self.generic_reserve(additional)?;

                    unsafe {
                        let new_len = self.len() + additional;
                        self.end.sub(new_len).write_bytes(0, additional);
                        self.set_len(new_len);
                    }

                    Ok(())
                }

                #[inline]
                fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E> {
                    let len = self.len();

                    if new_len > len {
                        self.generic_extend_zeroed(new_len - len)
                    } else {
                        self.truncate(new_len);
                        Ok(())
                    }
                }
            }
        }
    };
}

pub(crate) use bytemuck_or_zerocopy;
