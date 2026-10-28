use core::alloc::{AllocError, Allocator, Layout};

use alloc::vec::Vec;

#[cfg(feature = "panic-on-alloc")]
use crate::{capacity_overflow, handle_alloc_error};

pub(crate) trait ErrorBehavior: Sized {
    const FALLIBLE: bool;
    fn allocation(layout: Layout) -> Self;
    fn vec_reserve() -> Self;
    fn capacity_overflow() -> Self;
    fn claimed() -> Self;
    #[cfg(feature = "std")]
    fn base_allocator_default_constructor_panicked() -> Self;
    fn unmanaged() -> Self;
    fn format_trait_error() -> Self;
    fn vec_with_capacity_in<T, A: Allocator>(cap: usize, alloc: A) -> Result<Vec<T, A>, Self>;
    fn vec_reserve_if_fallible<T, A: Allocator>(vec: &mut Vec<T, A>, additional: usize) -> Result<(), Self>;
}

#[cfg(feature = "panic-on-alloc")]
impl ErrorBehavior for ! {
    const FALLIBLE: bool = false;

    #[inline(always)]
    fn allocation(layout: Layout) -> Self {
        handle_alloc_error(layout)
    }

    #[inline(always)]
    fn vec_reserve() -> Self {
        panic::vec_reserve();
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
    #[cfg(feature = "std")]
    fn base_allocator_default_constructor_panicked() -> Self {
        panic::base_allocator_default_constructor_panicked()
    }

    #[inline(always)]
    fn unmanaged() -> Self {
        panic::unmanaged();
    }

    #[inline(always)]
    fn format_trait_error() -> Self {
        panic::format_trait_error()
    }

    #[inline(always)]
    fn vec_with_capacity_in<T, A: Allocator>(cap: usize, alloc: A) -> Result<Vec<T, A>, Self> {
        Ok(Vec::<T, _>::with_capacity_in(cap, alloc))
    }

    #[inline(always)]
    fn vec_reserve_if_fallible<T, A: Allocator>(vec: &mut Vec<T, A>, additional: usize) -> Result<(), Self> {
        _ = (vec, additional);
        Ok(())
    }
}

impl ErrorBehavior for AllocError {
    const FALLIBLE: bool = true;

    #[inline(always)]
    fn allocation(_: Layout) -> Self {
        Self
    }

    #[inline(always)]
    fn vec_reserve() -> Self {
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
    #[cfg(feature = "std")]
    fn base_allocator_default_constructor_panicked() -> Self {
        Self
    }

    #[inline(always)]
    fn unmanaged() -> Self {
        Self
    }

    #[inline(always)]
    fn format_trait_error() -> Self {
        Self
    }

    #[inline(always)]
    fn vec_with_capacity_in<T, A: Allocator>(cap: usize, alloc: A) -> Result<Vec<T, A>, Self> {
        let mut vec = Vec::new_in(alloc);
        vec.try_reserve(cap).map_err(|_| AllocError)?;
        Ok(vec)
    }

    #[inline(always)]
    fn vec_reserve_if_fallible<T, A: Allocator>(vec: &mut Vec<T, A>, additional: usize) -> Result<(), Self> {
        vec.try_reserve(additional).map_err(|_| AllocError)
    }
}

pub(crate) mod panic {
    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) fn claimed() -> ! {
        panic!("bump allocator is claimed");
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "std")]
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) fn base_allocator_default_constructor_panicked() -> ! {
        panic!("base allocator default constructor panicked")
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) fn unmanaged() -> ! {
        panic!("can't allocate on an `Unmanaged` allocator")
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    pub const fn format_trait_error() -> ! {
        panic!("formatting trait implementation returned an error");
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    pub const fn vec_reserve() -> ! {
        panic!("memory allocation failed");
    }
}
