#![expect(clippy::missing_safety_doc)]

use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

use crate::{
    Bump, BumpBox, BumpScope, SizedTypeProperties, WithoutDealloc, WithoutShrink,
    alloc::{AllocError, Allocator},
    bump_down,
    polyfill::non_null,
    settings::BumpAllocatorSettings,
    stats::{AnyStats, Stats},
    traits::{
        BumpAllocatorCore, BumpAllocatorCoreScope, MutBumpAllocatorCore, MutBumpAllocatorCoreScope, assert_implements,
    },
    up_align_usize_unchecked,
};

#[cfg(feature = "panic-on-alloc")]
use crate::{handle_alloc_error, panic_on_error, private::capacity_overflow};

/// An extension trait for [`BumpAllocatorCore`]s.
///
/// Its main purpose is to provide methods that are optimized for a certain `T` and error behavior.
///
/// It also provides [`typed_stats`] to get a `Bump` specific `Stats` object.
///
/// **Note:** This trait is not automatically implemented for all `BumpAllocatorCore`s
/// because it is meant to provide specialized methods and types for better performance.
/// A blanket implementation for all `BumpAllocatorCore`s would defeat that purpose, at least
/// until some form of specialization is stabilized.
///
/// [`typed_stats`]: BumpAllocatorTyped::typed_stats
pub unsafe trait BumpAllocatorTyped: BumpAllocatorCore {
    /// The type returned by the [stats](BumpAllocatorTyped::typed_stats) method.
    type TypedStats<'b>: Into<AnyStats<'b>>
    where
        Self: 'b;

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    fn typed_stats(&self) -> Self::TypedStats<'_>;

    /// A specialized version of [`allocate`](crate::alloc::Allocator::allocate).
    ///
    /// Behaves like the following code:
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self, layout: Layout) -> NonNull<u8> {
    /// self.allocate(layout).unwrap().cast()
    /// #     }
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8>;

    /// A specialized version of [`allocate`](crate::alloc::Allocator::allocate).
    ///
    /// Behaves like the following code:
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # use bump_scope::alloc::AllocError;
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
    /// Ok(self.allocate(layout)?.cast())
    /// #     }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError>;

    /// A specialized version of [`allocate`](crate::alloc::Allocator::allocate).
    ///
    /// Behaves like the following code:
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # type T = i32;
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self) -> NonNull<T> {
    /// self.allocate(Layout::new::<T>()).unwrap().cast()
    /// #     }
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T>;

    /// A specialized version of [`allocate`](crate::alloc::Allocator::allocate).
    ///
    /// Behaves like the following code:
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # use bump_scope::alloc::AllocError;
    /// # type T = i32;
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self) -> Result<NonNull<T>, AllocError> {
    /// Ok(self.allocate(Layout::new::<T>())?.cast())
    /// #     }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>;

    /// A specialized version of [`allocate`](crate::alloc::Allocator::allocate).
    ///
    /// Behaves like the following code:
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # type T = i32;
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self, len: usize) -> NonNull<T> {
    /// self.allocate(Layout::array::<T>(len).unwrap()).unwrap().cast()
    /// #     }
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>;

    /// A specialized version of [`allocate`](crate::alloc::Allocator::allocate).
    ///
    /// Behaves like the following code:
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # use bump_scope::alloc::AllocError;
    /// # type T = i32;
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self, len: usize) -> Result<NonNull<T>, AllocError> {
    /// Ok(self.allocate(Layout::array::<T>(len).map_err(|_| AllocError)?)?.cast())
    /// #     }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>;

    /// A specialized version of [`allocate`](crate::alloc::Allocator::allocate).
    ///
    /// Behaves like the following code:
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # type T = i32;
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self, slice: &[T]) -> NonNull<T> {
    /// self.allocate(Layout::for_value(slice)).unwrap().cast()
    /// #     }
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T>;

    /// A specialized version of [`allocate`](crate::alloc::Allocator::allocate).
    ///
    /// Behaves like the following code:
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # use bump_scope::alloc::AllocError;
    /// # type T = i32;
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
    /// Ok(self.allocate(Layout::for_value(slice))?.cast())
    /// #     }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError>;

    /// A specialized version of [`shrink`](crate::alloc::Allocator::shrink).
    ///
    /// Behaves like the following code except that it returns `None`
    /// when the allocation remains unchanged and the pointer stays valid.
    /// ```
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # type T = i32;
    /// # #[expect(dead_code)]
    /// # trait MyExt: bump_scope::traits::BumpAllocatorCore {
    /// #     unsafe fn my_ext_fn(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> NonNull<T> {
    /// #         unsafe {
    /// self.shrink(ptr.cast(),
    ///     Layout::array::<T>(old_len).unwrap_unchecked(),
    ///     Layout::array::<T>(new_len).unwrap_unchecked(),
    /// ).unwrap_unchecked().cast()
    /// #         }
    /// #     }
    /// # }
    /// ```
    ///
    /// # Safety
    ///
    /// Same safety conditions as for the code above apply.
    ///
    /// [shrink]: crate::alloc::Allocator::shrink
    /// [array]: Layout::array
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>>;

    /// A specialized version of [`prepare_allocation`].
    ///
    /// Returns a `[T]` of free space in the bump allocator.
    ///
    /// [`prepare_allocation`]: crate::traits::BumpAllocatorCore::prepare_allocation
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, cap: usize) -> NonNull<[T]>;

    /// A specialized version of [`prepare_allocation`].
    ///
    /// Returns a `[T]` of free space in the bump allocator.
    ///
    /// [`prepare_allocation`]: crate::traits::BumpAllocatorCore::prepare_allocation
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn try_prepare_slice_allocation<T>(&self, cap: usize) -> Result<NonNull<[T]>, AllocError>;

    /// A specialized version of [`allocate_prepared`].
    ///
    /// Allocates part of the free space returned from a
    /// <code>([try_](BumpAllocatorTyped::try_prepare_slice_allocation))[prepare_slice_allocation](BumpAllocatorTyped::prepare_slice_allocation)</code>
    /// call.
    ///
    /// # Safety
    /// - `ptr..ptr + cap` must be the pointer range returned from
    ///   <code>([try_](BumpAllocatorTyped::try_prepare_slice_allocation))[prepare_slice_allocation](BumpAllocatorTyped::prepare_slice_allocation)</code>.
    /// - no allocation, grow, shrink or deallocate must have taken place since then
    /// - no resets must have taken place since then
    /// - `len` must be less than or equal to `cap`
    ///
    /// [`allocate_prepared`]: BumpAllocatorCore::allocate_prepared
    unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>;

    /// A specialized version of [`allocate_prepared_rev`].
    ///
    /// Allocates part of the free space returned from a
    /// <code>([try_](BumpAllocatorTyped::try_prepare_slice_allocation))[prepare_slice_allocation](BumpAllocatorTyped::prepare_slice_allocation)</code>
    /// call.
    ///
    /// # Safety
    /// - `ptr - cap..ptr` must be the pointer range returned from
    ///   <code>([try_](BumpAllocatorTyped::try_prepare_slice_allocation))[prepare_slice_allocation](BumpAllocatorTyped::prepare_slice_allocation)</code>.
    /// - no allocation, grow, shrink or deallocate must have taken place since then
    /// - no resets must have taken place since then
    /// - `len` must be less than or equal to `cap`
    ///
    /// [`allocate_prepared_rev`]: crate::traits::BumpAllocatorCore::allocate_prepared_rev
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>;

    /// Drops an allocated value and attempts to free its memory.
    ///
    /// The memory can only be freed if this is the last allocation.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let boxed = bump.alloc(3i32);
    /// assert_eq!(bump.stats().allocated(), 4);
    /// bump.dealloc(boxed);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    fn dealloc<T: ?Sized>(&self, boxed: BumpBox<T>) {
        let layout = Layout::for_value::<T>(&boxed);
        let ptr = boxed.into_raw();

        unsafe { ptr.drop_in_place() };

        // zst and empty slices did not actually allocate
        // and thus mustn't deallocate
        if layout.size() == 0 {
            return;
        }

        unsafe { self.deallocate(ptr.cast(), layout) };
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve`, <code>self.[stats][]().[remaining][]()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that these additional bytes are not necessarily in one contiguous region but
    /// might be spread out among many chunks.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    ///
    /// [stats]: crate::traits::BumpAllocatorScope::stats
    /// [remaining]: Stats::remaining
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize);

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve`, <code>self.[stats][]().[remaining][]()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Panics
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.try_reserve(4096)?;
    /// assert!(bump.stats().capacity() >= 4096);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [stats]: crate::traits::BumpAllocatorScope::stats
    /// [remaining]: Stats::remaining
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError>;
}

assert_implements! {
    [BumpAllocatorTyped + ?Sized]

    dyn BumpAllocatorCore
    &dyn BumpAllocatorCore
    &mut dyn BumpAllocatorCore

    dyn BumpAllocatorCoreScope
    &dyn BumpAllocatorCoreScope
    &mut dyn BumpAllocatorCoreScope

    dyn MutBumpAllocatorCore
    &dyn MutBumpAllocatorCore
    &mut dyn MutBumpAllocatorCore

    dyn MutBumpAllocatorCoreScope
    &dyn MutBumpAllocatorCoreScope
    &mut dyn MutBumpAllocatorCoreScope
}

macro_rules! impl_for_trait_object {
    ($($ty:ty)*) => {
        $(
            unsafe impl BumpAllocatorTyped for $ty {
                type TypedStats<'b>
                    = AnyStats<'b>
                where
                    Self: 'b;

                #[inline(always)]
                fn typed_stats(&self) -> AnyStats<'_> {
                    self.any_stats()
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
                    for_trait_object::allocate_layout(self, layout)
                }

                #[inline(always)]
                fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
                    for_trait_object::try_allocate_layout(self, layout)
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn allocate_sized<T>(&self) -> NonNull<T> {
                    for_trait_object::allocate_sized(self)
                }

                #[inline(always)]
                fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
                    for_trait_object::try_allocate_sized(self)
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
                    for_trait_object::allocate_slice(self, len)
                }

                #[inline(always)]
                fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
                    for_trait_object::try_allocate_slice(self, len)
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
                    for_trait_object::allocate_slice_for(self, slice)
                }

                #[inline(always)]
                fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
                    for_trait_object::try_allocate_slice_for(self, slice)
                }

                #[inline(always)]
                unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
                    unsafe { for_trait_object::shrink_slice(self, ptr, old_len, new_len) }
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
                    for_trait_object::prepare_slice_allocation(self, len)
                }

                #[inline(always)]
                fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
                    for_trait_object::try_prepare_slice_allocation(self, len)
                }

                #[inline(always)]
                unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
                    unsafe { for_trait_object::allocate_prepared_slice(self, ptr, len, cap) }
                }

                #[inline(always)]
                unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
                    unsafe { for_trait_object::allocate_prepared_slice_rev(self, ptr, len, cap) }
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn reserve(&self, additional: usize) {
                    for_trait_object::reserve(self, additional);
                }

                #[inline(always)]
                fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
                    for_trait_object::try_reserve(self, additional)
                }
            }
        )*
    };
}

mod for_trait_object {
    use super::*;

    #[inline]
    #[cfg(feature = "panic-on-alloc")]
    pub(super) fn allocate_layout(bump: impl BumpAllocatorCore, layout: Layout) -> NonNull<u8> {
        match bump.allocate(layout) {
            Ok(ptr) => ptr.cast(),
            Err(AllocError) => handle_alloc_error(layout),
        }
    }

    #[inline]
    pub(super) fn try_allocate_layout(bump: impl BumpAllocatorCore, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        match bump.allocate(layout) {
            Ok(ptr) => Ok(ptr.cast()),
            Err(err) => Err(err),
        }
    }

    #[inline]
    #[cfg(feature = "panic-on-alloc")]
    pub(super) fn allocate_sized<T>(bump: impl BumpAllocatorCore) -> NonNull<T> {
        let layout = Layout::new::<T>();

        match bump.allocate(layout) {
            Ok(ptr) => ptr.cast(),
            Err(AllocError) => handle_alloc_error(Layout::new::<T>()),
        }
    }

    #[inline]
    pub(super) fn try_allocate_sized<T>(bump: impl BumpAllocatorCore) -> Result<NonNull<T>, AllocError> {
        match bump.allocate(Layout::new::<T>()) {
            Ok(ptr) => Ok(ptr.cast()),
            Err(err) => Err(err),
        }
    }

    #[inline]
    #[cfg(feature = "panic-on-alloc")]
    pub(super) fn allocate_slice<T>(bump: impl BumpAllocatorCore, len: usize) -> NonNull<T> {
        let Ok(layout) = Layout::array::<T>(len) else {
            invalid_slice_layout()
        };

        match bump.allocate(layout) {
            Ok(ptr) => ptr.cast(),
            Err(AllocError) => handle_alloc_error(layout),
        }
    }

    #[inline]
    pub(super) fn try_allocate_slice<T>(bump: impl BumpAllocatorCore, len: usize) -> Result<NonNull<T>, AllocError> {
        let Ok(layout) = Layout::array::<T>(len) else {
            return Err(AllocError);
        };

        match bump.allocate(layout) {
            Ok(ptr) => Ok(ptr.cast()),
            Err(err) => Err(err),
        }
    }

    #[inline]
    #[cfg(feature = "panic-on-alloc")]
    pub(super) fn allocate_slice_for<T>(bump: impl BumpAllocatorCore, slice: &[T]) -> NonNull<T> {
        let layout = Layout::for_value(slice);

        match bump.allocate(layout) {
            Ok(ptr) => ptr.cast(),
            Err(AllocError) => handle_alloc_error(layout),
        }
    }

    #[inline]
    pub(super) fn try_allocate_slice_for<T>(bump: impl BumpAllocatorCore, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        let layout = Layout::for_value(slice);

        match bump.allocate(layout) {
            Ok(ptr) => Ok(ptr.cast()),
            Err(err) => Err(err),
        }
    }

    #[inline]
    #[expect(clippy::unnecessary_wraps)]
    pub(super) unsafe fn shrink_slice<T>(
        bump: impl BumpAllocatorCore,
        ptr: NonNull<T>,
        old_len: usize,
        new_len: usize,
    ) -> Option<NonNull<T>> {
        unsafe {
            Some(
                bump.shrink(
                    ptr.cast(),
                    Layout::array::<T>(old_len).unwrap_unchecked(),
                    Layout::array::<T>(new_len).unwrap_unchecked(),
                )
                .unwrap_unchecked()
                .cast(),
            )
        }
    }

    fn is_upwards_allocating(bump: &impl BumpAllocatorCore) -> bool {
        let chunk = bump.checkpoint().chunk;
        let header = chunk.addr();
        let end = unsafe { chunk.as_ref() }.end.addr();
        end > header
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub(super) fn prepare_slice_allocation<T>(bump: impl BumpAllocatorCore, min_cap: usize) -> NonNull<[T]> {
        let Ok(layout) = Layout::array::<T>(min_cap) else {
            capacity_overflow()
        };

        match bump.prepare_allocation(layout) {
            Ok(range) => {
                // NB: We can't use `offset_from_unsigned`, because the size is not a multiple of `T`'s.
                let cap = unsafe { non_null::byte_offset_from_unsigned(range.end, range.start) } / T::SIZE;

                let ptr = if is_upwards_allocating(&bump) {
                    range.start.cast::<T>()
                } else {
                    unsafe { range.end.cast::<T>().sub(cap) }
                };

                NonNull::slice_from_raw_parts(ptr.cast(), cap)
            }
            Err(AllocError) => handle_alloc_error(layout),
        }
    }

    #[inline(always)]
    pub(super) fn try_prepare_slice_allocation<T>(
        bump: impl BumpAllocatorCore,
        len: usize,
    ) -> Result<NonNull<[T]>, AllocError> {
        let Ok(layout) = Layout::array::<T>(len) else {
            return Err(AllocError);
        };

        match bump.prepare_allocation(layout) {
            Ok(range) => {
                // NB: We can't use `offset_from_unsigned`, because the size is not a multiple of `T`'s.
                let cap = unsafe { non_null::byte_offset_from_unsigned(range.end, range.start) } / T::SIZE;

                let ptr = if is_upwards_allocating(&bump) {
                    range.start.cast::<T>()
                } else {
                    unsafe { range.end.cast::<T>().sub(cap) }
                };

                Ok(NonNull::slice_from_raw_parts(ptr.cast(), cap))
            }
            Err(err) => Err(err),
        }
    }

    #[inline(always)]
    pub(super) unsafe fn allocate_prepared_slice<T>(
        bump: impl BumpAllocatorCore,
        ptr: NonNull<T>,
        len: usize,
        cap: usize,
    ) -> NonNull<[T]> {
        unsafe {
            let range = non_null::cast_range(ptr..ptr.add(cap));
            let layout = Layout::from_size_align_unchecked(core::mem::size_of::<T>() * len, T::ALIGN);
            let data = bump.allocate_prepared(layout, range).cast();
            NonNull::slice_from_raw_parts(data, len)
        }
    }

    #[inline(always)]
    pub(super) unsafe fn allocate_prepared_slice_rev<T>(
        bump: impl BumpAllocatorCore,
        ptr: NonNull<T>,
        len: usize,
        cap: usize,
    ) -> NonNull<[T]> {
        unsafe {
            let range = non_null::cast_range(ptr.sub(cap)..ptr);
            let layout = Layout::from_size_align_unchecked(core::mem::size_of::<T>() * len, T::ALIGN);
            let data = bump.allocate_prepared_rev(layout, range).cast();
            NonNull::slice_from_raw_parts(data, len)
        }
    }

    #[cfg(feature = "panic-on-alloc")]
    pub(super) fn reserve(bump: impl BumpAllocatorCore, additional: usize) {
        let Ok(layout) = Layout::array::<u8>(additional) else {
            invalid_slice_layout();
        };

        if let Err(AllocError) = bump.prepare_allocation(layout) {
            handle_alloc_error(layout);
        }
    }

    pub(super) fn try_reserve(bump: impl BumpAllocatorCore, additional: usize) -> Result<(), AllocError> {
        let Ok(layout) = Layout::array::<u8>(additional) else {
            return Err(AllocError);
        };

        match bump.prepare_allocation(layout) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }
}

impl_for_trait_object! {
    dyn BumpAllocatorCore + '_
    dyn MutBumpAllocatorCore + '_
    dyn BumpAllocatorCoreScope<'_> + '_
    dyn MutBumpAllocatorCoreScope<'_> + '_
}

macro_rules! impl_for_ref {
    ($($ty:ty)*) => {
        $(
            unsafe impl<B: BumpAllocatorTyped + ?Sized> BumpAllocatorTyped for $ty {
                type TypedStats<'b>
                    = B::TypedStats<'b>
                where
                    Self: 'b;

                #[inline(always)]
                fn typed_stats(&self) -> Self::TypedStats<'_> {
                    B::typed_stats(self)
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
                    B::allocate_layout(self, layout)
                }

                #[inline(always)]
                fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
                    B::try_allocate_layout(self, layout)
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn allocate_sized<T>(&self) -> NonNull<T> {
                    B::allocate_sized(self)
                }

                #[inline(always)]
                fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
                    B::try_allocate_sized(self)
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
                    B::allocate_slice(self, len)
                }

                #[inline(always)]
                fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
                    B::try_allocate_slice(self, len)
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
                    B::allocate_slice_for(self, slice)
                }

                #[inline(always)]
                fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
                    B::try_allocate_slice_for(self, slice)
                }

                #[inline(always)]
                unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
                    unsafe { B::shrink_slice(self, ptr, old_len, new_len) }
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
                    B::prepare_slice_allocation(self, len)
                }

                #[inline(always)]
                fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
                    B::try_prepare_slice_allocation(self, len)
                }

                #[inline(always)]
                unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
                    unsafe { B::allocate_prepared_slice(self, ptr, len, cap) }
                }

                #[inline(always)]
                unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
                    unsafe { B::allocate_prepared_slice_rev(self, ptr, len, cap) }
                }

                #[inline(always)]
                #[cfg(feature = "panic-on-alloc")]
                fn reserve(&self, additional: usize) {
                    B::reserve(self, additional);
                }

                #[inline(always)]
                fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
                    B::try_reserve(self, additional)
                }
            }
        )*
    };
}

impl_for_ref! {
    &B
    &mut B
}

unsafe impl<B: BumpAllocatorTyped> BumpAllocatorTyped for WithoutDealloc<B> {
    type TypedStats<'b>
        = B::TypedStats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn typed_stats(&self) -> Self::TypedStats<'_> {
        B::typed_stats(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        B::allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        B::try_allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        B::allocate_sized(&self.0)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_sized(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        B::allocate_slice(&self.0, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_slice(&self.0, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
        B::allocate_slice_for(&self.0, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_slice_for(&self.0, slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { B::shrink_slice(&self.0, ptr, old_len, new_len) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
        B::prepare_slice_allocation(&self.0, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
        B::try_prepare_slice_allocation(&self.0, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { B::allocate_prepared_slice(&self.0, ptr, len, cap) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { B::allocate_prepared_slice_rev(&self.0, ptr, len, cap) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        B::reserve(&self.0, additional);
    }

    #[inline(always)]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        B::try_reserve(&self.0, additional)
    }
}

unsafe impl<B: BumpAllocatorTyped> BumpAllocatorTyped for WithoutShrink<B> {
    type TypedStats<'b>
        = B::TypedStats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn typed_stats(&self) -> Self::TypedStats<'_> {
        B::typed_stats(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        B::allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        B::try_allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        B::allocate_sized(&self.0)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_sized(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        B::allocate_slice(&self.0, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_slice(&self.0, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
        B::allocate_slice_for(&self.0, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_slice_for(&self.0, slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        // it's called `WithoutShrink` for a reason...
        _ = (ptr, old_len, new_len);
        None
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
        B::prepare_slice_allocation(&self.0, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
        B::try_prepare_slice_allocation(&self.0, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { B::allocate_prepared_slice(&self.0, ptr, len, cap) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { B::allocate_prepared_slice_rev(&self.0, ptr, len, cap) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        B::reserve(&self.0, additional);
    }

    #[inline(always)]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        B::try_reserve(&self.0, additional)
    }
}

unsafe impl<A, S> BumpAllocatorTyped for BumpScope<'_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    type TypedStats<'b>
        = Stats<'b, S>
    where
        Self: 'b;

    #[inline(always)]
    fn typed_stats(&self) -> Self::TypedStats<'_> {
        BumpScope::stats(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        panic_on_error(self.raw.alloc(layout))
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        self.raw.alloc(layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        panic_on_error(self.raw.alloc_sized())
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        self.raw.alloc_sized()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        panic_on_error(self.raw.alloc_slice(len))
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        self.raw.alloc_slice(len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
        panic_on_error(self.raw.alloc_slice_for(slice))
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        self.raw.alloc_slice_for(slice)
    }

    #[inline]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        if !S::SHRINKS {
            return None;
        }

        let old_ptr = ptr.cast::<u8>();
        let old_size = old_len * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_len * T::SIZE; // its less than the capacity so this can't overflow

        // Adapted from `Allocator::shrink`.
        unsafe {
            let chunk = self.raw.chunk.get();

            let is_last = if S::UP {
                old_ptr.as_ptr().add(old_size) == chunk.pos().as_ptr()
            } else {
                old_ptr == chunk.pos()
            };

            // if that's not the last allocation, there is nothing we can do
            if !is_last {
                return None;
            }

            // `is_last` is true, which guarantees a non-dummy, see `allocator_impl::is_last`
            let chunk = chunk.as_non_dummy_unchecked();

            if S::UP {
                let end = old_ptr.addr().get() + new_size;

                // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(end, S::MIN_ALIGN);

                chunk.set_pos_addr(new_pos);
                Some(old_ptr.cast())
            } else {
                let old_addr = old_ptr.addr();
                let old_addr_old_end = NonZeroUsize::new_unchecked(old_addr.get() + old_size);

                let new_addr = bump_down(old_addr_old_end, new_size, T::ALIGN.max(S::MIN_ALIGN));
                let new_addr = NonZeroUsize::new_unchecked(new_addr);
                let old_addr_new_end = NonZeroUsize::new_unchecked(old_addr.get() + new_size);

                let new_ptr = old_ptr.with_addr(new_addr);
                let overlaps = old_addr_new_end > new_addr;

                if overlaps {
                    old_ptr.copy_to(new_ptr, new_size);
                } else {
                    old_ptr.copy_to_nonoverlapping(new_ptr, new_size);
                }

                chunk.set_pos(new_ptr);
                Some(new_ptr.cast())
            }
        }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
        panic_on_error(self.raw.prepare_slice_allocation(len))
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
        self.raw.prepare_slice_allocation(len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe {
            // a successful `prepare_allocation` guarantees a non-dummy-chunk
            let chunk = self.raw.chunk.get().as_non_dummy_unchecked();

            let end = start.add(len);

            if S::UP {
                chunk.set_pos_addr_and_align_from(end.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(start, len)
            } else {
                let dst_end = start.add(cap);
                let dst = dst_end.sub(len);
                start.copy_to(dst, len);
                chunk.set_pos_addr_and_align_from(dst.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            }
        }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, end: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe {
            // a successful `prepare_allocation` guarantees a non-dummy-chunk
            let chunk = self.raw.chunk.get().as_non_dummy_unchecked();

            if S::UP {
                let dst = end.sub(cap);
                let dst_end = dst.add(len);

                let src = end.sub(len);

                src.copy_to(dst, len);

                chunk.set_pos_addr_and_align_from(dst_end.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            } else {
                let dst = end.sub(len);
                chunk.set_pos_addr_and_align_from(dst.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            }
        }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        panic_on_error(self.raw.reserve(additional));
    }

    #[inline(always)]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        self.raw.reserve(additional)
    }
}

unsafe impl<A, S> BumpAllocatorTyped for Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    type TypedStats<'b>
        = Stats<'b, S>
    where
        Self: 'b;

    #[inline(always)]
    fn typed_stats(&self) -> Self::TypedStats<'_> {
        self.as_scope().stats()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        self.as_scope().allocate_layout(layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        self.as_scope().try_allocate_layout(layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        self.as_scope().allocate_sized()
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        self.as_scope().try_allocate_sized()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        self.as_scope().allocate_slice(len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        self.as_scope().try_allocate_slice(len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
        self.as_scope().allocate_slice_for(slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        self.as_scope().try_allocate_slice_for(slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { self.as_scope().shrink_slice(ptr, old_len, new_len) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
        self.as_scope().prepare_slice_allocation(len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
        self.as_scope().try_prepare_slice_allocation(len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { self.as_scope().allocate_prepared_slice(ptr, len, cap) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { self.as_scope().allocate_prepared_slice_rev(ptr, len, cap) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        self.as_scope().reserve(additional);
    }

    #[inline(always)]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        self.as_scope().try_reserve(additional)
    }
}

#[cold]
#[inline(never)]
#[cfg(feature = "panic-on-alloc")]
pub(crate) const fn invalid_slice_layout() -> ! {
    panic!("invalid slice layout");
}
