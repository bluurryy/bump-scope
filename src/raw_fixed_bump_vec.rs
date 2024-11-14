use core::{mem::transmute, ptr::NonNull};

use crate::{
    error_behavior::ErrorBehavior,
    polyfill::{nonnull, transmute_mut, transmute_ref},
    raw_bump_box::RawBumpBox,
    BumpAllocator, FixedBumpVec, SizedTypeProperties,
};

/// Like [`FixedBumpVec`] but without its lifetime.
#[repr(C)]
pub struct RawFixedBumpVec<T> {
    pub(crate) initialized: RawBumpBox<[T]>,
    pub(crate) capacity: usize,
}

impl<T> RawFixedBumpVec<T> {
    #[inline(always)]
    pub(crate) const unsafe fn cook<'a>(self) -> FixedBumpVec<'a, T> {
        transmute(self)
    }

    #[inline(always)]
    pub(crate) const unsafe fn cook_ref<'a>(&self) -> &FixedBumpVec<'a, T> {
        transmute_ref(self)
    }

    #[inline(always)]
    pub(crate) unsafe fn cook_mut<'a>(&mut self) -> &mut FixedBumpVec<'a, T> {
        transmute_mut(self)
    }

    #[inline(always)]
    pub(crate) unsafe fn from_cooked(cooked: FixedBumpVec<'_, T>) -> Self {
        let (initialized, capacity) = cooked.into_raw_parts();
        let initialized = RawBumpBox::from_cooked(initialized);
        Self { initialized, capacity }
    }

    pub(crate) const EMPTY: Self = RawFixedBumpVec {
        initialized: RawBumpBox::EMPTY,
        capacity: if T::IS_ZST { usize::MAX } else { 0 },
    };

    #[inline(always)]
    pub(crate) unsafe fn allocate<B: ErrorBehavior>(allocator: &impl BumpAllocator, len: usize) -> Result<Self, B> {
        let ptr = B::allocate_slice::<T>(allocator, len)?;
        let initialized = RawBumpBox::from_ptr(nonnull::slice_from_raw_parts(ptr, 0));
        Ok(Self {
            initialized,
            capacity: len,
        })
    }

    #[inline(always)]
    pub(crate) fn as_ptr(&self) -> *const T {
        self.initialized.as_non_null_ptr().as_ptr().cast()
    }

    #[inline(always)]
    pub(crate) fn as_mut_ptr(&mut self) -> *mut T {
        self.initialized.as_non_null_ptr().as_ptr().cast()
    }

    #[must_use]
    #[inline(always)]
    #[allow(dead_code)]
    pub fn as_non_null_ptr(&self) -> NonNull<T> {
        self.initialized.as_non_null_ptr().cast()
    }

    #[inline(always)]
    pub(crate) const fn len(&self) -> usize {
        self.initialized.as_non_null_ptr().len()
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) unsafe fn set_ptr(&mut self, new_ptr: NonNull<T>) {
        self.initialized.set_ptr(new_ptr);
    }

    #[inline(always)]
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        self.initialized.set_len(new_len);
    }
}