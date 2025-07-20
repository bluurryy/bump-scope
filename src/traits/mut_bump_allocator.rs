use core::ptr::NonNull;

use crate::{
    alloc::Allocator, traits::assert_implements, BaseAllocator, Bump, BumpAllocator, BumpScope, MinimumAlignment,
    SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};

/// A marker trait for [`BumpAllocator`]s who have exclusive access to allocation.
pub unsafe trait MutBumpAllocator: BumpAllocator {}

assert_implements! {
    [MutBumpAllocator + ?Sized]

    Bump
    &mut Bump

    BumpScope
    &mut BumpScope

    dyn MutBumpAllocator
    &mut dyn MutBumpAllocator
}

unsafe impl Allocator for &mut (dyn MutBumpAllocator + '_) {
    #[inline(always)]
    fn allocate(&self, layout: core::alloc::Layout) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).allocate(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: core::alloc::Layout) {
        (**self).deallocate(ptr, layout);
    }

    #[inline(always)]
    fn allocate_zeroed(&self, layout: core::alloc::Layout) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).allocate_zeroed(layout)
    }

    #[inline(always)]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).grow(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).grow_zeroed(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).shrink(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    fn by_ref(&self) -> &Self
    where
        Self: Sized,
    {
        self
    }
}

unsafe impl<A: MutBumpAllocator + ?Sized> MutBumpAllocator for &mut A where for<'a> &'a mut A: Allocator {}

unsafe impl<A: MutBumpAllocator> MutBumpAllocator for WithoutDealloc<A> {}
unsafe impl<A: MutBumpAllocator> MutBumpAllocator for WithoutShrink<A> {}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> MutBumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}
