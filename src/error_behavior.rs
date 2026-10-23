use core::alloc::{AllocError, Layout};

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
    fn unmanaged() -> Self;
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
    fn unmanaged() -> Self {
        panic::unmanaged();
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
    fn unmanaged() -> Self {
        Self
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

    #[cold]
    #[inline(never)]
    pub(crate) fn unmanaged() -> ! {
        panic!("can't allocate on an `Unmanaged` allocator")
    }
}
