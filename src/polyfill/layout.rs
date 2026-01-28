use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

/// See [`std::alloc::Layout::dangling`].
#[inline]
#[must_use]
pub(crate) const fn dangling(layout: Layout) -> NonNull<u8> {
    unsafe { super::non_null::without_provenance(NonZeroUsize::new_unchecked(layout.align())) }
}
