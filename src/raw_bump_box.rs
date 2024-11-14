use core::{
    marker::PhantomData,
    mem::{self, transmute},
    ptr::{self, NonNull},
};

use crate::{
    polyfill::{nonnull, transmute_mut, transmute_ref},
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
    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) unsafe fn cook<'a>(self) -> BumpBox<'a, T> {
        transmute(self)
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) unsafe fn cook_ref<'a>(&self) -> &BumpBox<'a, T> {
        transmute_ref(self)
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) unsafe fn cook_mut<'a>(&mut self) -> &mut BumpBox<'a, T> {
        transmute_mut(self)
    }

    #[inline(always)]
    pub(crate) unsafe fn from_cooked(cooked: BumpBox<'_, T>) -> Self {
        Self {
            ptr: cooked.into_raw(),
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) const unsafe fn from_ptr(ptr: NonNull<T>) -> Self {
        Self {
            ptr,
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) fn into_ptr(self) -> NonNull<T> {
        let ptr = unsafe { ptr::read(&self.ptr) };
        mem::forget(self);
        ptr
    }

    #[must_use]
    #[inline(always)]
    pub const fn as_non_null_ptr(&self) -> NonNull<T> {
        self.ptr
    }
}

impl<T> RawBumpBox<[T]> {
    pub(crate) const EMPTY: Self = Self {
        ptr: nonnull::slice_from_raw_parts(NonNull::dangling(), 0),
        marker: PhantomData,
    };

    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.ptr.len()
    }

    #[inline(always)]
    pub(crate) unsafe fn set_ptr(&mut self, new_ptr: NonNull<T>) {
        nonnull::set_ptr(&mut self.ptr, new_ptr);
    }

    #[inline(always)]
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        nonnull::set_len(&mut self.ptr, new_len);
    }
}

impl RawBumpBox<str> {
    pub(crate) const EMPTY_STR: Self = Self {
        ptr: nonnull::str_from_utf8(nonnull::slice_from_raw_parts(NonNull::dangling(), 0)),
        marker: PhantomData,
    };

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        nonnull::str_bytes(self.ptr).len()
    }

    #[inline(always)]
    pub(crate) unsafe fn set_ptr(&mut self, new_ptr: NonNull<u8>) {
        let len = self.len();
        self.ptr = nonnull::str_from_utf8(nonnull::slice_from_raw_parts(new_ptr, len));
    }

    #[inline(always)]
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        let ptr = self.ptr.cast::<u8>();
        self.ptr = nonnull::str_from_utf8(nonnull::slice_from_raw_parts(ptr, new_len));
    }
}