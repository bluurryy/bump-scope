use core::ptr::NonNull;

use crate::{
    alloc::Allocator,
    traits::{assert_dyn_compatible, assert_implements},
    BaseAllocator, Bump, BumpAllocator, BumpScope, MinimumAlignment, MutBumpAllocatorScope, SupportedMinimumAlignment,
    WithoutDealloc, WithoutShrink,
};

/// A bump allocator scope.
///
/// This is a [`BumpAllocator`] which can make allocations that outlive itself.
/// Specifically, its allocations live for the lifetime `'a`.
///
/// # Safety
///
/// This trait must only be implemented when allocations live for `'a`.
/// For example this function must be sound:
///
/// ```
/// # #![allow(dead_code)]
/// use bump_scope::BumpAllocatorScope;
/// use core::alloc::Layout;
///
/// fn allocate_zeroed_bytes<'a>(allocator: impl BumpAllocatorScope<'a>, len: usize) -> &'a [u8] {
///     let layout = Layout::array::<u8>(len).unwrap();
///     let ptr = allocator.allocate_zeroed(layout).unwrap();
///     unsafe { ptr.as_ref() }
/// }
/// ```
pub unsafe trait BumpAllocatorScope<'a>: BumpAllocator {}

assert_dyn_compatible!(BumpAllocatorScope<'_>);

assert_implements! {
    [BumpAllocatorScope<'a> + ?Sized]

    BumpScope

    &Bump
    &BumpScope

    &mut Bump
    &mut BumpScope

    dyn BumpAllocatorScope
    &dyn BumpAllocatorScope
    &mut dyn BumpAllocatorScope

    dyn MutBumpAllocatorScope
    &dyn MutBumpAllocatorScope
    &mut dyn MutBumpAllocatorScope
}

unsafe impl Allocator for &mut (dyn BumpAllocatorScope<'_> + '_) {
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

unsafe impl<'a, B: BumpAllocatorScope<'a> + ?Sized> BumpAllocatorScope<'a> for &B {}
unsafe impl<'a, B: BumpAllocatorScope<'a> + ?Sized> BumpAllocatorScope<'a> for &mut B where for<'b> &'b mut B: Allocator {}

unsafe impl<'a, B: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for WithoutDealloc<B> {}
unsafe impl<'a, B: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for WithoutShrink<B> {}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for &'a Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for &'a mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}
