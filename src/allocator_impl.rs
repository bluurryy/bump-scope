// FIXME: Figure out a way to make coverage report not suck because of `if UP { ... } else { ... }`.

use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

use crate::{
    BaseAllocator, BumpScope, alloc::AllocError, bump_down, polyfill::non_null, settings::BumpAllocatorSettings,
    up_align_usize_unchecked,
};

#[inline(always)]
pub(crate) fn allocate<A, S>(bump: &BumpScope<A, S>, layout: Layout) -> Result<NonNull<[u8]>, AllocError>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    Ok(NonNull::slice_from_raw_parts(bump.try_alloc_layout(layout)?, layout.size()))
}

#[inline(always)]
pub(crate) unsafe fn deallocate<A, S>(bump: &BumpScope<A, S>, ptr: NonNull<u8>, layout: Layout)
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    if !S::DEALLOCATES {
        return;
    }

    unsafe {
        // free allocated space if this is the last allocation
        if is_last_and_allocated(bump, ptr, layout) {
            deallocate_assume_last(bump, ptr, layout);
        }
    }
}

#[inline(always)]
unsafe fn deallocate_assume_last<A, S>(bump: &BumpScope<A, S>, ptr: NonNull<u8>, layout: Layout)
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    if !S::DEALLOCATES {
        return;
    }

    unsafe {
        debug_assert!(is_last_and_allocated(bump, ptr, layout));

        if S::UP {
            bump.set_pos(ptr.addr());
        } else {
            let mut addr = ptr.addr().get();
            addr += layout.size();
            bump.set_pos(NonZeroUsize::new_unchecked(addr));
        }
    }
}

#[inline(always)]
unsafe fn is_last_and_allocated<A, S>(bump: &BumpScope<A, S>, ptr: NonNull<u8>, layout: Layout) -> bool
where
    S: BumpAllocatorSettings,
{
    if bump.chunk.get().is_unallocated() {
        return false;
    }

    unsafe {
        if S::UP {
            ptr.add(layout.size()) == bump.chunk.get().pos()
        } else {
            ptr == bump.chunk.get().pos()
        }
    }
}

#[inline(always)]
pub(crate) unsafe fn grow<A, S>(
    bump: &BumpScope<A, S>,
    old_ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, AllocError>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    debug_assert!(
        new_layout.size() >= old_layout.size(),
        "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
    );

    unsafe {
        if S::UP {
            if is_last_and_allocated(bump, old_ptr, old_layout) & align_fits(old_ptr, old_layout, new_layout) {
                // We may be able to grow in place! Just need to check if there is enough space.

                let chunk = bump.chunk.get().guaranteed_allocated_unchecked();
                let chunk_end = chunk.content_end();
                let remaining = chunk_end.addr().get() - old_ptr.addr().get();

                if new_layout.size() <= remaining {
                    // There is enough space! We will grow in place. Just need to update the bump pointer.

                    let old_addr = old_ptr.addr();

                    // Up-aligning a pointer inside a chunks content by `MIN_ALIGN` never overflows.
                    let new_pos = up_align_usize_unchecked(old_addr.get() + new_layout.size(), S::MIN_ALIGN);

                    // `is_last_and_allocated` returned true
                    chunk.set_pos_addr(new_pos);

                    Ok(NonNull::slice_from_raw_parts(old_ptr, new_layout.size()))
                } else {
                    // The current chunk doesn't have enough space to allocate this layout. We need to allocate in another chunk.
                    let new_ptr = bump.alloc_in_another_chunk::<AllocError>(new_layout)?;
                    old_ptr.copy_to_nonoverlapping(new_ptr, old_layout.size());
                    Ok(NonNull::slice_from_raw_parts(new_ptr, new_layout.size()))
                }
            } else {
                // We can't grow in place. We have to make a new allocation.
                let new_ptr = bump.try_alloc_layout(new_layout)?;
                old_ptr.copy_to_nonoverlapping(new_ptr, old_layout.size());
                Ok(NonNull::slice_from_raw_parts(new_ptr, new_layout.size()))
            }
        } else {
            if is_last_and_allocated(bump, old_ptr, old_layout) {
                // We may be able to reuse the currently allocated space. Just need to check if the current chunk has enough space for that.
                let additional_size = new_layout.size() - old_layout.size();

                let old_addr = old_ptr.addr();
                let new_addr = bump_down(old_addr, additional_size, new_layout.align().max(S::MIN_ALIGN));

                let chunk = bump.chunk.get().guaranteed_allocated_unchecked();
                let very_start = chunk.content_start().addr();

                if new_addr >= very_start.get() {
                    // There is enough space in the current chunk! We will reuse the allocated space.

                    let new_addr = NonZeroUsize::new_unchecked(new_addr);
                    let new_addr_end = new_addr.get() + new_layout.size();

                    let new_ptr = old_ptr.with_addr(new_addr);

                    // Check if the regions don't overlap so we may use the faster `copy_nonoverlapping`.
                    if new_addr_end < old_addr.get() {
                        old_ptr.copy_to_nonoverlapping(new_ptr, old_layout.size());
                    } else {
                        old_ptr.copy_to(new_ptr, old_layout.size());
                    }

                    // `is_last_and_allocated` returned true
                    chunk.set_pos_addr(new_addr.get());

                    Ok(NonNull::slice_from_raw_parts(new_ptr, new_layout.size()))
                } else {
                    // The current chunk doesn't have enough space to allocate this layout. We need to allocate in another chunk.
                    let new_ptr = bump.alloc_in_another_chunk::<AllocError>(new_layout)?;
                    old_ptr.copy_to_nonoverlapping(new_ptr, old_layout.size());
                    Ok(NonNull::slice_from_raw_parts(new_ptr, new_layout.size()))
                }
            } else {
                // We can't reuse the allocated space. We have to make a new allocation.
                let new_ptr = bump.try_alloc_layout(new_layout)?;
                old_ptr.copy_to_nonoverlapping(new_ptr, old_layout.size());
                Ok(NonNull::slice_from_raw_parts(new_ptr, new_layout.size()))
            }
        }
    }
}

#[inline(always)]
pub(crate) unsafe fn grow_zeroed<A, S>(
    bump: &BumpScope<A, S>,
    old_ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, AllocError>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    unsafe {
        let new_ptr = grow(bump, old_ptr, old_layout, new_layout)?;

        let delta = new_layout.size() - old_layout.size();
        new_ptr.cast::<u8>().add(old_layout.size()).write_bytes(0, delta);

        Ok(new_ptr)
    }
}

/// This shrink implementation always tries to reuse old memory if it can.
///
/// That's different to bumpalo's shrink implementation, which only shrinks if it can do so with `copy_nonoverlapping`
/// and doesn't attempt to recover memory if the alignment doesn't fit.
#[inline(always)]
pub(crate) unsafe fn shrink<A, S>(
    bump: &BumpScope<A, S>,
    old_ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, AllocError>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    /// Called when `new_layout` doesn't fit alignment.
    #[cold]
    #[inline(never)]
    unsafe fn shrink_unfit<A, S>(
        bump: &BumpScope<A, S>,
        old_ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError>
    where
        A: BaseAllocator<S::GuaranteedAllocated>,
        S: BumpAllocatorSettings,
    {
        unsafe {
            if S::DEALLOCATES && is_last_and_allocated(bump, old_ptr, old_layout) {
                let old_pos = bump.chunk.get().pos();
                deallocate_assume_last(bump, old_ptr, old_layout);

                let overlaps;
                let new_ptr;

                if let Some(in_chunk) = bump.alloc_in_current_chunk(new_layout) {
                    new_ptr = in_chunk;
                    overlaps = if S::UP {
                        let old_ptr_end = old_ptr.add(new_layout.size());
                        old_ptr_end > new_ptr
                    } else {
                        let new_ptr_end = new_ptr.add(new_layout.size());
                        new_ptr_end > old_ptr
                    }
                } else {
                    new_ptr = match bump.alloc_in_another_chunk(new_layout) {
                        Ok(new_ptr) => new_ptr,
                        Err(error) => {
                            // Need to reset the bump pointer to the old position.

                            // `is_last_and_allocated` returned true
                            bump.chunk.get().guaranteed_allocated_unchecked().set_pos(old_pos);
                            return Err(error);
                        }
                    };
                    overlaps = false;
                }

                if overlaps {
                    old_ptr.copy_to(new_ptr, new_layout.size());
                } else {
                    old_ptr.copy_to_nonoverlapping(new_ptr, new_layout.size());
                }

                Ok(NonNull::slice_from_raw_parts(new_ptr, new_layout.size()))
            } else {
                let new_ptr = bump.try_alloc_layout(new_layout)?;
                old_ptr.copy_to_nonoverlapping(new_ptr, new_layout.size());
                Ok(NonNull::slice_from_raw_parts(new_ptr, new_layout.size()))
            }
        }
    }

    debug_assert!(
        new_layout.size() <= old_layout.size(),
        "`new_layout.size()` must be smaller than or equal to `old_layout.size()`"
    );

    unsafe {
        if !align_fits(old_ptr, old_layout, new_layout) {
            return shrink_unfit(bump, old_ptr, old_layout, new_layout);
        }

        // if that's not the last allocation, there is nothing we can do
        if !S::DEALLOCATES || !is_last_and_allocated(bump, old_ptr, old_layout) {
            // we return the size of the old layout
            return Ok(NonNull::slice_from_raw_parts(old_ptr, old_layout.size()));
        }

        if S::UP {
            let end = old_ptr.addr().get() + new_layout.size();

            // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
            let new_pos = up_align_usize_unchecked(end, S::MIN_ALIGN);

            // `is_last_and_allocated` returned true
            bump.chunk.get().guaranteed_allocated_unchecked().set_pos_addr(new_pos);

            Ok(NonNull::slice_from_raw_parts(old_ptr, new_layout.size()))
        } else {
            let old_addr = old_ptr.addr();
            let old_end_addr = NonZeroUsize::new_unchecked(old_addr.get() + old_layout.size());

            let new_addr = bump_down(old_end_addr, new_layout.size(), new_layout.align().max(S::MIN_ALIGN));
            let new_addr = NonZeroUsize::new_unchecked(new_addr);
            let new_ptr = old_ptr.with_addr(new_addr);

            let copy_src_end = NonZeroUsize::new_unchecked(old_addr.get() + new_layout.size());
            let copy_dst_start = new_addr;
            let overlaps = copy_src_end > copy_dst_start;

            if overlaps {
                old_ptr.copy_to(new_ptr, new_layout.size());
            } else {
                old_ptr.copy_to_nonoverlapping(new_ptr, new_layout.size());
            }

            // `is_last_and_allocated` returned true
            bump.chunk.get().guaranteed_allocated_unchecked().set_pos(new_ptr);

            Ok(NonNull::slice_from_raw_parts(new_ptr, new_layout.size()))
        }
    }
}

#[inline(always)]
fn align_fits(old_ptr: NonNull<u8>, _old_layout: Layout, new_layout: Layout) -> bool {
    non_null::is_aligned_to(old_ptr, new_layout.align())
}
