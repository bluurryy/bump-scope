use core::{marker::PhantomData, mem::transmute, ptr::NonNull};

use crate::{
    polyfill::{transmute_mut, transmute_ref},
    BumpBox,
};

/// Like [`BumpBox`] but without its lifetime.
#[repr(transparent)]
pub struct RawBumpBox<T: ?Sized> {
    pub(crate) ptr: NonNull<T>,

    /// Marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    marker: PhantomData<T>,
}

unsafe impl<T: ?Sized + Send> Send for RawBumpBox<T> {}
unsafe impl<T: ?Sized + Sync> Sync for RawBumpBox<T> {}

impl<T: ?Sized> Drop for RawBumpBox<T> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { self.ptr.as_ptr().drop_in_place() }
    }
}

impl<T: ?Sized> RawBumpBox<T> {
    #[inline(always)]
    pub(crate) unsafe fn cook<'a>(self) -> BumpBox<'a, T> {
        transmute(self)
    }

    #[inline(always)]
    pub(crate) unsafe fn cook_ref<'a>(&self) -> &BumpBox<'a, T> {
        transmute_ref(self)
    }

    #[inline(always)]
    pub(crate) unsafe fn cook_mut<'a>(&mut self) -> &mut BumpBox<'a, T> {
        transmute_mut(self)
    }
}
