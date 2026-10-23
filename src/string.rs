//! Minimal string implementation for internal usage.
//! Would be replaced by `alloc::String` if that becomes generic over its allocator.
//!
//! For now we add a `ErrorBehavior` generic, just because it makes for simpler code.

use core::{ffi::CStr, fmt, marker::PhantomData};

use alloc::vec::Vec;

use crate::{
    BoxU, BumpAllocator, BumpAllocatorScope,
    error_behavior::ErrorBehavior,
    extra::{box_from_utf8_unchecked, box_leak, box_remove_allocator},
};

#[repr(transparent)]
pub(crate) struct String<A: BumpAllocator, E: ErrorBehavior> {
    vec: Vec<u8, A>,
    marker: PhantomData<fn() -> E>,
}

impl<'a, A: BumpAllocatorScope<'a>, E: ErrorBehavior> String<A, E> {
    pub(crate) fn new(alloc: A) -> Self {
        Self {
            vec: Vec::new_in(alloc),
            marker: PhantomData,
        }
    }

    pub(crate) fn new_from_vec(mut vec: Vec<u8, A>) -> Self {
        // no bytes is valid utf8
        vec.clear();

        Self {
            vec,
            marker: PhantomData,
        }
    }

    pub(crate) fn reserve_if_fallible(&mut self, additional: usize) -> Result<(), E> {
        E::vec_reserve_if_fallible(&mut self.vec, additional)
    }

    pub(crate) fn push_str(&mut self, str: &str) -> Result<(), E> {
        self.reserve_if_fallible(str.len())?;
        self.vec.extend_from_slice(str.as_bytes());
        Ok(())
    }

    pub(crate) fn write_fmt(&mut self, args: fmt::Arguments) -> Result<(), E> {
        fmt::Write::write_fmt(self, args).map_err(|_| {
            // Either an allocation failed or the `fmt()` implementor returned an error,
            // in both cases we return an `AllocError`.
            //
            // Note that `fmt()` implementors *should* not return errors (see `std::fmt::Error`).
            // So it's fine to not differentiate from an `AllocError`.
            E::format_trait_error()
        })
    }

    pub(crate) fn into_bytes(self) -> BoxU<'a, [u8]> {
        box_remove_allocator(self.vec.into_boxed_slice()).0
    }

    pub(crate) fn into_str(self) -> BoxU<'a, str> {
        let bytes = self.into_bytes();
        unsafe { box_from_utf8_unchecked(bytes) }
    }

    pub(crate) fn into_cstr(mut self) -> Result<&'a CStr, E> {
        match self.vec.iter().position(|&c| c == b'\0') {
            Some(nul) => self.vec.truncate(nul + 1),
            None => {
                self.reserve_if_fallible(1)?;
                self.vec.push(b'\0');
            }
        }

        let bytes_with_nul = box_leak(self.into_bytes());
        Ok(unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) })
    }
}

impl<'a, A: BumpAllocatorScope<'a>, E: ErrorBehavior> fmt::Write for String<A, E> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).ok().ok_or(fmt::Error)
    }
}
