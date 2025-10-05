use core::alloc::Layout;

use crate::{BumpAllocatorExt, MutBumpAllocatorExt, NonNull, alloc::AllocError};

#[cfg(feature = "panic-on-alloc")]
use crate::{Infallible, capacity_overflow, format_trait_error, handle_alloc_error};

pub(crate) trait ErrorBehavior: Sized {
    #[cfg(feature = "panic-on-alloc")]
    const PANICS_ON_ALLOC: bool;

    fn allocation(layout: Layout) -> Self;
    fn capacity_overflow() -> Self;
    fn fixed_size_vector_is_full() -> Self;
    fn fixed_size_vector_no_space(amount: usize) -> Self;
    fn format_trait_error() -> Self;
    #[expect(dead_code)]
    fn allocate_layout(allocator: &impl BumpAllocatorExt, layout: Layout) -> Result<NonNull<u8>, Self>;
    #[expect(dead_code)]
    fn allocate_sized<T>(allocator: &impl BumpAllocatorExt) -> Result<NonNull<T>, Self>;
    fn allocate_slice<T>(allocator: &impl BumpAllocatorExt, len: usize) -> Result<NonNull<T>, Self>;
    unsafe fn prepare_slice_allocation<T>(
        allocator: &mut impl MutBumpAllocatorExt,
        len: usize,
    ) -> Result<NonNull<[T]>, Self>;
}

#[cfg(feature = "panic-on-alloc")]
impl ErrorBehavior for Infallible {
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
    fn fixed_size_vector_is_full() -> Self {
        fixed_size_vector_is_full()
    }

    #[inline(always)]
    fn fixed_size_vector_no_space(amount: usize) -> Self {
        fixed_size_vector_no_space(amount)
    }

    #[inline(always)]
    fn format_trait_error() -> Self {
        format_trait_error()
    }

    #[inline(always)]
    fn allocate_layout(allocator: &impl BumpAllocatorExt, layout: Layout) -> Result<NonNull<u8>, Self> {
        Ok(allocator.allocate_layout(layout))
    }

    #[inline(always)]
    fn allocate_sized<T>(allocator: &impl BumpAllocatorExt) -> Result<NonNull<T>, Self> {
        Ok(allocator.allocate_sized::<T>())
    }

    #[inline(always)]
    fn allocate_slice<T>(allocator: &impl BumpAllocatorExt, len: usize) -> Result<NonNull<T>, Self> {
        Ok(allocator.allocate_slice::<T>(len))
    }

    #[inline(always)]
    unsafe fn prepare_slice_allocation<T>(
        allocator: &mut impl MutBumpAllocatorExt,
        len: usize,
    ) -> Result<NonNull<[T]>, Self> {
        Ok(allocator.prepare_slice_allocation::<T>(len))
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
    fn fixed_size_vector_is_full() -> Self {
        Self
    }

    #[inline(always)]
    fn fixed_size_vector_no_space(amount: usize) -> Self {
        let _ = amount;
        Self
    }

    #[inline(always)]
    fn format_trait_error() -> Self {
        Self
    }

    #[inline(always)]
    fn allocate_layout(allocator: &impl BumpAllocatorExt, layout: Layout) -> Result<NonNull<u8>, Self> {
        allocator.try_allocate_layout(layout)
    }

    #[inline(always)]
    fn allocate_sized<T>(allocator: &impl BumpAllocatorExt) -> Result<NonNull<T>, Self> {
        allocator.try_allocate_sized::<T>()
    }

    #[inline(always)]
    fn allocate_slice<T>(allocator: &impl BumpAllocatorExt, len: usize) -> Result<NonNull<T>, Self> {
        allocator.try_allocate_slice::<T>(len)
    }

    #[inline(always)]
    unsafe fn prepare_slice_allocation<T>(
        allocator: &mut impl MutBumpAllocatorExt,
        len: usize,
    ) -> Result<NonNull<[T]>, Self> {
        allocator.try_prepare_slice_allocation::<T>(len)
    }
}

#[cold]
#[inline(never)]
#[cfg(feature = "panic-on-alloc")]
fn fixed_size_vector_is_full() -> ! {
    panic!("fixed size vector is full");
}

#[cold]
#[inline(never)]
#[cfg(feature = "panic-on-alloc")]
fn fixed_size_vector_no_space(amount: usize) -> ! {
    panic!("fixed size vector does not have space for {amount} more elements");
}
