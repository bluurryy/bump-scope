//! Minimal string implementation for internal usage.
//! Would be replaced by `alloc::String` if that becomes generic over its allocator.
//!
//! For now we add a `ErrorBehavior` generic, just because it makes for simpler code.

use core::{alloc::Allocator, ffi::CStr, fmt, marker::PhantomData};

use alloc::{boxed::Box, vec::Vec};

use crate::{bump_allocation::IntoRef, error_behavior::ErrorBehavior, extra::box_from_utf8_unchecked};

#[repr(transparent)]
pub(crate) struct String<A: Allocator, E: ErrorBehavior> {
    vec: Vec<u8, A>,
    marker: PhantomData<fn() -> E>,
}

impl<A: Allocator, E: ErrorBehavior> String<A, E> {
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

    pub(crate) fn into_bytes(self) -> Box<[u8], A> {
        self.vec.into_boxed_slice()
    }

    pub(crate) fn into_str(self) -> Box<str, A> {
        let bytes = self.into_bytes();
        unsafe { box_from_utf8_unchecked(bytes) }
    }

    pub(crate) fn into_cstr(mut self) -> Result<IntoRef<CStr, A>, E> {
        if let Some(nul) = self.vec.iter().position(|&c| c == b'\0') {
            self.vec.truncate(nul + 1);
        } else {
            self.reserve_if_fallible(1)?;
            self.vec.push(b'\0');
        }

        let (bytes_with_nul, alloc) = Box::into_raw_with_allocator(self.into_bytes());
        let ptr = unsafe { CStr::from_bytes_with_nul_unchecked(&*bytes_with_nul) };
        Ok(unsafe { IntoRef::new(ptr, alloc) })
    }
}

impl<A: Allocator, E: ErrorBehavior> fmt::Write for String<A, E> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).ok().ok_or(fmt::Error)
    }
}
