use core::{alloc::Layout, ptr::NonNull};

use allocator_api2_02::alloc::{AllocError, Allocator};

#[cfg(feature = "alloc")]
use allocator_api2_02::boxed::Box;

use crate::{
    alloc::{box_like, AllocError as CrateAllocError, Allocator as CrateAllocator, BoxLike},
    BaseAllocator, Bump, BumpAllocator, BumpScope, MinimumAlignment, SupportedMinimumAlignment, WithoutDealloc,
    WithoutShrink,
};

impl From<AllocError> for CrateAllocError {
    fn from(_: AllocError) -> Self {
        CrateAllocError
    }
}

impl From<CrateAllocError> for AllocError {
    fn from(_: CrateAllocError) -> Self {
        AllocError
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <Self as CrateAllocator>::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for &mut BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <Self as CrateAllocator>::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <Self as CrateAllocator>::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for &mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <Self as CrateAllocator>::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into)
    }
}

unsafe impl<A: BumpAllocator> Allocator for WithoutShrink<A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <Self as CrateAllocator>::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into)
    }
}

unsafe impl<A: BumpAllocator> Allocator for WithoutDealloc<A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <Self as CrateAllocator>::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into)
    }
}

impl<T: ?Sized, A: Allocator> box_like::Sealed for Box<T, A> {
    type T = T;
    type A = A;

    unsafe fn from_raw_in(ptr: *mut Self::T, allocator: Self::A) -> Self {
        Box::from_raw_in(ptr, allocator)
    }
}

impl<T: ?Sized, A: Allocator> BoxLike for Box<T, A> {}
