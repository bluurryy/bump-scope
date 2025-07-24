use core::{mem::transmute, ptr::NonNull};

use crate::{
    error_behavior::ErrorBehavior,
    polyfill::{non_null, transmute_mut, transmute_ref},
    raw_bump_box::RawBumpBox,
    BumpAllocatorExt, FixedBumpString, MutBumpAllocatorExt,
};

/// Like [`FixedBumpVec`](crate::FixedBumpVec) but without its lifetime.
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
        unsafe { transmute(self) }
    }

    #[inline(always)]
    pub(crate) const unsafe fn cook_ref<'a>(&self) -> &FixedBumpString<'a> {
        unsafe { transmute_ref(self) }
    }

    #[inline(always)]
    pub(crate) unsafe fn cook_mut<'a>(&mut self) -> &mut FixedBumpString<'a> {
        unsafe { transmute_mut(self) }
    }

    #[inline(always)]
    pub(crate) unsafe fn from_cooked(cooked: FixedBumpString<'_>) -> Self {
        let capacity = cooked.capacity();
        let initialized = cooked.into_boxed_str();
        let initialized = unsafe { RawBumpBox::from_cooked(initialized) };
        Self { initialized, capacity }
    }

    #[inline(always)]
    pub(crate) unsafe fn allocate<B: ErrorBehavior>(allocator: &impl BumpAllocatorExt, len: usize) -> Result<Self, B> {
        let ptr = B::allocate_slice::<u8>(allocator, len)?;
        let initialized = unsafe { RawBumpBox::from_ptr(non_null::str_from_utf8(NonNull::slice_from_raw_parts(ptr, 0))) };
        Ok(Self {
            initialized,
            capacity: len,
        })
    }

    #[inline(always)]
    pub(crate) unsafe fn prepare_allocation<B: ErrorBehavior>(
        allocator: &mut impl MutBumpAllocatorExt,
        len: usize,
    ) -> Result<Self, B> {
        let allocation = B::prepare_slice_allocation::<u8>(allocator, len)?;
        let initialized = RawBumpBox::from_ptr(non_null::str_from_utf8(NonNull::slice_from_raw_parts(
            non_null::as_non_null_ptr(allocation),
            0,
        )));

        Ok(Self {
            initialized,
            capacity: allocation.len(),
        })
    }

    #[inline(always)]
    pub(crate) const fn len(&self) -> usize {
        non_null::str_bytes(self.initialized.as_non_null()).len()
    }

    #[inline(always)]
    pub(crate) const fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline(always)]
    pub(crate) fn as_ptr(&self) -> *const u8 {
        self.initialized.as_non_null().as_ptr().cast()
    }

    #[inline(always)]
    pub(crate) fn as_mut_ptr(&mut self) -> *mut u8 {
        self.initialized.as_non_null().as_ptr().cast()
    }

    #[must_use]
    #[inline(always)]
    pub const fn as_non_null(&self) -> NonNull<u8> {
        self.initialized.as_non_null().cast()
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) unsafe fn set_ptr(&mut self, new_ptr: NonNull<u8>) {
        unsafe { self.initialized.set_ptr(new_ptr) };
    }

    #[inline(always)]
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        unsafe { self.initialized.set_len(new_len) };
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
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) unsafe fn from_raw_parts(slice: NonNull<str>, capacity: usize) -> Self {
        Self {
            initialized: unsafe { RawBumpBox::from_ptr(slice) },
            capacity,
        }
    }
}
