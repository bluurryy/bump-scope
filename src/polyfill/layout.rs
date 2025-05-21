#[cfg(feature = "alloc")]
use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

/// See [`std::alloc::Layout::dangling`].
#[must_use]
#[inline]
#[cfg(feature = "alloc")]
pub(crate) const fn dangling(layout: Layout) -> NonNull<u8> {
    unsafe { super::non_null::without_provenance(NonZeroUsize::new_unchecked(layout.align())) }
}
