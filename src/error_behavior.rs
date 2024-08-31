use layout::LayoutProps;

use crate::{layout, AllocError, Layout, NonNull, RawChunk, SupportedMinimumAlignment};

#[cfg(not(no_global_oom_handling))]
use crate::{capacity_overflow, format_trait_error, handle_alloc_error, Infallible};

pub(crate) trait ErrorBehavior: Sized {
    const IS_FALLIBLE: bool;

    fn allocation(layout: Layout) -> Self;
    fn capacity_overflow() -> Self;
    fn fixed_size_vector_is_full() -> Self;
    fn fixed_size_vector_no_space(amount: usize) -> Self;
    fn format_trait_error() -> Self;

    /// For the infallible case we want to inline `f` but not for the fallible one. (Because it produces better code)
    fn alloc_or_else<const UP: bool, A>(
        chunk: RawChunk<UP, A>,
        minimum_alignment: impl SupportedMinimumAlignment,
        layout: impl LayoutProps,
        f: impl FnOnce() -> Result<NonNull<u8>, Self>,
    ) -> Result<NonNull<u8>, Self>;
}

#[cfg(not(no_global_oom_handling))]
impl ErrorBehavior for Infallible {
    const IS_FALLIBLE: bool = false;

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
    fn alloc_or_else<const UP: bool, A>(
        chunk: RawChunk<UP, A>,
        minimum_alignment: impl SupportedMinimumAlignment,
        layout: impl LayoutProps,
        f: impl FnOnce() -> Result<NonNull<u8>, Self>,
    ) -> Result<NonNull<u8>, Self> {
        chunk.alloc_or_else(minimum_alignment, layout, f)
    }
}

impl ErrorBehavior for AllocError {
    const IS_FALLIBLE: bool = true;

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
    fn alloc_or_else<const UP: bool, A>(
        chunk: RawChunk<UP, A>,
        minimum_alignment: impl SupportedMinimumAlignment,
        layout: impl LayoutProps,
        f: impl FnOnce() -> Result<NonNull<u8>, Self>,
    ) -> Result<NonNull<u8>, Self> {
        match chunk.alloc(minimum_alignment, layout) {
            Some(ptr) => Ok(ptr),
            None => f(),
        }
    }
}

#[cold]
#[inline(never)]
#[cfg(not(no_global_oom_handling))]
fn fixed_size_vector_is_full() -> ! {
    panic!("fixed size vector is full");
}

#[cold]
#[inline(never)]
#[cfg(not(no_global_oom_handling))]
fn fixed_size_vector_no_space(amount: usize) -> ! {
    panic!("fixed size vector does not have space for {amount} more elements");
}
