use crate::{
    bumping::{bump_down, bump_up, BumpUp},
    polyfill::nonnull,
    BaseAllocator, BumpBox, MinimumAlignment,
};
#[cfg(not(no_global_oom_handling))]
use crate::{
    capacity_overflow, format_trait_error, handle_alloc_error, layout, AllocError, BumpScope, Infallible, Layout, NonNull,
    RawChunk, SizedTypeProperties, SupportedMinimumAlignment,
};
use layout::LayoutProps;

pub(crate) trait ErrorBehavior: Sized {
    const IS_FALLIBLE: bool;

    fn allocation(layout: Layout) -> Self;
    fn capacity_overflow() -> Self;
    fn fixed_size_vector_is_full() -> Self;
    fn fixed_size_vector_no_space(amount: usize) -> Self;
    fn format_trait_error() -> Self;

    #[inline(always)]
    fn alloc_with<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, T>(
        bump: &BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
        f: impl FnOnce() -> T,
    ) -> Result<BumpBox<'a, T>, Self>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        A: BaseAllocator<GUARANTEED_ALLOCATED>,
    {
        if T::IS_ZST {
            let value = f();
            return Ok(BumpBox::zst(value));
        }

        let chunk = bump.chunk.get();
        let props = chunk.bump_props(MinimumAlignment::<MIN_ALIGN>, crate::layout::SizedLayout::new::<T>());

        let ptr = unsafe {
            if UP {
                if let Some(BumpUp { new_pos, ptr }) = bump_up(props) {
                    chunk.set_pos_addr(new_pos);
                    chunk.with_addr(ptr)
                } else {
                    bump.do_alloc_sized_in_another_chunk::<Self, T>()?
                }
            } else {
                if let Some(addr) = bump_down(props) {
                    chunk.set_pos_addr(addr);
                    chunk.with_addr(addr)
                } else {
                    bump.do_alloc_sized_in_another_chunk::<Self, T>()?
                }
            }
        };

        let ptr = ptr.cast::<T>();

        unsafe {
            nonnull::write_with(ptr, f);
            Ok(BumpBox::from_raw(ptr))
        }
    }

    /// For the infallible case we want to inline `f` but not for the fallible one. (Because it produces better code)
    fn alloc_or_else<const UP: bool, A>(
        chunk: RawChunk<UP, A>,
        minimum_alignment: impl SupportedMinimumAlignment,
        layout: impl LayoutProps,
        f: impl FnOnce() -> Result<NonNull<u8>, Self>,
    ) -> Result<NonNull<u8>, Self>;

    /// For the infallible case we want to inline `f` but not for the fallible one. (Because it produces better code)
    fn reserve_or_else<const UP: bool, A>(
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

    #[inline(always)]
    fn reserve_or_else<const UP: bool, A>(
        chunk: RawChunk<UP, A>,
        minimum_alignment: impl SupportedMinimumAlignment,
        layout: impl LayoutProps,
        f: impl FnOnce() -> Result<NonNull<u8>, Self>,
    ) -> Result<NonNull<u8>, Self> {
        chunk.reserve_or_else(minimum_alignment, layout, f)
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

    #[inline(always)]
    fn reserve_or_else<const UP: bool, A>(
        chunk: RawChunk<UP, A>,
        minimum_alignment: impl SupportedMinimumAlignment,
        layout: impl LayoutProps,
        f: impl FnOnce() -> Result<NonNull<u8>, Self>,
    ) -> Result<NonNull<u8>, Self> {
        match chunk.reserve(minimum_alignment, layout) {
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
