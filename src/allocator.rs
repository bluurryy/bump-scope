// TODO: Figure out a way to make coverage report not suck because of `if UP { ... } else { ... }`.
#![allow(clippy::unnecessary_wraps)]

use allocator_api2::alloc::{AllocError, Allocator};
use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

use crate::{
    bump_down, polyfill::nonnull, up_align_usize_unchecked, Bump, BumpScope, MinimumAlignment, SupportedMinimumAlignment
};

unsafe impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> Allocator for BumpScope<'_, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        allocate(self, layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        deallocate(self, ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        grow(self, ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        grow_zeroed(self, ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        shrink(self, ptr, old_layout, new_layout)
    }
}

unsafe impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> Allocator for Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.as_scope().allocate(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.as_scope().deallocate(ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.as_scope().grow(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.as_scope().grow_zeroed(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.as_scope().shrink(ptr, old_layout, new_layout)
    }
}

#[inline(always)]
fn allocate<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool>(
    bump: &BumpScope<A, MIN_ALIGN, UP>,
    layout: Layout,
) -> Result<NonNull<[u8]>, AllocError>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    Ok(nonnull::slice_from_raw_parts(bump.try_alloc_layout(layout)?, layout.size()))
}

#[inline(always)]
unsafe fn deallocate<const MIN_ALIGN: usize, const UP: bool, A>(
    bump: &BumpScope<A, MIN_ALIGN, UP>,
    ptr: NonNull<u8>,
    layout: Layout,
) where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    // free allocated space if this is the last allocation
    if is_last(bump, ptr, layout) {
        deallocate_assume_last(bump, ptr, layout);
    }
}

#[inline(always)]
unsafe fn deallocate_assume_last<const MIN_ALIGN: usize, const UP: bool, A>(
    bump: &BumpScope<A, MIN_ALIGN, UP>,
    ptr: NonNull<u8>,
    layout: Layout,
) where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    debug_assert!(is_last(bump, ptr, layout));

    if UP {
        bump.chunk.get().set_pos(ptr);
    } else {
        let mut addr = nonnull::addr(ptr).get();
        addr += layout.size();
        addr = up_align_usize_unchecked(addr, MIN_ALIGN);

        let pos = nonnull::with_addr(ptr, NonZeroUsize::new_unchecked(addr));
        bump.chunk.get().set_pos(pos);
    }
}

#[inline(always)]
unsafe fn is_last<const MIN_ALIGN: usize, const UP: bool, A>(
    bump: &BumpScope<A, MIN_ALIGN, UP>,
    ptr: NonNull<u8>,
    layout: Layout,
) -> bool
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    if UP {
        ptr.as_ptr().add(layout.size()) == bump.chunk.get().pos().as_ptr()
    } else {
        ptr == bump.chunk.get().pos()
    }
}

#[inline(always)]
unsafe fn grow<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool>(
    bump: &BumpScope<A, MIN_ALIGN, UP>,
    old_ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, AllocError>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    debug_assert!(
        new_layout.size() >= old_layout.size(),
        "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
    );

    if UP {
        if is_last(bump, old_ptr, old_layout) & align_fits(old_ptr, old_layout, new_layout) {
            // We may be able to grow in place! Just need to check if there is enough space.

            let chunk_end = bump.chunk.get().content_end();
            let remaining = nonnull::addr(chunk_end).get() - nonnull::addr(old_ptr).get();

            if new_layout.size() <= remaining {
                // There is enough space! We will grow in place. Just need to update the bump pointer.

                let old_addr = nonnull::addr(old_ptr);

                // Up-aligning a pointer inside a chunks content by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(old_addr.get() + new_layout.size(), MIN_ALIGN);

                bump.chunk.get().set_pos_addr(new_pos);

                Ok(nonnull::slice_from_raw_parts(old_ptr, new_layout.size()))
            } else {
                // The current chunk doesn't have enough space to allocate this layout. We need to allocate in another chunk.
                let new_ptr = bump.alloc_in_another_chunk(new_layout)?;
                nonnull::copy_nonoverlapping(old_ptr, new_ptr, old_layout.size());
                Ok(nonnull::slice_from_raw_parts(new_ptr, new_layout.size()))
            }
        } else {
            // We can't grow in place. We have to make a new allocation.
            let new_ptr = bump.try_alloc_layout(new_layout)?;
            nonnull::copy_nonoverlapping(old_ptr, new_ptr, old_layout.size());
            Ok(nonnull::slice_from_raw_parts(new_ptr, new_layout.size()))
        }
    } else {
        if is_last(bump, old_ptr, old_layout) {
            // We may be able to reuse the currently allocated space. Just need to check if the current chunk has enough space for that.
            let additional_size = new_layout.size() - old_layout.size();

            let old_addr = nonnull::addr(old_ptr);
            let new_addr = bump_down(old_addr, additional_size, new_layout.align().max(MIN_ALIGN));

            let very_start = nonnull::addr(bump.chunk.get().content_start());

            if new_addr >= very_start.get() {
                // There is enough space in the current chunk! We will reuse the allocated space.

                let new_addr = NonZeroUsize::new_unchecked(new_addr);
                let new_addr_end = new_addr.get() + new_layout.size();

                let new_ptr = nonnull::with_addr(old_ptr, new_addr);

                // Check if the regions don't overlap so we may use the faster `copy_nonoverlapping`.
                if new_addr_end < old_addr.get() {
                    nonnull::copy_nonoverlapping(old_ptr, new_ptr, old_layout.size());
                } else {
                    nonnull::copy(old_ptr, new_ptr, old_layout.size());
                }

                bump.chunk.get().set_pos_addr(new_addr.get());
                Ok(nonnull::slice_from_raw_parts(new_ptr, new_layout.size()))
            } else {
                // The current chunk doesn't have enough space to allocate this layout. We need to allocate in another chunk.
                let new_ptr = bump.alloc_in_another_chunk(new_layout)?;
                nonnull::copy_nonoverlapping(old_ptr, new_ptr, old_layout.size());
                Ok(nonnull::slice_from_raw_parts(new_ptr, new_layout.size()))
            }
        } else {
            // We can't reuse the allocated space. We have to make a new allocation.
            let new_ptr = bump.try_alloc_layout(new_layout)?;
            nonnull::copy_nonoverlapping(old_ptr, new_ptr, old_layout.size());
            Ok(nonnull::slice_from_raw_parts(new_ptr, new_layout.size()))
        }
    }
}

#[inline(always)]
unsafe fn grow_zeroed<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool>(
    bump: &BumpScope<A, MIN_ALIGN, UP>,
    old_ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, AllocError>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    let new_ptr = grow(bump, old_ptr, old_layout, new_layout)?;

    let delta = new_layout.size() - old_layout.size();
    new_ptr.cast::<u8>().as_ptr().add(old_layout.size()).write_bytes(0, delta);

    Ok(new_ptr)
}

#[inline(always)]
unsafe fn shrink<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool>(
    bump: &BumpScope<A, MIN_ALIGN, UP>,
    old_ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, AllocError>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    /// Called when `new_layout` doesn't fit alignment.
    /// Does ANY consumer cause this?
    /// Bumpalo just errors in this case..
    #[cold]
    #[inline(never)]
    unsafe fn shrink_unfit<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool>(
        bump: &BumpScope<A, MIN_ALIGN, UP>,
        old_ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        if is_last(bump, old_ptr, old_layout) {
            let old_pos = bump.chunk.get().pos();
            deallocate_assume_last(bump, old_ptr, old_layout);

            let overlaps;
            let new_ptr;

            if let Some(in_chunk) = bump.alloc_in_current_chunk(new_layout) {
                new_ptr = in_chunk;
                overlaps = if UP {
                    let old_ptr_end = nonnull::add(old_ptr, new_layout.size());
                    old_ptr_end > new_ptr
                } else {
                    let new_ptr_end = nonnull::add(new_ptr, new_layout.size());
                    new_ptr_end > old_ptr
                }
            } else {
                new_ptr = match bump.alloc_in_another_chunk(new_layout) {
                    Ok(new_ptr) => new_ptr,
                    Err(error) => {
                        // Need to reset the bump pointer to the old position.
                        bump.chunk.get().set_pos(old_pos);
                        return Err(error);
                    }
                };
                overlaps = false;
            }

            if overlaps {
                nonnull::copy(old_ptr, new_ptr, new_layout.size());
            } else {
                nonnull::copy_nonoverlapping(old_ptr, new_ptr, new_layout.size());
            }

            Ok(nonnull::slice_from_raw_parts(new_ptr, new_layout.size()))
        } else {
            let new_ptr = bump.try_alloc_layout(new_layout)?;
            nonnull::copy_nonoverlapping(old_ptr, new_ptr, new_layout.size());
            Ok(nonnull::slice_from_raw_parts(new_ptr, new_layout.size()))
        }
    }

    debug_assert!(
        new_layout.size() <= old_layout.size(),
        "`new_layout.size()` must be smaller than or equal to `old_layout.size()`"
    );

    if !align_fits(old_ptr, old_layout, new_layout) {
        return shrink_unfit(bump, old_ptr, old_layout, new_layout);
    }

    // if that's not the last allocation, there is nothing we can do
    if !is_last(bump, old_ptr, old_layout) {
        // we return the size of the old layout
        return Ok(nonnull::slice_from_raw_parts(old_ptr, old_layout.size()));
    }

    if UP {
        let end = nonnull::addr(old_ptr).get() + new_layout.size();

        // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
        let new_pos = up_align_usize_unchecked(end, MIN_ALIGN);

        bump.chunk.get().set_pos_addr(new_pos);
        Ok(nonnull::slice_from_raw_parts(old_ptr, new_layout.size()))
    } else {
        let old_addr = nonnull::addr(old_ptr);
        let old_addr_old_end = NonZeroUsize::new_unchecked(old_addr.get() + old_layout.size());

        let new_addr = bump_down(old_addr_old_end, new_layout.size(), new_layout.align().max(MIN_ALIGN));
        let new_addr = NonZeroUsize::new_unchecked(new_addr);
        let old_addr_new_end = NonZeroUsize::new_unchecked(old_addr.get() + new_layout.size());

        let new_ptr = nonnull::with_addr(old_ptr, new_addr);

        let overlaps = old_addr_new_end > new_addr;

        if overlaps {
            nonnull::copy(old_ptr, new_ptr, new_layout.size());
        } else {
            nonnull::copy_nonoverlapping(old_ptr, new_ptr, new_layout.size());
        }

        bump.chunk.get().set_pos(new_ptr);
        Ok(nonnull::slice_from_raw_parts(new_ptr, new_layout.size()))
    }
}

#[inline(always)]
fn align_fits(old_ptr: NonNull<u8>, _old_layout: Layout, new_layout: Layout) -> bool {
    nonnull::is_aligned_to(old_ptr, new_layout.align())
}