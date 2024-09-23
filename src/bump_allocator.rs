#![allow(private_bounds, missing_docs, unused_variables, clippy::missing_errors_doc)]

use crate::{
    bump_down, error_behavior::ErrorBehavior, polyfill::nonnull, up_align_usize_unchecked, BaseAllocator, Bump, BumpBox,
    BumpScope, FixedBumpVec, MinimumAlignment, SizedTypeProperties, SupportedMinimumAlignment, WithoutDealloc,
    WithoutShrink,
};
use allocator_api2::alloc::Allocator;
use core::num::NonZeroUsize;

/// An allocator that allows `grow(_zeroed)`, `shrink` and `deallocate` calls with pointers that were not allocated by this allocator.
/// This trait is used for [`BumpBox::into_box`](BumpBox::into_box) to allow safely converting a `BumpBox` into a `Box`.
///
/// The allocations made with this allocator will have a lifetime of `'a`.
///
/// # Safety
/// - `grow(_zeroed)`, `shrink` and `deallocate` must be ok to be called with a pointer that was not allocated by this Allocator
pub unsafe trait BumpAllocator<'a>: Allocator {
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B>;
    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B>;
    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>);
    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B>;
}

unsafe impl<'a, A> BumpAllocator<'a> for &A
where
    A: BumpAllocator<'a>,
{
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        A::vec_alloc(self, capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        A::vec_grow(self, fixed, additional)
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        A::vec_shrink_to_fit(self, fixed);
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        A::slice_clone(self, slice)
    }
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator<'a>
    for &'a Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        self.as_scope().vec_alloc(capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        self.as_scope().vec_grow(fixed, additional)
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        self.as_scope().vec_shrink_to_fit(fixed);
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        self.as_scope().slice_clone(slice)
    }
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator<'a>
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        self.generic_alloc_fixed_vec(capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        let required_cap = match fixed.len().checked_add(additional) {
            Some(required_cap) => required_cap,
            None => return Err(B::capacity_overflow())?,
        };

        if T::IS_ZST {
            return Ok(());
        }

        if fixed.capacity() == 0 {
            *fixed = self.generic_alloc_fixed_vec(required_cap)?;
            return Ok(());
        }

        let old_ptr = fixed.as_non_null_ptr();
        let new_cap = fixed.capacity().checked_mul(2).unwrap_or(required_cap).max(required_cap);
        let old_size = fixed.capacity * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_cap.checked_mul(T::SIZE).ok_or_else(|| B::capacity_overflow())?;

        unsafe {
            if UP {
                let is_last = nonnull::byte_add(old_ptr, old_size).cast() == self.chunk.get().pos();

                if is_last {
                    let chunk_end = self.chunk.get().content_end();
                    let remaining = nonnull::addr(chunk_end).get() - nonnull::addr(old_ptr).get();

                    if new_size <= remaining {
                        // There is enough space! We will grow in place. Just need to update the bump pointer.

                        let old_addr = nonnull::addr(old_ptr);
                        let new_end = old_addr.get() + new_size;

                        // Up-aligning a pointer inside a chunks content by `MIN_ALIGN` never overflows.
                        let new_pos = up_align_usize_unchecked(new_end, MIN_ALIGN);

                        self.chunk.get().set_pos_addr(new_pos);
                    } else {
                        // The current chunk doesn't have enough space to allocate this layout. We need to allocate in another chunk.
                        let new_ptr = self.do_alloc_slice_in_another_chunk::<B, T>(new_cap)?.cast();
                        nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                        fixed.initialized.set_ptr(new_ptr);
                    }
                } else {
                    let new_ptr = self.do_alloc_slice::<B, T>(new_cap)?.cast();
                    nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                    fixed.initialized.set_ptr(new_ptr);
                }
            } else {
                let is_last = old_ptr.cast() == self.chunk.get().pos();

                if is_last {
                    // We may be able to reuse the currently allocated space. Just need to check if the current chunk has enough space for that.
                    let additional_size = new_size - old_size;

                    let old_addr = nonnull::addr(old_ptr);
                    let new_addr = bump_down(old_addr, additional_size, T::ALIGN.max(MIN_ALIGN));

                    let very_start = nonnull::addr(self.chunk.get().content_start());

                    if new_addr >= very_start.get() {
                        // There is enough space in the current chunk! We will reuse the allocated space.

                        let new_addr = NonZeroUsize::new_unchecked(new_addr);
                        let new_addr_end = new_addr.get() + new_size;

                        let new_ptr = nonnull::with_addr(old_ptr, new_addr);

                        // Check if the regions don't overlap so we may use the faster `copy_nonoverlapping`.
                        if new_addr_end < old_addr.get() {
                            nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                        } else {
                            nonnull::copy::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                        }

                        self.chunk.get().set_pos(new_ptr.cast());
                        fixed.initialized.set_ptr(new_ptr);
                    } else {
                        // The current chunk doesn't have enough space to allocate this layout. We need to allocate in another chunk.
                        let new_ptr = self.do_alloc_slice_in_another_chunk::<B, T>(new_cap)?.cast();
                        nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                        fixed.initialized.set_ptr(new_ptr);
                    }
                } else {
                    let new_ptr = self.do_alloc_slice::<B, T>(new_cap)?.cast();
                    nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                    fixed.initialized.set_ptr(new_ptr);
                }
            }
        }

        fixed.capacity = new_cap;
        Ok(())
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        let old_ptr = fixed.as_non_null_ptr().cast::<u8>();
        let old_size = fixed.capacity * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = fixed.len() * T::SIZE; // its less than the capacity so this can't overflow

        // Adapted from `Allocator::shrink`.
        unsafe {
            let is_last = if UP {
                old_ptr.as_ptr().add(old_size) == self.chunk.get().pos().as_ptr()
            } else {
                old_ptr == self.chunk.get().pos()
            };

            // if that's not the last allocation, there is nothing we can do
            if !is_last {
                return;
            }

            if UP {
                let end = nonnull::addr(old_ptr).get() + new_size;

                // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(end, MIN_ALIGN);

                self.chunk.get().set_pos_addr(new_pos);
            } else {
                let old_addr = nonnull::addr(old_ptr);
                let old_addr_old_end = NonZeroUsize::new_unchecked(old_addr.get() + old_size);

                let new_addr = bump_down(old_addr_old_end, new_size, T::ALIGN.max(MIN_ALIGN));
                let new_addr = NonZeroUsize::new_unchecked(new_addr);
                let old_addr_new_end = NonZeroUsize::new_unchecked(old_addr.get() + new_size);

                let new_ptr = nonnull::with_addr(old_ptr, new_addr);
                let overlaps = old_addr_new_end > new_addr;

                if overlaps {
                    nonnull::copy(old_ptr, new_ptr, new_size);
                } else {
                    nonnull::copy_nonoverlapping(old_ptr, new_ptr, new_size);
                }

                self.chunk.get().set_pos(new_ptr);
                fixed.initialized.set_ptr(new_ptr.cast());
            }

            fixed.capacity = fixed.len();
        }
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        self.generic_alloc_slice_clone(slice)
    }
}

unsafe impl<'a, A: BumpAllocator<'a>> BumpAllocator<'a> for WithoutDealloc<A> {
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        A::vec_alloc(&self.0, capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        A::vec_grow(&self.0, fixed, additional)
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        A::vec_shrink_to_fit(&self.0, fixed);
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        A::slice_clone(&self.0, slice)
    }
}

unsafe impl<'a, A: BumpAllocator<'a>> BumpAllocator<'a> for WithoutShrink<A> {
    fn vec_alloc<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        A::vec_alloc(&self.0, capacity)
    }

    fn vec_grow<B: ErrorBehavior, T>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), B> {
        A::vec_grow(&self.0, fixed, additional)
    }

    fn vec_shrink_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        A::vec_shrink_to_fit(&self.0, fixed);
    }

    fn slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        A::slice_clone(&self.0, slice)
    }
}
