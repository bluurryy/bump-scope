use core::{
    mem::transmute,
    ptr::{self, NonNull},
};

use crate::{
    error_behavior::ErrorBehavior,
    polyfill::{nonnull, transmute_mut, transmute_ref},
    raw_bump_box::RawBumpBox,
    BumpAllocator, FixedBumpVec, MutBumpAllocator, SizedTypeProperties,
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

        Ok(Self {
            initialized: RawBumpBox::from_ptr(nonnull::slice_from_raw_parts(ptr, 0)),
            capacity: len,
        })
    }

    #[inline(always)]
    pub(crate) unsafe fn prepare_allocation<B: ErrorBehavior>(
        allocator: &mut impl MutBumpAllocator,
        len: usize,
    ) -> Result<Self, B> {
        let allocation = B::prepare_slice_allocation::<T>(allocator, len)?;

        Ok(Self {
            initialized: RawBumpBox::from_ptr(nonnull::slice_from_raw_parts(nonnull::as_non_null_ptr(allocation), 0)),
            capacity: allocation.len(),
        })
    }

    /// `new_cap` must be greater than `self.capacity`
    pub(crate) unsafe fn grow_prepared_allocation<B: ErrorBehavior>(
        &mut self,
        allocator: &mut impl MutBumpAllocator,
        minimum_new_cap: usize,
    ) -> Result<(), B> {
        debug_assert!(minimum_new_cap > self.capacity);
        let allocation = B::prepare_slice_allocation::<T>(allocator, minimum_new_cap)?;

        let new_ptr = allocation.cast::<T>();
        let new_cap = allocation.len();

        ptr::copy_nonoverlapping(self.as_ptr(), new_ptr.as_ptr(), self.len());

        self.initialized.set_ptr(new_ptr);
        self.capacity = new_cap;

        Ok(())
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
