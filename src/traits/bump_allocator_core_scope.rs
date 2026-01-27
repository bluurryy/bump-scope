use crate::{
    Bump, BumpScope, WithoutDealloc, WithoutShrink,
    alloc::Allocator,
    settings::BumpAllocatorSettings,
    traits::{BumpAllocatorCore, assert_dyn_compatible, assert_implements},
};

/// A bump allocator scope.
///
/// This is a [`BumpAllocatorCore`] which can make allocations that outlive itself.
/// Specifically, its allocations live for the lifetime `'a`.
///
/// # Safety
///
/// This trait must only be implemented when allocations live for `'a`.
/// For example this function must be sound:
///
/// ```
/// # #![expect(dead_code)]
/// use bump_scope::traits::BumpAllocatorCoreScope;
/// use core::alloc::Layout;
///
/// fn allocate_zeroed_bytes<'a>(allocator: impl BumpAllocatorCoreScope<'a>, len: usize) -> &'a [u8] {
///     let layout = Layout::array::<u8>(len).unwrap();
///     let ptr = allocator.allocate_zeroed(layout).unwrap();
///     unsafe { ptr.as_ref() }
/// }
/// ```
pub unsafe trait BumpAllocatorCoreScope<'a>: BumpAllocatorCore {}

assert_dyn_compatible!(BumpAllocatorCoreScope<'_>);

assert_implements! {
    [BumpAllocatorCoreScope<'a> + ?Sized]

    BumpScope

    &Bump
    &BumpScope

    &mut Bump
    &mut BumpScope

    dyn BumpAllocatorCoreScope
    &dyn BumpAllocatorCoreScope
    &mut dyn BumpAllocatorCoreScope

    dyn MutBumpAllocatorCoreScope
    &dyn MutBumpAllocatorCoreScope
    &mut dyn MutBumpAllocatorCoreScope
}

unsafe impl<'a, B: BumpAllocatorCoreScope<'a> + ?Sized> BumpAllocatorCoreScope<'a> for &B {}
unsafe impl<'a, B: BumpAllocatorCoreScope<'a> + ?Sized> BumpAllocatorCoreScope<'a> for &mut B {}

unsafe impl<'a, B: BumpAllocatorCoreScope<'a>> BumpAllocatorCoreScope<'a> for WithoutDealloc<B> {}
unsafe impl<'a, B: BumpAllocatorCoreScope<'a>> BumpAllocatorCoreScope<'a> for WithoutShrink<B> {}

unsafe impl<'a, A, S> BumpAllocatorCoreScope<'a> for BumpScope<'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
}

unsafe impl<'a, A, S> BumpAllocatorCoreScope<'a> for &'a Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
}

unsafe impl<'a, A, S> BumpAllocatorCoreScope<'a> for &'a mut Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
}
