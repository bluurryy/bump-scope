use crate::{
    BaseAllocator, Bump, BumpAllocator, BumpScope, WithoutDealloc, WithoutShrink,
    settings::BumpAllocatorSettings,
    traits::{assert_dyn_compatible, assert_implements},
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
/// # #![expect(dead_code)]
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

unsafe impl<'a, B: BumpAllocatorScope<'a> + ?Sized> BumpAllocatorScope<'a> for &B {}
unsafe impl<'a, B: BumpAllocatorScope<'a> + ?Sized> BumpAllocatorScope<'a> for &mut B {}

unsafe impl<'a, B: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for WithoutDealloc<B> {}
unsafe impl<'a, B: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for WithoutShrink<B> {}

unsafe impl<'a, A, S> BumpAllocatorScope<'a> for BumpScope<'a, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
}

unsafe impl<'a, A, S> BumpAllocatorScope<'a> for &'a Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
}

unsafe impl<'a, A, S> BumpAllocatorScope<'a> for &'a mut Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
}
