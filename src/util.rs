use core::{alloc::Layout, mem::ManuallyDrop, ptr};

#[cfg(test)]
use core::{
    alloc::{AllocError, Allocator},
    ptr::NonNull,
};

macro_rules! config {
    ($config:expr, $name:ident) => {
        const { ::bump_scope_core::config::read::$name($config) }
    };
    ($name:ident) => {
        const { ::bump_scope_core::config::read::$name(C) }
    };
}

pub(crate) use config;

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

/// Does not check for overflow.
#[inline(always)]
pub(crate) fn up_align_usize_unchecked(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    (addr + mask) & !mask
}

#[inline(always)]
pub(crate) fn down_align_usize(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}

pub(crate) trait SizedTypeProperties: Sized {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = align_of::<Self>();
    const LAYOUT: Layout = Layout::new::<Self>();

    const IS_ZST: bool = size_of::<Self>() == 0;
}

impl<T> SizedTypeProperties for T {}

/// Used for static assertions.
#[cfg(test)]
#[derive(Default, Clone)]
pub(crate) struct NoopAllocator;

#[cfg(test)]
unsafe impl Allocator for NoopAllocator {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
}
