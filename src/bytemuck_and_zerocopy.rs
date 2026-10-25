macro_rules! make {
    (
        crate $name:literal
        mod $mod:ident
        trait $trait:ident
    ) => {
        #[doc = concat!("Extension traits for zero-initializable types using [`", $name, "::", stringify!($trait), "`](::", stringify!($mod), "::", stringify!($trait), ").")]
        pub mod $mod {
            use core::{
                alloc::{AllocError, Allocator},
                mem::MaybeUninit,
                ptr,
            };

            use alloc::boxed::Box;

            use ::$mod::$trait;

            use crate::{
                BumpAllocation,
                traits::{BumpAllocator},
            };

            /// Initializes a boxed `MaybeUninit` by filling it with zeroes.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::box_zero_init};")]
            ///
            /// let bump: Bump = Bump::new();
            ///
            /// let uninit = bump.alloc_uninit::<i32>().into_boxu();
            /// let init = box_zero_init(uninit);
            /// assert_eq!(*init, 0);
            /// ```
            #[must_use]
            pub fn box_zero_init<T, A>(mut uninit: Box<MaybeUninit<T>, A>) -> Box<T, A>
            where
                T: $trait,
                A: Allocator,
            {
                unsafe {
                    uninit.as_mut_ptr().write_bytes(0, 1);
                    Box::<MaybeUninit<T>, A>::assume_init(uninit)
                }
            }

            /// Initializes a slice of boxed `MaybeUninit` by filling it with zeroes.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::box_zero_init_slice};")]
            ///
            /// let bump: Bump = Bump::new();
            ///
            /// let uninit = bump.alloc_uninit_slice::<i32>(10).into_boxu();
            /// let init = box_zero_init_slice(uninit);
            /// assert_eq!(*init, [0; 10]);
            /// ```
            #[must_use]
            pub fn box_zero_init_slice<T, A>(uninit: Box<[MaybeUninit<T>], A>) -> Box<[T], A>
            where
                T: $trait,
                A: Allocator,
            {
                let (ptr, alloc) = Box::into_raw_with_allocator(uninit);
                let len = ptr.len();
                let ptr = ptr.cast::<T>();

                unsafe {
                    ptr.write_bytes(0, len);
                    Box::from_raw_in(ptr::slice_from_raw_parts_mut(ptr, len), alloc)
                }
            }

            /// Extension trait for [`BumpAllocator`] that adds the `(try_)alloc_zeroed(_slice)` methods.
            pub trait BumpAllocatorExt: BumpAllocator {
                /// Allocate a zeroed object.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::BumpAllocatorScopeExt};")]
                /// let bump: Bump = Bump::new();
                ///
                /// let zero = bump.as_scope().alloc_zeroed::<i32>();
                /// assert_eq!(*zero, 0);
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn alloc_zeroed<T>(&self) -> BumpAllocation<T, &Self>
                where
                    T: $trait,
                {
                    BumpAllocation::new(box_zero_init(self.alloc_uninit::<T>().into_box()))
                }

                /// Allocate a zeroed object.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::BumpAllocatorExt};")]
                /// let bump: Bump = Bump::new();
                ///
                /// let zero = bump.as_scope().try_alloc_zeroed::<i32>()?;
                /// assert_eq!(*zero, 0);
                /// # Ok::<(), ::core::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_alloc_zeroed<T>(&self) -> Result<BumpAllocation<T, &Self>, AllocError>
                where
                    T: $trait,
                {
                    Ok(BumpAllocation::new(box_zero_init(self.try_alloc_uninit::<T>()?.into_box())))
                }

                /// Allocate a zeroed object slice.
                ///
                /// # Panics
                /// Panics if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::BumpAllocatorExt};")]
                /// let bump: Bump = Bump::new();
                ///
                /// let zeroes = bump.as_scope().alloc_zeroed_slice::<i32>(3);
                /// assert_eq!(*zeroes, [0; 3]);
                /// ```
                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn alloc_zeroed_slice<T>(&self, len: usize) -> BumpAllocation<[T], &Self>
                where
                    T: $trait,
                {
                    BumpAllocation::new(box_zero_init_slice(self.alloc_uninit_slice::<T>(len).into_box()))
                }

                /// Allocate a zeroed object slice.
                ///
                /// # Errors
                /// Errors if the allocation fails.
                ///
                /// # Examples
                /// ```
                #[doc = concat!("use bump_scope::{Bump, ", stringify!($mod), "::BumpAllocatorExt};")]
                /// let bump: Bump = Bump::new();
                ///
                /// let zeroes = bump.as_scope().try_alloc_zeroed_slice::<i32>(3)?;
                /// assert_eq!(*zeroes, [0; 3]);
                /// # Ok::<(), ::core::alloc::AllocError>(())
                /// ```
                #[inline(always)]
                fn try_alloc_zeroed_slice<T>(&self, len: usize) -> Result<BumpAllocation<[T], &Self>, AllocError>
                where
                    T: $trait,
                {
                    Ok(BumpAllocation::new(box_zero_init_slice(self.try_alloc_uninit_slice::<T>(len)?.into_box())))
                }
            }

            impl<T: BumpAllocator> BumpAllocatorExt for T {}
        }
    };
}

#[cfg(feature = "bytemuck")]
make! {
    crate "bytemuck"
    mod bytemuck
    trait Zeroable
}

#[cfg(feature = "zerocopy-08")]
make! {
    crate "zerocopy"
    mod zerocopy_08
    trait FromZeros
}
