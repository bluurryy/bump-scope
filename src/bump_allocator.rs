#![allow(private_bounds, missing_docs, unused_variables, clippy::missing_errors_doc)]

use crate::{
    error_behavior::ErrorBehavior, BaseAllocator, Bump, BumpBox, BumpScope, FixedBumpVec, MinimumAlignment,
    SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};
use allocator_api2::alloc::Allocator;

/// An allocator that allows `grow(_zeroed)`, `shrink` and `deallocate` calls with pointers that were not allocated by this allocator.
/// This trait is used for [`BumpBox::into_box`](BumpBox::into_box) to allow safely converting a `BumpBox` into a `Box`.
///
/// The allocations made with this allocator will have a lifetime of `'a`.
///
/// # Safety
/// - `grow(_zeroed)`, `shrink` and `deallocate` must be ok to be called with a pointer that was not allocated by this Allocator
pub unsafe trait BumpAllocator<'a>: Allocator {
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B>;
    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B>;
    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>);
    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B>;
}

unsafe impl<'a, A> BumpAllocator<'a> for &A
where
    A: BumpAllocator<'a>,
{
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        A::vec_alloc(self, capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        A::vec_grow(self, fixed, additional)
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        A::vec_shrink_to_fit(self, fixed);
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        A::slice_clone(self, slice)
    }
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator<'a>
    for &'a Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        self.as_scope().vec_alloc(capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        self.as_scope().vec_grow(fixed, additional)
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        self.as_scope().vec_shrink_to_fit(fixed);
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        self.as_scope().slice_clone(slice)
    }
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator<'a>
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        self.generic_alloc_fixed_vec(capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        self.generic_grow_vec(fixed, additional)
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        self.generic_shrink_vec_to_fit(fixed);
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        self.generic_alloc_slice_clone(slice)
    }
}

unsafe impl<'a, A: BumpAllocator<'a>> BumpAllocator<'a> for WithoutDealloc<A> {
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        A::vec_alloc(&self.0, capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        A::vec_grow(&self.0, fixed, additional)
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        A::vec_shrink_to_fit(&self.0, fixed);
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        A::slice_clone(&self.0, slice)
    }
}

unsafe impl<'a, A: BumpAllocator<'a>> BumpAllocator<'a> for WithoutShrink<A> {
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        A::vec_alloc(&self.0, capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        A::vec_grow(&self.0, fixed, additional)
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        A::vec_shrink_to_fit(&self.0, fixed);
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        A::slice_clone(&self.0, slice)
    }
}
