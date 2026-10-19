use core::alloc::{AllocError, Layout};

use crate::{
    NonNull,
    traits::{BumpAllocator, MutBumpAllocator},
};

#[cfg(feature = "panic-on-alloc")]
use crate::{capacity_overflow, format_trait_error, handle_alloc_error};

pub(crate) trait ErrorBehavior: Sized {
    #[cfg(feature = "panic-on-alloc")]
    const PANICS_ON_ALLOC: bool;

    fn allocation(layout: Layout) -> Self;
    fn capacity_overflow() -> Self;
    fn claimed() -> Self;
    fn fixed_size_vector_is_full() -> Self;
    fn fixed_size_vector_no_space(amount: usize) -> Self;
    fn invalid_slice_layout() -> Self;
    fn format_trait_error() -> Self;
    #[cfg(feature = "std")]
    fn base_allocator_default_constructor_panicked() -> Self;
    #[expect(dead_code)]
    fn allocate_layout(allocator: &impl BumpAllocator, layout: Layout) -> Result<NonNull<u8>, Self>;
    #[expect(dead_code)]
    fn allocate_sized<T>(allocator: &impl BumpAllocator) -> Result<NonNull<T>, Self>;
    fn allocate_slice<T>(allocator: &impl BumpAllocator, len: usize) -> Result<NonNull<T>, Self>;
    unsafe fn prepare_slice_allocation<T>(allocator: &mut impl MutBumpAllocator, len: usize) -> Result<NonNull<[T]>, Self>;
    unsafe fn prepare_slice_allocation_rev<T>(
        allocator: &mut impl MutBumpAllocator,
        len: usize,
    ) -> Result<(NonNull<T>, usize), Self>;
}

#[cfg(feature = "panic-on-alloc")]
impl ErrorBehavior for ! {
    #[cfg(feature = "panic-on-alloc")]
    const PANICS_ON_ALLOC: bool = true;

    #[inline(always)]
    fn allocation(layout: Layout) -> Self {
        handle_alloc_error(layout)
    }

    #[inline(always)]
    fn capacity_overflow() -> Self {
        capacity_overflow()
    }

    #[inline(always)]
    fn claimed() -> Self {
        panic::claimed()
    }

    #[inline(always)]
    fn fixed_size_vector_is_full() -> Self {
        panic::fixed_size_vector_is_full()
    }

    #[inline(always)]
    fn fixed_size_vector_no_space(amount: usize) -> Self {
        panic::fixed_size_vector_no_space(amount)
    }

    #[inline(always)]
    fn invalid_slice_layout() -> Self {
        panic::invalid_slice_layout()
    }

    #[inline(always)]
    fn format_trait_error() -> Self {
        format_trait_error()
    }

    #[inline(always)]
    #[cfg(feature = "std")]
    fn base_allocator_default_constructor_panicked() -> Self {
        panic::base_allocator_default_constructor_panicked()
    }

    #[inline(always)]
    fn allocate_layout(allocator: &impl BumpAllocator, layout: Layout) -> Result<NonNull<u8>, Self> {
        Ok(allocator.allocate_layout(layout))
    }

    #[inline(always)]
    fn allocate_sized<T>(allocator: &impl BumpAllocator) -> Result<NonNull<T>, Self> {
        Ok(allocator.allocate_sized::<T>())
    }

    #[inline(always)]
    fn allocate_slice<T>(allocator: &impl BumpAllocator, len: usize) -> Result<NonNull<T>, Self> {
        Ok(allocator.allocate_slice::<T>(len))
    }

    #[inline(always)]
    unsafe fn prepare_slice_allocation<T>(allocator: &mut impl MutBumpAllocator, len: usize) -> Result<NonNull<[T]>, Self> {
        Ok(allocator.prepare_slice_allocation::<T>(len))
    }

    #[inline(always)]
    unsafe fn prepare_slice_allocation_rev<T>(
        allocator: &mut impl MutBumpAllocator,
        len: usize,
    ) -> Result<(NonNull<T>, usize), Self> {
        Ok(allocator.prepare_slice_allocation_rev::<T>(len))
    }
}

impl ErrorBehavior for AllocError {
    #[cfg(feature = "panic-on-alloc")]
    const PANICS_ON_ALLOC: bool = false;

    #[inline(always)]
    fn allocation(_: Layout) -> Self {
        Self
    }

    #[inline(always)]
    fn capacity_overflow() -> Self {
        Self
    }

    #[inline(always)]
    fn claimed() -> Self {
        Self
    }

    #[inline(always)]
    fn fixed_size_vector_is_full() -> Self {
        Self
    }

    #[inline(always)]
    fn fixed_size_vector_no_space(amount: usize) -> Self {
        let _ = amount;
        Self
    }

    #[inline(always)]
    fn invalid_slice_layout() -> Self {
        Self
    }

    #[inline(always)]
    fn format_trait_error() -> Self {
        Self
    }

    #[inline(always)]
    #[cfg(feature = "std")]
    fn base_allocator_default_constructor_panicked() -> Self {
        Self
    }

    #[inline(always)]
    fn allocate_layout(allocator: &impl BumpAllocator, layout: Layout) -> Result<NonNull<u8>, Self> {
        allocator.try_allocate_layout(layout)
    }

    #[inline(always)]
    fn allocate_sized<T>(allocator: &impl BumpAllocator) -> Result<NonNull<T>, Self> {
        allocator.try_allocate_sized::<T>()
    }

    #[inline(always)]
    fn allocate_slice<T>(allocator: &impl BumpAllocator, len: usize) -> Result<NonNull<T>, Self> {
        allocator.try_allocate_slice::<T>(len)
    }

    #[inline(always)]
    unsafe fn prepare_slice_allocation<T>(allocator: &mut impl MutBumpAllocator, len: usize) -> Result<NonNull<[T]>, Self> {
        allocator.try_prepare_slice_allocation::<T>(len)
    }

    #[inline(always)]
    unsafe fn prepare_slice_allocation_rev<T>(
        allocator: &mut impl MutBumpAllocator,
        len: usize,
    ) -> Result<(NonNull<T>, usize), Self> {
        allocator.try_prepare_slice_allocation_rev::<T>(len)
    }
}

pub(crate) mod panic {
    #[cold]
    #[inline(never)]
    pub(crate) fn claimed() -> ! {
        panic!("bump allocator is claimed");
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn unallocated() -> ! {
        panic!("bump allocator is unallocated");
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) fn fixed_size_vector_is_full() -> ! {
        panic!("fixed size vector is full");
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) fn fixed_size_vector_no_space(amount: usize) -> ! {
        panic!("fixed size vector does not have space for {amount} more elements");
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) const fn invalid_slice_layout() -> ! {
        panic!("invalid slice layout");
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "std")]
    pub(crate) fn base_allocator_default_constructor_panicked() -> ! {
        panic!("base allocator default constructor panicked")
    }
}
