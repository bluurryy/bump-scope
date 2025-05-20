use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::{self, NonNull},
};
use std::alloc::System;

use crate::polyfill;

use super::{AllocError, Allocator};

// The Allocator impl checks the layout size to be non-zero and forwards to the GlobalAlloc impl,
// which is in `std::sys::*::alloc`.
unsafe impl Allocator for System {
    #[inline]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        alloc_impl(layout, false)
    }

    #[inline]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        alloc_impl(layout, true)
    }

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if layout.size() != 0 {
            // SAFETY: `layout` is non-zero in size,
            // other conditions must be upheld by the caller
            unsafe { GlobalAlloc::dealloc(self, ptr.as_ptr(), layout) }
        }
    }

    #[inline]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // SAFETY: all conditions must be upheld by the caller
        unsafe { grow_impl(ptr, old_layout, new_layout, false) }
    }

    #[inline]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // SAFETY: all conditions must be upheld by the caller
        unsafe { grow_impl(ptr, old_layout, new_layout, true) }
    }

    #[inline]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(
            new_layout.size() <= old_layout.size(),
            "`new_layout.size()` must be smaller than or equal to `old_layout.size()`"
        );

        match new_layout.size() {
            // SAFETY: conditions must be upheld by the caller
            0 => unsafe {
                Allocator::deallocate(self, ptr, old_layout);
                Ok(polyfill::non_null::slice_from_raw_parts(
                    polyfill::layout::dangling(new_layout),
                    0,
                ))
            },

            // SAFETY: `new_size` is non-zero. Other conditions must be upheld by the caller
            new_size if old_layout.align() == new_layout.align() => unsafe {
                // `realloc` probably checks for `new_size <= old_layout.size()` or something similar.
                polyfill::hint::assert_unchecked(new_size <= old_layout.size());

                let raw_ptr = GlobalAlloc::realloc(self, ptr.as_ptr(), old_layout, new_size);
                let ptr = NonNull::new(raw_ptr).ok_or(AllocError)?;
                Ok(polyfill::non_null::slice_from_raw_parts(ptr, new_size))
            },

            // SAFETY: because `new_size` must be smaller than or equal to `old_layout.size()`,
            // both the old and new memory allocation are valid for reads and writes for `new_size`
            // bytes. Also, because the old allocation wasn't yet deallocated, it cannot overlap
            // `new_ptr`. Thus, the call to `copy_nonoverlapping` is safe. The safety contract
            // for `dealloc` must be upheld by the caller.
            new_size => unsafe {
                let new_ptr = Allocator::allocate(self, new_layout)?;
                ptr::copy_nonoverlapping(ptr.as_ptr(), polyfill::non_null::as_mut_ptr(new_ptr), new_size);
                Allocator::deallocate(self, ptr, old_layout);
                Ok(new_ptr)
            },
        }
    }
}

#[inline]
fn alloc_impl(layout: Layout, zeroed: bool) -> Result<NonNull<[u8]>, AllocError> {
    match layout.size() {
        0 => Ok(polyfill::non_null::slice_from_raw_parts(polyfill::layout::dangling(layout), 0)),
        // SAFETY: `layout` is non-zero in size,
        size => unsafe {
            let raw_ptr = if zeroed {
                GlobalAlloc::alloc_zeroed(&System, layout)
            } else {
                GlobalAlloc::alloc(&System, layout)
            };
            let ptr = NonNull::new(raw_ptr).ok_or(AllocError)?;
            Ok(polyfill::non_null::slice_from_raw_parts(ptr, size))
        },
    }
}

// SAFETY: Same as `Allocator::grow`
#[inline]
unsafe fn grow_impl(
    ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
    zeroed: bool,
) -> Result<NonNull<[u8]>, AllocError> {
    debug_assert!(
        new_layout.size() >= old_layout.size(),
        "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
    );

    match old_layout.size() {
        0 => alloc_impl(new_layout, zeroed),

        // SAFETY: `new_size` is non-zero as `new_size` is greater than or equal to `old_size`
        // as required by safety conditions and the `old_size == 0` case was handled in the
        // previous match arm. Other conditions must be upheld by the caller
        old_size if old_layout.align() == new_layout.align() => unsafe {
            let new_size = new_layout.size();

            // `realloc` probably checks for `new_size >= old_layout.size()` or something similar.
            polyfill::hint::assert_unchecked(new_size >= old_layout.size());

            let raw_ptr = GlobalAlloc::realloc(&System, ptr.as_ptr(), old_layout, new_size);
            let ptr = NonNull::new(raw_ptr).ok_or(AllocError)?;
            if zeroed {
                raw_ptr.add(old_size).write_bytes(0, new_size - old_size);
            }
            Ok(polyfill::non_null::slice_from_raw_parts(ptr, new_size))
        },

        // SAFETY: because `new_layout.size()` must be greater than or equal to `old_size`,
        // both the old and new memory allocation are valid for reads and writes for `old_size`
        // bytes. Also, because the old allocation wasn't yet deallocated, it cannot overlap
        // `new_ptr`. Thus, the call to `copy_nonoverlapping` is safe. The safety contract
        // for `dealloc` must be upheld by the caller.
        old_size => unsafe {
            let new_ptr = alloc_impl(new_layout, zeroed)?;
            ptr::copy_nonoverlapping(ptr.as_ptr(), polyfill::non_null::as_mut_ptr(new_ptr), old_size);
            Allocator::deallocate(&System, ptr, old_layout);
            Ok(new_ptr)
        },
    }
}
