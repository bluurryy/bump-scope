use core::{alloc::Layout, ptr::NonNull};

use allocator_api2_02::alloc::{AllocError, Allocator};

#[cfg(feature = "alloc")]
use allocator_api2_02::boxed::Box;

use crate::{
    alloc::{box_like, AllocError as CrateAllocError, Allocator as CrateAllocator, BoxLike},
    polyfill, BaseAllocator, Bump, BumpAllocator, BumpScope, MinimumAlignment, SupportedMinimumAlignment, WithoutDealloc,
    WithoutShrink,
};

/// Wrap an <code>allocator_api2::[Allocator](Allocator)</code> to implement <code>bump_scope::[Allocator](CrateAllocator)</code>.
#[repr(transparent)]
pub struct AllocatorApi2V02Compat<A: ?Sized>(pub A);

impl<A: ?Sized> AllocatorApi2V02Compat<A> {
    #[allow(missing_docs)]
    pub fn from_ref(allocator: &A) -> &Self {
        unsafe { &*(polyfill::pointer::from_ref(allocator) as *const Self) }
    }

    #[allow(missing_docs)]
    pub fn from_mut(allocator: &mut A) -> &mut Self {
        unsafe { &mut *(polyfill::pointer::from_mut(allocator) as *mut Self) }
    }
}

unsafe impl<A: ?Sized + Allocator> CrateAllocator for AllocatorApi2V02Compat<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, CrateAllocError> {
        <A as Allocator>::allocate(&self.0, layout).map_err(Into::into)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <A as Allocator>::deallocate(&self.0, ptr, layout);
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, CrateAllocError> {
        <A as Allocator>::allocate_zeroed(&self.0, layout).map_err(Into::into)
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, CrateAllocError> {
        <A as Allocator>::grow(&self.0, ptr, old_layout, new_layout).map_err(Into::into)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, CrateAllocError> {
        <A as Allocator>::grow_zeroed(&self.0, ptr, old_layout, new_layout).map_err(Into::into)
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, CrateAllocError> {
        <A as Allocator>::shrink(&self.0, ptr, old_layout, new_layout).map_err(Into::into)
    }

    fn by_ref(&self) -> &Self
    where
        Self: Sized,
    {
        self
    }
}

unsafe impl<A: ?Sized + CrateAllocator> Allocator for AllocatorApi2V02Compat<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <A as CrateAllocator>::allocate(&self.0, layout).map_err(Into::into)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <A as CrateAllocator>::deallocate(&self.0, ptr, layout);
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <A as CrateAllocator>::allocate_zeroed(&self.0, layout).map_err(Into::into)
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <A as CrateAllocator>::grow(&self.0, ptr, old_layout, new_layout).map_err(Into::into)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        <A as CrateAllocator>::grow_zeroed(&self.0, ptr, old_layout, new_layout).map_err(Into::into)
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <A as CrateAllocator>::shrink(&self.0, ptr, old_layout, new_layout).map_err(Into::into)
    }

    fn by_ref(&self) -> &Self
    where
        Self: Sized,
    {
        self
    }
}

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
