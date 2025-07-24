//! Stuff that is missing from the msrv's std.
//!
//! This module also includes utility functions
//! that are not from the standard library.
#![allow(
    // it's not the same in terms of strict provenance
    clippy::transmutes_expressible_as_ptr_casts,
    // it's not the same in terms of strict provenance
    clippy::useless_transmute,
    clippy::pedantic,
)]

pub(crate) mod hint;
pub(crate) mod iter;
pub(crate) mod layout;
pub(crate) mod non_null;
pub(crate) mod option;
pub(crate) mod pointer;
pub(crate) mod pointer_mut;
pub(crate) mod ptr;
pub(crate) mod slice;
pub(crate) mod str;
pub(crate) mod usize;

use core::mem::{size_of, ManuallyDrop};

/// Not part of std.
///
/// A version of [`std::mem::transmute`] that can transmute between generic types.
pub(crate) unsafe fn transmute_value<A, B>(a: A) -> B {
    assert!(size_of::<A>() == size_of::<B>());
    core::mem::transmute_copy(&ManuallyDrop::new(a))
}

/// Not part of std.
///
/// A safer [`std::mem::transmute`].
pub(crate) const unsafe fn transmute_ref<A, B>(a: &A) -> &B {
    assert!(size_of::<A>() == size_of::<B>());
    &*(a as *const A).cast::<B>()
}

/// Not part of std.
///
/// A safer [`std::mem::transmute`].
pub(crate) unsafe fn transmute_mut<A, B>(a: &mut A) -> &mut B {
    assert!(size_of::<A>() == size_of::<B>());
    &mut *(a as *mut A).cast::<B>()
}
