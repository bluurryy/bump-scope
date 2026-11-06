use core::{mem::ManuallyDrop, ptr};

#[inline(always)]
pub(crate) unsafe fn ptr_result<T, E>(ptr: *mut Result<T, E>) -> Result<*mut T, *mut E> {
    unsafe {
        match &mut *ptr {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err),
        }
    }
}

/// A version of [`std::mem::transmute`] that can transmute between generic types.
#[allow(clippy::manual_assert_eq, reason = "wouldn't be const otherwise")]
pub(crate) unsafe fn transmute_value<A, B>(a: A) -> B {
    assert!(size_of::<A>() == size_of::<B>());
    unsafe { core::mem::transmute_copy(&ManuallyDrop::new(a)) }
}

/// A safer [`std::mem::transmute`] for references.
#[allow(clippy::manual_assert_eq, reason = "wouldn't be const otherwise")]
pub(crate) const unsafe fn transmute_ref<A, B>(a: &A) -> &B {
    assert!(size_of::<A>() == size_of::<B>());
    unsafe { &*ptr::from_ref(a).cast::<B>() }
}

/// A safer [`std::mem::transmute`] for mutable references.
#[allow(clippy::manual_assert_eq, reason = "wouldn't be const otherwise")]
pub(crate) unsafe fn transmute_mut<A, B>(a: &mut A) -> &mut B {
    assert!(size_of::<A>() == size_of::<B>());
    unsafe { &mut *ptr::from_mut(a).cast::<B>() }
}
