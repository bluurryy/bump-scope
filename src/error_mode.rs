use core::alloc::{AllocError, Allocator, Layout};

#[cfg(feature = "panic-on-alloc")]
use alloc::alloc::handle_alloc_error;
use alloc::vec::Vec;

#[cfg(feature = "panic-on-alloc")]
use crate::capacity_overflow;

// This is just `Result::into_ok` but with a name to match our use case.
#[inline(always)]
#[cfg(feature = "panic-on-alloc")]
pub(crate) fn panic_on_error<T>(result: Result<T, !>) -> T {
    match result {
        Ok(ok) => ok,
    }
}

pub(crate) trait ErrorMode: Sized {
    const FALLIBLE: bool;
    fn allocation(layout: Layout) -> Self;
    fn capacity_overflow() -> Self;
    fn claimed() -> Self;
    fn unmanaged() -> Self;
    fn format_trait_error() -> Self;
    fn vec_reserve<T, A: Allocator>(vec: &mut Vec<T, A>, additional: usize) -> Result<(), Self>;
    fn vec_reserve_if_fallible<T, A: Allocator>(vec: &mut Vec<T, A>, additional: usize) -> Result<(), Self>;
}

#[cfg(feature = "panic-on-alloc")]
impl ErrorMode for ! {
    const FALLIBLE: bool = false;

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
    fn unmanaged() -> Self {
        panic::unmanaged();
    }

    #[inline(always)]
    fn format_trait_error() -> Self {
        panic::format_trait_error()
    }

    #[inline(always)]
    fn vec_reserve<T, A: Allocator>(vec: &mut Vec<T, A>, additional: usize) -> Result<(), Self> {
        vec.reserve(additional);
        Ok(())
    }

    #[inline(always)]
    fn vec_reserve_if_fallible<T, A: Allocator>(vec: &mut Vec<T, A>, additional: usize) -> Result<(), Self> {
        _ = (vec, additional);
        Ok(())
    }
}

impl ErrorMode for AllocError {
    const FALLIBLE: bool = true;

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
    fn unmanaged() -> Self {
        Self
    }

    #[inline(always)]
    fn format_trait_error() -> Self {
        Self
    }

    #[inline(always)]
    fn vec_reserve<T, A: Allocator>(vec: &mut Vec<T, A>, additional: usize) -> Result<(), Self> {
        vec.try_reserve(additional).map_err(|_| AllocError)
    }

    #[inline(always)]
    fn vec_reserve_if_fallible<T, A: Allocator>(vec: &mut Vec<T, A>, additional: usize) -> Result<(), Self> {
        vec.try_reserve(additional).map_err(|_| AllocError)
    }
}

pub(crate) mod panic {
    #[cold]
    #[cfg(feature = "panic-on-alloc")]
    #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
    pub(crate) fn claimed() -> ! {
        panic!("bump allocator is claimed");
    }

    #[cold]
    #[cfg(feature = "panic-on-alloc")]
    #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
    pub(crate) fn unmanaged() -> ! {
        panic!("can't allocate on an `Unmanaged` allocator")
    }

    #[cold]
    #[cfg(feature = "panic-on-alloc")]
    #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
    pub const fn format_trait_error() -> ! {
        panic!("formatting trait implementation returned an error");
    }
}
