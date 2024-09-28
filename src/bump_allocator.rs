#![allow(
    private_bounds,
    missing_docs,
    unused_variables,
    clippy::missing_errors_doc,
    clippy::needless_lifetimes,
    clippy::missing_safety_doc
)]

use core::{marker::PhantomData, mem::transmute, ptr::NonNull};

use crate::{
    error_behavior::ErrorBehavior, fixed_bump_vec::RawFixedBumpVec, BaseAllocator, Bump, BumpBox, BumpScope,
    MinimumAlignment, SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};
use allocator_api2::alloc::Allocator;

/// An allocator that allows `grow(_zeroed)`, `shrink` and `deallocate` calls with pointers that were not allocated by this allocator.
/// This trait is used for [`BumpBox::into_box`](BumpBox::into_box) to allow safely converting a `BumpBox` into a `Box`.
///
/// The allocations made with this allocator will have a lifetime of `'a`.
///
/// # Safety
/// - `grow(_zeroed)`, `shrink` and `deallocate` must be ok to be called with a pointer that was not allocated by this Allocator
pub unsafe trait BumpAllocator: Allocator {
    type Lifetime;

    fn alloc_vec<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<NonNull<T>, B>;

    unsafe fn grow_vec<B: ErrorBehavior, T>(&self, vec: &mut RawFixedBumpVec<T>, additional: usize) -> Result<(), B>;
    unsafe fn raw_shrink_vec_to_fit<T>(&self, fixed: &mut RawFixedBumpVec<T>);
    unsafe fn raw_clone_slice<'x, B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'x, [T]>, B>;
}

#[cfg(not(no_global_oom_handling))]
pub(crate) unsafe trait BumpAllocatorExt<'a>: BumpAllocator<Lifetime = LifetimeMarker<'a>> {
    fn clone_slice<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        unsafe { self.raw_clone_slice(slice) }
    }
}

#[cfg(not(no_global_oom_handling))]
unsafe impl<'a, A> BumpAllocatorExt<'a> for A where A: BumpAllocator<Lifetime = LifetimeMarker<'a>> {}

pub struct LifetimeMarker<'a>(PhantomData<&'a ()>);

unsafe impl<'a, A> BumpAllocator for &A
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
    type Lifetime = LifetimeMarker<'a>;

    fn alloc_vec<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<NonNull<T>, B> {
        A::alloc_vec(self, capacity)
    }

    unsafe fn grow_vec<B: ErrorBehavior, T>(&self, vec: &mut RawFixedBumpVec<T>, additional: usize) -> Result<(), B> {
        A::grow_vec(self, vec, additional)
    }

    unsafe fn raw_shrink_vec_to_fit<T>(&self, fixed: &mut RawFixedBumpVec<T>) {
        A::raw_shrink_vec_to_fit(self, fixed);
    }

    unsafe fn raw_clone_slice<'x, B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'x, [T]>, B> {
        A::raw_clone_slice(self, slice)
    }
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for &'a Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Lifetime = LifetimeMarker<'a>;

    fn alloc_vec<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<NonNull<T>, B> {
        self.as_scope().alloc_vec(capacity)
    }

    unsafe fn grow_vec<B: ErrorBehavior, T>(&self, vec: &mut RawFixedBumpVec<T>, additional: usize) -> Result<(), B> {
        self.as_scope().grow_vec(vec, additional)
    }

    unsafe fn raw_shrink_vec_to_fit<T>(&self, fixed: &mut RawFixedBumpVec<T>) {
        self.as_scope().raw_shrink_vec_to_fit(fixed);
    }

    unsafe fn raw_clone_slice<'x, B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'x, [T]>, B> {
        self.as_scope().raw_clone_slice(slice)
    }
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Lifetime = LifetimeMarker<'a>;

    fn alloc_vec<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<NonNull<T>, B> {
        Ok(self.generic_alloc_uninit_slice::<B, T>(capacity)?.into_raw().cast())
    }

    unsafe fn grow_vec<B: ErrorBehavior, T>(&self, vec: &mut RawFixedBumpVec<T>, additional: usize) -> Result<(), B> {
        self.generic_grow_vec(vec, additional)
    }

    unsafe fn raw_shrink_vec_to_fit<T>(&self, fixed: &mut RawFixedBumpVec<T>) {
        self.generic_shrink_vec_to_fit(fixed.as_mut_cooked());
    }

    unsafe fn raw_clone_slice<'x, B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'x, [T]>, B> {
        self.generic_alloc_slice_clone(slice)
            .map(|boxed| unsafe { transmute::<BumpBox<'a, [T]>, BumpBox<'x, [T]>>(boxed) })
    }
}

unsafe impl<'a, A> BumpAllocator for WithoutDealloc<A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
    type Lifetime = LifetimeMarker<'a>;

    fn alloc_vec<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<NonNull<T>, B> {
        A::alloc_vec(&self.0, capacity)
    }

    unsafe fn grow_vec<B: ErrorBehavior, T>(&self, vec: &mut RawFixedBumpVec<T>, additional: usize) -> Result<(), B> {
        A::grow_vec(&self.0, vec, additional)
    }

    unsafe fn raw_shrink_vec_to_fit<T>(&self, fixed: &mut RawFixedBumpVec<T>) {
        A::raw_shrink_vec_to_fit(&self.0, fixed);
    }

    unsafe fn raw_clone_slice<'x, B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'x, [T]>, B> {
        A::raw_clone_slice(&self.0, slice)
    }
}

unsafe impl<'a, A> BumpAllocator for WithoutShrink<A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
    type Lifetime = LifetimeMarker<'a>;

    fn alloc_vec<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<NonNull<T>, B> {
        A::alloc_vec(&self.0, capacity)
    }

    unsafe fn grow_vec<'x, B: ErrorBehavior, T>(&self, vec: &mut RawFixedBumpVec<T>, additional: usize) -> Result<(), B> {
        A::grow_vec(&self.0, vec, additional)
    }

    unsafe fn raw_shrink_vec_to_fit<T>(&self, fixed: &mut RawFixedBumpVec<T>) {
        A::raw_shrink_vec_to_fit(&self.0, fixed);
    }

    unsafe fn raw_clone_slice<'x, B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'x, [T]>, B> {
        A::raw_clone_slice(&self.0, slice)
    }
}
