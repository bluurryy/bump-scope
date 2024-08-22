use core::{alloc::Layout, mem::MaybeUninit, ptr::NonNull};

#[cfg(feature = "alloc")]
use core::fmt;

use crate::{
    allocation_behavior::LayoutProps, BaseAllocator, Bump, BumpBox, BumpScope, ErrorBehavior, MinimumAlignment,
    SupportedMinimumAlignment,
};

pub(crate) trait Sealed {
    fn alloc_uninit<B: ErrorBehavior, T>(&self) -> Result<BumpBox<MaybeUninit<T>>, B>;
    fn alloc_with<B: ErrorBehavior, T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<T>, B>;
    fn alloc_slice_copy<B: ErrorBehavior, T: Copy>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B>;
    fn alloc_slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B>;
    fn alloc_slice_fill<B: ErrorBehavior, T: Clone>(&self, len: usize, value: T) -> Result<BumpBox<[T]>, B>;
    fn alloc_slice_fill_with<B: ErrorBehavior, T>(&self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<[T]>, B>;
    fn alloc_str<B: ErrorBehavior>(&self, src: &str) -> Result<BumpBox<str>, B>;
    #[cfg(feature = "alloc")]
    fn alloc_fmt<B: ErrorBehavior>(&self, args: fmt::Arguments) -> Result<BumpBox<str>, B>;
    fn reserve_bytes<B: ErrorBehavior>(&self, additional: usize) -> Result<(), B>;

    fn alloc_in_current_chunk(&self, layout: impl LayoutProps) -> Option<NonNull<u8>>;
    fn alloc_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E>;
}

impl<U: Sealed> Sealed for &U {
    #[inline(always)]
    fn alloc_uninit<B: ErrorBehavior, T>(&self) -> Result<BumpBox<MaybeUninit<T>>, B> {
        U::alloc_uninit(self)
    }

    #[inline(always)]
    fn alloc_with<B: ErrorBehavior, T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<T>, B> {
        U::alloc_with(self, f)
    }

    #[inline(always)]
    fn alloc_slice_copy<B: ErrorBehavior, T: Copy>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B> {
        U::alloc_slice_copy(self, slice)
    }

    #[inline(always)]
    fn alloc_slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B> {
        U::alloc_slice_clone(self, slice)
    }

    #[inline(always)]
    fn alloc_slice_fill<B: ErrorBehavior, T: Clone>(&self, len: usize, value: T) -> Result<BumpBox<[T]>, B> {
        U::alloc_slice_fill(self, len, value)
    }

    #[inline(always)]
    fn alloc_slice_fill_with<B: ErrorBehavior, T>(&self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<[T]>, B> {
        U::alloc_slice_fill_with(self, len, f)
    }

    #[inline(always)]
    fn alloc_str<B: ErrorBehavior>(&self, src: &str) -> Result<BumpBox<str>, B> {
        U::alloc_str(self, src)
    }

    #[inline(always)]
    #[cfg(feature = "alloc")]
    fn alloc_fmt<B: ErrorBehavior>(&self, args: fmt::Arguments) -> Result<BumpBox<str>, B> {
        U::alloc_fmt(self, args)
    }

    #[inline(always)]
    fn reserve_bytes<B: ErrorBehavior>(&self, additional: usize) -> Result<(), B> {
        U::reserve_bytes(self, additional)
    }

    #[inline(always)]
    fn alloc_in_current_chunk(&self, layout: impl LayoutProps) -> Option<NonNull<u8>> {
        U::alloc_in_current_chunk(self, layout)
    }

    #[inline(always)]
    fn alloc_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        U::alloc_in_another_chunk(self, layout)
    }
}

impl<U: Sealed> Sealed for &mut U {
    #[inline(always)]
    fn alloc_uninit<B: ErrorBehavior, T>(&self) -> Result<BumpBox<MaybeUninit<T>>, B> {
        U::alloc_uninit(self)
    }

    #[inline(always)]
    fn alloc_with<B: ErrorBehavior, T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<T>, B> {
        U::alloc_with(self, f)
    }

    #[inline(always)]
    fn alloc_slice_copy<B: ErrorBehavior, T: Copy>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B> {
        U::alloc_slice_copy(self, slice)
    }

    #[inline(always)]
    fn alloc_slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B> {
        U::alloc_slice_clone(self, slice)
    }

    #[inline(always)]
    fn alloc_slice_fill<B: ErrorBehavior, T: Clone>(&self, len: usize, value: T) -> Result<BumpBox<[T]>, B> {
        U::alloc_slice_fill(self, len, value)
    }

    #[inline(always)]
    fn alloc_slice_fill_with<B: ErrorBehavior, T>(&self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<[T]>, B> {
        U::alloc_slice_fill_with(self, len, f)
    }

    #[inline(always)]
    fn alloc_str<B: ErrorBehavior>(&self, src: &str) -> Result<BumpBox<str>, B> {
        U::alloc_str(self, src)
    }

    #[inline(always)]
    #[cfg(feature = "alloc")]
    fn alloc_fmt<B: ErrorBehavior>(&self, args: fmt::Arguments) -> Result<BumpBox<str>, B> {
        U::alloc_fmt(self, args)
    }

    #[inline(always)]
    fn reserve_bytes<B: ErrorBehavior>(&self, additional: usize) -> Result<(), B> {
        U::reserve_bytes(self, additional)
    }

    #[inline(always)]
    fn alloc_in_current_chunk(&self, layout: impl LayoutProps) -> Option<NonNull<u8>> {
        U::alloc_in_current_chunk(self, layout)
    }

    #[inline(always)]
    fn alloc_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        U::alloc_in_another_chunk(self, layout)
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Sealed
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn alloc_uninit<B: ErrorBehavior, T>(&self) -> Result<BumpBox<MaybeUninit<T>>, B> {
        BumpScope::generic_alloc_uninit(self)
    }

    #[inline(always)]
    fn alloc_with<B: ErrorBehavior, T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<T>, B> {
        BumpScope::generic_alloc_with(self, f)
    }

    #[inline(always)]
    fn alloc_slice_copy<B: ErrorBehavior, T: Copy>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B> {
        BumpScope::generic_alloc_slice_copy(self, slice)
    }

    #[inline(always)]
    fn alloc_slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B> {
        BumpScope::generic_alloc_slice_clone(self, slice)
    }

    #[inline(always)]
    fn alloc_slice_fill<B: ErrorBehavior, T: Clone>(&self, len: usize, value: T) -> Result<BumpBox<[T]>, B> {
        BumpScope::generic_alloc_slice_fill(self, len, value)
    }

    #[inline(always)]
    fn alloc_slice_fill_with<B: ErrorBehavior, T>(&self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<[T]>, B> {
        BumpScope::generic_alloc_slice_fill_with(self, len, f)
    }

    #[inline(always)]
    fn alloc_str<B: ErrorBehavior>(&self, src: &str) -> Result<BumpBox<str>, B> {
        BumpScope::generic_alloc_str(self, src)
    }

    #[inline(always)]
    #[cfg(feature = "alloc")]
    fn alloc_fmt<B: ErrorBehavior>(&self, args: fmt::Arguments) -> Result<BumpBox<str>, B> {
        BumpScope::generic_alloc_fmt(self, args)
    }

    #[inline(always)]
    fn reserve_bytes<B: ErrorBehavior>(&self, additional: usize) -> Result<(), B> {
        BumpScope::generic_reserve_bytes(self, additional)
    }

    #[inline(always)]
    fn alloc_in_current_chunk(&self, layout: impl LayoutProps) -> Option<NonNull<u8>> {
        self.chunk.get().alloc::<MIN_ALIGN>(layout)
    }

    #[inline(always)]
    fn alloc_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        BumpScope::alloc_in_another_chunk(self, layout)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Sealed
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn alloc_uninit<B: ErrorBehavior, T>(&self) -> Result<BumpBox<MaybeUninit<T>>, B> {
        BumpScope::generic_alloc_uninit(self.as_scope())
    }

    #[inline(always)]
    fn alloc_with<B: ErrorBehavior, T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<T>, B> {
        BumpScope::generic_alloc_with(self.as_scope(), f)
    }

    #[inline(always)]
    fn alloc_slice_copy<B: ErrorBehavior, T: Copy>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B> {
        BumpScope::generic_alloc_slice_copy(self.as_scope(), slice)
    }

    #[inline(always)]
    fn alloc_slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<[T]>, B> {
        BumpScope::generic_alloc_slice_clone(self.as_scope(), slice)
    }

    #[inline(always)]
    fn alloc_slice_fill<B: ErrorBehavior, T: Clone>(&self, len: usize, value: T) -> Result<BumpBox<[T]>, B> {
        BumpScope::generic_alloc_slice_fill(self.as_scope(), len, value)
    }

    #[inline(always)]
    fn alloc_slice_fill_with<B: ErrorBehavior, T>(&self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<[T]>, B> {
        BumpScope::generic_alloc_slice_fill_with(self.as_scope(), len, f)
    }

    #[inline(always)]
    fn alloc_str<B: ErrorBehavior>(&self, src: &str) -> Result<BumpBox<str>, B> {
        BumpScope::generic_alloc_str(self.as_scope(), src)
    }

    #[inline(always)]
    #[cfg(feature = "alloc")]
    fn alloc_fmt<B: ErrorBehavior>(&self, args: fmt::Arguments) -> Result<BumpBox<str>, B> {
        BumpScope::generic_alloc_fmt(self.as_scope(), args)
    }

    #[inline(always)]
    fn reserve_bytes<B: ErrorBehavior>(&self, additional: usize) -> Result<(), B> {
        BumpScope::generic_reserve_bytes(self.as_scope(), additional)
    }

    #[inline(always)]
    fn alloc_in_current_chunk(&self, layout: impl LayoutProps) -> Option<NonNull<u8>> {
        <BumpScope<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> as Sealed>::alloc_in_current_chunk(self.as_scope(), layout)
    }

    #[inline(always)]
    fn alloc_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        BumpScope::alloc_in_another_chunk(self.as_scope(), layout)
    }
}

/// Implemented for any `Bump(Scope)` or reference thereof.
///
/// This is used as a bound for [`WithDrop`](crate::WithDrop).
#[allow(private_bounds)]
pub trait AnyBump: Sealed {}

impl<U: AnyBump> AnyBump for &U {}
impl<U: AnyBump> AnyBump for &mut U {}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AnyBump
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> AnyBump
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}
