#![expect(clippy::missing_safety_doc)]

use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

use crate::{
    BaseAllocator, Bump, BumpBox, BumpScope, SizedTypeProperties, WithoutDealloc, WithoutShrink,
    alloc::AllocError,
    bump_down,
    layout::CustomLayout,
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

        unsafe {
            ptr.drop_in_place();
            self.deallocate(ptr.cast(), layout);
        }
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_bytes`, <code>self.[stats][]().[remaining][]()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that these additional bytes are not necessarily in one contiguous region but
    /// might be spread out among many chunks.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve_bytes(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    ///
    /// [stats]: crate::traits::BumpAllocatorScope::stats
    /// [remaining]: Stats::remaining
    #[cfg(feature = "panic-on-alloc")]
    fn reserve_bytes(&self, additional: usize);

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_bytes`, <code>self.[stats][]().[remaining][]()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::try_new()?;
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.try_reserve_bytes(4096)?;
    /// assert!(bump.stats().capacity() >= 4096);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [stats]: crate::traits::BumpAllocatorScope::stats
    /// [remaining]: Stats::remaining
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError>;
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

unsafe impl BumpAllocatorTyped for dyn BumpAllocatorCore + '_ {
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
        allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice(self, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
        allocate_slice_for(self, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice_for(self, slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { shrink_slice(self, ptr, old_len, new_len) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
        prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
        try_prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { allocate_prepared_slice(self, ptr, len, cap) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { allocate_prepared_slice_rev(self, ptr, len, cap) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve_bytes(&self, additional: usize) {
        reserve_bytes(self, additional);
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        try_reserve_bytes(self, additional)
    }
}

unsafe impl BumpAllocatorTyped for dyn MutBumpAllocatorCore + '_ {
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
        allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice(self, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
        allocate_slice_for(self, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice_for(self, slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { shrink_slice(self, ptr, old_len, new_len) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
        prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
        try_prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { allocate_prepared_slice(self, ptr, len, cap) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { allocate_prepared_slice_rev(self, ptr, len, cap) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve_bytes(&self, additional: usize) {
        reserve_bytes(self, additional);
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        try_reserve_bytes(self, additional)
    }
}

unsafe impl BumpAllocatorTyped for dyn BumpAllocatorCoreScope<'_> + '_ {
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
        allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice(self, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
        allocate_slice_for(self, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice_for(self, slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { shrink_slice(self, ptr, old_len, new_len) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
        prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
        try_prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { allocate_prepared_slice(self, ptr, len, cap) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { allocate_prepared_slice_rev(self, ptr, len, cap) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve_bytes(&self, additional: usize) {
        reserve_bytes(self, additional);
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        try_reserve_bytes(self, additional)
    }
}

unsafe impl BumpAllocatorTyped for dyn MutBumpAllocatorCoreScope<'_> + '_ {
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
        allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice(self, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
        allocate_slice_for(self, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice_for(self, slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { shrink_slice(self, ptr, old_len, new_len) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
        prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
        try_prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { allocate_prepared_slice(self, ptr, len, cap) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { allocate_prepared_slice_rev(self, ptr, len, cap) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve_bytes(&self, additional: usize) {
        reserve_bytes(self, additional);
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        try_reserve_bytes(self, additional)
    }
}

#[inline]
#[cfg(feature = "panic-on-alloc")]
fn allocate_layout(bump: impl BumpAllocatorCore, layout: Layout) -> NonNull<u8> {
    match bump.allocate(layout) {
        Ok(ptr) => ptr.cast(),
        Err(AllocError) => handle_alloc_error(layout),
    }
}

#[inline]
fn try_allocate_layout(bump: impl BumpAllocatorCore, layout: Layout) -> Result<NonNull<u8>, AllocError> {
    match bump.allocate(layout) {
        Ok(ptr) => Ok(ptr.cast()),
        Err(err) => Err(err),
    }
}

#[inline]
#[cfg(feature = "panic-on-alloc")]
fn allocate_sized<T>(bump: impl BumpAllocatorCore) -> NonNull<T> {
    if T::IS_ZST {
        return NonNull::dangling();
    }

    let layout = Layout::new::<T>();

    match bump.allocate(layout) {
        Ok(ptr) => ptr.cast(),
        Err(AllocError) => handle_alloc_error(Layout::new::<T>()),
    }
}

#[inline]
fn try_allocate_sized<T>(bump: impl BumpAllocatorCore) -> Result<NonNull<T>, AllocError> {
    if T::IS_ZST {
        return Ok(NonNull::dangling());
    }

    match bump.allocate(Layout::new::<T>()) {
        Ok(ptr) => Ok(ptr.cast()),
        Err(err) => Err(err),
    }
}

#[inline]
#[cfg(feature = "panic-on-alloc")]
fn allocate_slice<T>(bump: impl BumpAllocatorCore, len: usize) -> NonNull<T> {
    if T::IS_ZST {
        return NonNull::dangling();
    }

    let Ok(layout) = Layout::array::<T>(len) else {
        invalid_slice_layout()
    };

    match bump.allocate(layout) {
        Ok(ptr) => ptr.cast(),
        Err(AllocError) => handle_alloc_error(layout),
    }
}

#[inline]
fn try_allocate_slice<T>(bump: impl BumpAllocatorCore, len: usize) -> Result<NonNull<T>, AllocError> {
    if T::IS_ZST {
        return Ok(NonNull::dangling());
    }

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
fn allocate_slice_for<T>(bump: impl BumpAllocatorCore, slice: &[T]) -> NonNull<T> {
    if T::IS_ZST {
        return NonNull::dangling();
    }

    let layout = Layout::for_value(slice);

    match bump.allocate(layout) {
        Ok(ptr) => ptr.cast(),
        Err(AllocError) => handle_alloc_error(layout),
    }
}

#[inline]
fn try_allocate_slice_for<T>(bump: impl BumpAllocatorCore, slice: &[T]) -> Result<NonNull<T>, AllocError> {
    if T::IS_ZST {
        return Ok(NonNull::dangling());
    }

    let layout = Layout::for_value(slice);

    match bump.allocate(layout) {
        Ok(ptr) => Ok(ptr.cast()),
        Err(err) => Err(err),
    }
}

#[inline]
#[expect(clippy::unnecessary_wraps)]
unsafe fn shrink_slice<T>(
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
fn prepare_slice_allocation<T>(bump: impl BumpAllocatorCore, min_cap: usize) -> NonNull<[T]> {
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
fn try_prepare_slice_allocation<T>(bump: impl BumpAllocatorCore, len: usize) -> Result<NonNull<[T]>, AllocError> {
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
unsafe fn allocate_prepared_slice<T>(bump: impl BumpAllocatorCore, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
    unsafe {
        let range = non_null::cast_range(ptr..ptr.add(cap));
        let layout = Layout::from_size_align_unchecked(core::mem::size_of::<T>() * len, T::ALIGN);
        let data = bump.allocate_prepared(layout, range).cast();
        NonNull::slice_from_raw_parts(data, len)
    }
}

#[inline(always)]
unsafe fn allocate_prepared_slice_rev<T>(
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
fn reserve_bytes(bump: impl BumpAllocatorCore, additional: usize) {
    let Ok(layout) = Layout::array::<u8>(additional) else {
        invalid_slice_layout();
    };

    if let Err(AllocError) = bump.prepare_allocation(layout) {
        handle_alloc_error(layout);
    }
}

fn try_reserve_bytes(bump: impl BumpAllocatorCore, additional: usize) -> Result<(), AllocError> {
    let Ok(layout) = Layout::array::<u8>(additional) else {
        return Err(AllocError);
    };

    match bump.prepare_allocation(layout) {
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}

unsafe impl<B: BumpAllocatorTyped + ?Sized> BumpAllocatorTyped for &B {
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
    fn reserve_bytes(&self, additional: usize) {
        B::reserve_bytes(self, additional);
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        B::try_reserve_bytes(self, additional)
    }
}

unsafe impl<B: BumpAllocatorTyped + ?Sized> BumpAllocatorTyped for &mut B {
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
    fn reserve_bytes(&self, additional: usize) {
        B::reserve_bytes(self, additional);
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        B::try_reserve_bytes(self, additional)
    }
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
    fn reserve_bytes(&self, additional: usize) {
        B::reserve_bytes(&self.0, additional);
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        B::try_reserve_bytes(&self.0, additional)
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
    fn reserve_bytes(&self, additional: usize) {
        B::reserve_bytes(&self.0, additional);
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        B::try_reserve_bytes(&self.0, additional)
    }
}

unsafe impl<A, S> BumpAllocatorTyped for BumpScope<'_, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    type TypedStats<'b>
        = Stats<'b, A, S>
    where
        Self: 'b;

    #[inline(always)]
    fn typed_stats(&self) -> Self::TypedStats<'_> {
        BumpScope::stats(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        match self.chunk.get().alloc::<S::MinimumAlignment>(CustomLayout(layout)) {
            Some(ptr) => ptr,
            None => panic_on_error(self.alloc_in_another_chunk(layout)),
        }
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        match self.chunk.get().alloc::<S::MinimumAlignment>(CustomLayout(layout)) {
            Some(ptr) => Ok(ptr),
            None => self.alloc_in_another_chunk(layout),
        }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        panic_on_error(self.do_alloc_sized())
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        self.do_alloc_sized()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        panic_on_error(self.do_alloc_slice(len))
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        self.do_alloc_slice(len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T> {
        panic_on_error(self.do_alloc_slice_for(slice))
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
        self.do_alloc_slice_for(slice)
    }

    #[inline]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        if !S::DEALLOCATES {
            return None;
        }

        let old_ptr = ptr.cast::<u8>();
        let old_size = old_len * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_len * T::SIZE; // its less than the capacity so this can't overflow

        // Adapted from `Allocator::shrink`.
        unsafe {
            let is_last_and_allocated = self.chunk.get().is_allocated()
                && if S::UP {
                    old_ptr.as_ptr().add(old_size) == self.chunk.get().pos().as_ptr()
                } else {
                    old_ptr == self.chunk.get().pos()
                };

            // if that's not the last allocation, there is nothing we can do
            if !is_last_and_allocated {
                return None;
            }

            if S::UP {
                let end = old_ptr.addr().get() + new_size;

                // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(end, S::MIN_ALIGN);

                self.chunk.get().guaranteed_allocated_unchecked().set_pos_addr(new_pos);
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

                self.chunk.get().guaranteed_allocated_unchecked().set_pos(new_ptr);
                Some(new_ptr.cast())
            }
        }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn prepare_slice_allocation<T>(&self, len: usize) -> NonNull<[T]> {
        panic_on_error(BumpScope::generic_prepare_slice_allocation::<_, T>(self, len))
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&self, len: usize) -> Result<NonNull<[T]>, AllocError> {
        BumpScope::generic_prepare_slice_allocation::<_, T>(self, len)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { BumpScope::use_prepared_slice_allocation(self, ptr, len, cap) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { BumpScope::use_prepared_slice_allocation_rev(self, ptr, len, cap) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve_bytes(&self, additional: usize) {
        panic_on_error(self.generic_reserve_bytes(additional));
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        self.generic_reserve_bytes(additional)
    }
}

unsafe impl<A, S> BumpAllocatorTyped for Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    type TypedStats<'b>
        = Stats<'b, A, S>
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
        unsafe { self.as_scope().use_prepared_slice_allocation(ptr, len, cap) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_slice_rev<T>(&self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { self.as_scope().use_prepared_slice_allocation_rev(ptr, len, cap) }
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve_bytes(&self, additional: usize) {
        self.as_scope().reserve_bytes(additional);
    }

    #[inline(always)]
    fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        self.as_scope().try_reserve_bytes(additional)
    }
}

#[cold]
#[inline(never)]
#[cfg(feature = "panic-on-alloc")]
pub(crate) const fn invalid_slice_layout() -> ! {
    panic!("invalid slice layout");
}
