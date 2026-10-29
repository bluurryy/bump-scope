//! Minimal string implementation for internal usage.
//! Would be replaced by `alloc::String` if that becomes generic over its allocator.
//!
//! For now we add a `ErrorBehavior` generic, just because it makes for simpler code.

use core::{ffi::CStr, fmt, marker::PhantomData};

use alloc::boxed::Box;

use crate::{
    bump_allocation::IntoRef, error_behavior::ErrorBehavior, extra::box_from_utf8_unchecked, mut_vec::MutVec, traits::Sealed,
};

#[repr(transparent)]
pub(crate) struct MutString<'a, A: Sealed, E> {
    vec: MutVec<'a, u8, A>,
    marker: PhantomData<fn() -> E>,
}

impl<'a, A: Sealed, E> MutString<'a, A, E>
where
    E: ErrorBehavior,
{
    #[expect(unused)]
    pub(crate) fn new(min_cap: usize, alloc: &'a mut A) -> Result<Self, E> {
        Ok(Self {
            vec: MutVec::new(min_cap, alloc)?,
            marker: PhantomData,
        })
    }

    pub(crate) fn new_from_vec(mut vec: MutVec<'a, u8, A>) -> Self {
        // no bytes is valid utf8
        vec.clear();

        Self {
            vec,
            marker: PhantomData,
        }
    }

    pub(crate) fn push_str(&mut self, str: &str) -> Result<(), E> {
        self.vec.extend_from_slice(str.as_bytes())
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

    pub(crate) fn into_bytes(self) -> Box<[u8], &'a A> {
        self.vec.into_boxed_slice()
    }

    pub(crate) fn into_str(self) -> Box<str, &'a A> {
        let bytes = self.into_bytes();
        unsafe { box_from_utf8_unchecked(bytes) }
    }

    pub(crate) fn into_cstr(mut self) -> Result<IntoRef<CStr, &'a A>, E> {
        if let Some(nul) = self.vec.iter().position(|&c| c == b'\0') {
            self.vec.truncate(nul + 1);
        } else {
            self.push_str("\0")?;
        }

        let (bytes_with_nul, alloc) = Box::into_raw_with_allocator(self.into_bytes());
        let ptr = unsafe { CStr::from_bytes_with_nul_unchecked(&*bytes_with_nul) };
        Ok(unsafe { IntoRef::new(ptr, alloc) })
    }
}

impl<A: Sealed, E: ErrorBehavior> fmt::Write for MutString<'_, A, E> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).ok().ok_or(fmt::Error)
    }
}
