use core::ptr::NonNull;

use crate::{alloc::Allocator, BaseAllocator, Bump, BumpScope, MinimumAlignment, SupportedMinimumAlignment};

mod bump_allocator;
pub(crate) mod bump_allocator_ext;
mod bump_allocator_scope;
pub(crate) mod bump_allocator_scope_ext;
mod mut_bump_allocator;
pub(crate) mod mut_bump_allocator_ext;
mod mut_bump_allocator_scope;
pub(crate) mod mut_bump_allocator_scope_ext;

pub use bump_allocator::BumpAllocator;
pub use bump_allocator_ext::BumpAllocatorExt;
pub use bump_allocator_scope::BumpAllocatorScope;
pub use bump_allocator_scope_ext::BumpAllocatorScopeExt;
pub use mut_bump_allocator::MutBumpAllocator;
pub use mut_bump_allocator_ext::MutBumpAllocatorExt;
pub use mut_bump_allocator_scope::MutBumpAllocatorScope;
pub use mut_bump_allocator_scope_ext::MutBumpAllocatorScopeExt;

macro_rules! assert_dyn_compatible {
    ($($tt:tt)*) => {
        const _: () = {
            #[allow(dead_code)]
            fn assert_dyn_compatible(_: &dyn $($tt)*) {}
        };
    };
}

pub(crate) use assert_dyn_compatible;

macro_rules! assert_implements {
    ([$($what:tt)*] $($ty:ty)*) => {
        const _: () = {
            #[allow(dead_code)]
            type A = crate::alloc::NoopAllocator;
            #[allow(dead_code)]
            type Bump = crate::Bump<A>;
            #[allow(dead_code)]
            type BumpScope<'a> = crate::BumpScope<'a, A>;
            #[allow(clippy::extra_unused_lifetimes)]
            const fn implements<'a, What: $($what)*>() {}
            $(
                #[allow(clippy::mut_mut)]
                implements::<$ty>();
            )*
        };
    };
}

pub(crate) use assert_implements;

assert_implements! {
    [Allocator + ?Sized]

    Bump
    &Bump
    &&Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &BumpScope
    &&BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

#[allow(clippy::mut_mut)]
unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for &mut &mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
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

#[allow(clippy::mut_mut)]
unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for &mut &mut BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
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
