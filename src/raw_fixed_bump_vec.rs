use core::mem::transmute;

use crate::{
    polyfill::{transmute_mut, transmute_ref},
    raw_bump_box::RawBumpBox,
    FixedBumpVec,
};

/// Like [`FixedBumpVec`] but without its lifetime.
#[repr(C)]
pub struct RawFixedBumpVec<T> {
    pub(crate) initialized: RawBumpBox<[T]>,
    pub(crate) capacity: usize,
}

impl<T> RawFixedBumpVec<T> {
    #[inline(always)]
    pub(crate) unsafe fn cook<'a>(self) -> FixedBumpVec<'a, T> {
        transmute(self)
    }

    #[inline(always)]
    pub(crate) unsafe fn cook_ref<'a>(&self) -> &FixedBumpVec<'a, T> {
        transmute_ref(self)
    }

    #[inline(always)]
    pub(crate) unsafe fn cook_mut<'a>(&mut self) -> &mut FixedBumpVec<'a, T> {
        transmute_mut(self)
    }
}
