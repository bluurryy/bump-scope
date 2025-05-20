#[cfg(feature = "alloc")]
use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

/// See [`std::alloc::Layout::dangling`].
#[must_use]
#[inline]
#[cfg(feature = "alloc")]
pub const fn dangling(layout: Layout) -> NonNull<u8> {
    unsafe { super::nonnull::without_provenance(NonZeroUsize::new_unchecked(layout.align())) }
}
