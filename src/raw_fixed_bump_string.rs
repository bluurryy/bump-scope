use core::{mem::transmute, ptr::NonNull};

use crate::{
    error_behavior::ErrorBehavior,
    polyfill::{nonnull, transmute_mut, transmute_ref},
    raw_bump_box::RawBumpBox,
    BumpAllocator, FixedBumpString, MutBumpAllocator,
};

/// Like [`FixedBumpVec`] but without its lifetime.
#[repr(C)]
pub struct RawFixedBumpString {
    initialized: RawBumpBox<str>,
    capacity: usize,
}

impl RawFixedBumpString {
    pub(crate) const EMPTY: Self = RawFixedBumpString {
        initialized: RawBumpBox::EMPTY_STR,
        capacity: 0,
    };

    #[inline(always)]
    pub(crate) const unsafe fn cook<'a>(self) -> FixedBumpString<'a> {
        transmute(self)
    }

    #[inline(always)]
    pub(crate) const unsafe fn cook_ref<'a>(&self) -> &FixedBumpString<'a> {
        transmute_ref(self)
    }

    #[inline(always)]
    pub(crate) unsafe fn cook_mut<'a>(&mut self) -> &mut FixedBumpString<'a> {
        transmute_mut(self)
    }

    #[inline(always)]
    pub(crate) unsafe fn from_cooked(cooked: FixedBumpString<'_>) -> Self {
        let capacity = cooked.capacity();
        let initialized = cooked.into_boxed_str();
        let initialized = RawBumpBox::from_cooked(initialized);
        Self { initialized, capacity }
    }

    #[inline(always)]
    pub(crate) unsafe fn allocate<B: ErrorBehavior>(allocator: &impl BumpAllocator, len: usize) -> Result<Self, B> {
        let ptr = B::allocate_slice::<u8>(allocator, len)?;
        let initialized = RawBumpBox::from_ptr(nonnull::str_from_utf8(nonnull::slice_from_raw_parts(ptr, 0)));
        Ok(Self {
            initialized,
            capacity: len,
        })
    }

    #[inline(always)]
    pub(crate) unsafe fn prepare_allocation<B: ErrorBehavior>(
        allocator: &mut impl MutBumpAllocator,
        len: usize,
    ) -> Result<Self, B> {
        let allocation = B::prepare_slice_allocation::<u8>(allocator, len)?;
        let initialized = RawBumpBox::from_ptr(nonnull::str_from_utf8(nonnull::slice_from_raw_parts(
            nonnull::as_non_null_ptr(allocation),
            0,
        )));

        Ok(Self {
            initialized,
            capacity: allocation.len(),
        })
    }

    #[inline(always)]
    pub(crate) const fn len(&self) -> usize {
        nonnull::str_bytes(self.initialized.as_non_null_ptr()).len()
    }

    #[inline(always)]
    pub(crate) const fn capacity(&self) -> usize {
        self.capacity
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn as_ptr(&self) -> *const u8 {
        self.initialized.as_non_null_ptr().as_ptr().cast()
    }

    #[inline(always)]
    pub(crate) fn as_mut_ptr(&mut self) -> *mut u8 {
        self.initialized.as_non_null_ptr().as_ptr().cast()
    }

    #[must_use]
    #[inline(always)]
    #[allow(dead_code)]
    pub fn as_non_null_ptr(&self) -> NonNull<u8> {
        self.initialized.as_non_null_ptr().cast()
    }

    #[must_use]
    #[inline(always)]
    #[allow(dead_code)]
    pub fn as_non_null_str(&self) -> NonNull<str> {
        self.initialized.as_non_null_ptr()
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) unsafe fn set_ptr(&mut self, new_ptr: NonNull<u8>) {
        self.initialized.set_ptr(new_ptr);
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        self.initialized.set_len(new_len);
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub(crate) unsafe fn set_cap(&mut self, new_cap: usize) {
        self.capacity = new_cap;
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub(crate) fn into_raw_parts(self) -> (NonNull<str>, usize) {
        let Self { initialized, capacity } = self;
        (initialized.into_ptr(), capacity)
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub(crate) unsafe fn from_raw_parts(slice: NonNull<str>, capacity: usize) -> Self {
        Self {
            initialized: RawBumpBox::from_ptr(slice),
            capacity,
        }
    }
}
