use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

/// See [`core::alloc::Layout::dangling_ptr`].
#[inline]
#[must_use]
pub(crate) const fn dangling_ptr(layout: Layout) -> NonNull<u8> {
    unsafe { super::non_null::without_provenance(NonZeroUsize::new_unchecked(layout.align())) }
}
