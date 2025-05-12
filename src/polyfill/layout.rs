#[cfg(feature = "alloc")]
use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

/// Creates a `NonNull` that is dangling, but well-aligned for this Layout.
///
/// Note that the pointer value may potentially represent a valid pointer,
/// which means this must not be used as a "not yet initialized"
/// sentinel value. Types that lazily allocate must track initialization by
/// some other means.
#[must_use]
#[inline]
#[cfg(feature = "alloc")]
pub const fn dangling(layout: Layout) -> NonNull<u8> {
    unsafe { super::nonnull::without_provenance(NonZeroUsize::new_unchecked(layout.align())) }
}
