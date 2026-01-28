//! Stuff that is missing from the msrv's std.
//!
//! This module also includes utility functions
//! that are not from the standard library.
#![expect(clippy::pedantic)]

pub(crate) mod hint;
pub(crate) mod iter;
#[cfg(feature = "alloc")]
pub(crate) mod layout;
pub(crate) mod non_null;
pub(crate) mod pointer;
pub(crate) mod slice;

use core::mem::{ManuallyDrop, size_of};

/// Not part of std.
///
/// A version of [`std::mem::transmute`] that can transmute between generic types.
pub(crate) unsafe fn transmute_value<A, B>(a: A) -> B {
    assert!(size_of::<A>() == size_of::<B>());
    unsafe { core::mem::transmute_copy(&ManuallyDrop::new(a)) }
}

/// Not part of std.
///
/// A safer [`std::mem::transmute`].
pub(crate) const unsafe fn transmute_ref<A, B>(a: &A) -> &B {
    assert!(size_of::<A>() == size_of::<B>());
    unsafe { &*(a as *const A).cast::<B>() }
}

/// Not part of std.
///
/// A safer [`std::mem::transmute`].
pub(crate) unsafe fn transmute_mut<A, B>(a: &mut A) -> &mut B {
    assert!(size_of::<A>() == size_of::<B>());
    unsafe { &mut *(a as *mut A).cast::<B>() }
}
