use core::alloc::{AllocError, Layout};

#[cfg(feature = "panic-on-alloc")]
use crate::{capacity_overflow, handle_alloc_error};

pub(crate) trait ErrorBehavior: Sized {
    fn allocation(layout: Layout) -> Self;
    fn capacity_overflow() -> Self;
    fn claimed() -> Self;
    #[cfg(feature = "std")]
    fn base_allocator_default_constructor_panicked() -> Self;
    fn unmanaged() -> Self;
}

#[cfg(feature = "panic-on-alloc")]
impl ErrorBehavior for ! {
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
