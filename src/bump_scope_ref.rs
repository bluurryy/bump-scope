mod sealed {
    use super::*;

    #[allow(private_bounds)]
    pub trait Sealed<'a> {
        type Stats<'b>;
        type Allocator;

        fn stats(&self) -> Self::Stats<'a>;
        fn allocator(&self) -> &Self::Allocator;

        fn alloc_fixed_vec<T, E>(&self, len: usize) -> Result<FixedBumpVec<'a, T>, E>
        where
            E: ErrorBehavior;

        fn grow_fixed_vec<T, E>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), E>
        where
            E: ErrorBehavior;

        fn shrink_fixed_vec_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>);
    }
}

use core::num::NonZeroUsize;

use sealed::Sealed;

use crate::{
    bump_down, polyfill::nonnull, up_align_usize_unchecked, BumpScope, ErrorBehavior, FixedBumpVec, MinimumAlignment,
    SizedTypeProperties, Stats, SupportedMinimumAlignment,
};

use allocator_api2::alloc::Allocator;

// This trait is intentionally not implemented for every `T: Sealed` so the implementors show up in the docs.
pub trait BumpScopeRef<'a>: Sealed<'a> {}

#[allow(private_bounds)]
impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Sealed<'a> for BumpScope<'a, A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    type Stats<'b> = Stats<'b, UP>;
    type Allocator = A;

    fn stats(&self) -> Self::Stats<'a> {
        BumpScope::stats(self)
    }

    fn allocator(&self) -> &Self::Allocator {
        BumpScope::allocator(self)
    }

    fn alloc_fixed_vec<T, E>(&self, len: usize) -> Result<FixedBumpVec<'a, T>, E>
    where
        E: ErrorBehavior,
    {
        BumpScope::generic_alloc_fixed_vec(self, len)
    }

    fn grow_fixed_vec<T, E>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), E>
    where
        E: ErrorBehavior,
    {
        let required_cap = match fixed.len().checked_add(additional) {
            Some(required_cap) => required_cap,
            None => return Err(E::capacity_overflow())?,
        };

        if T::IS_ZST {
            return Ok(());
        }

        if fixed.capacity == 0 {
            *fixed = self.generic_alloc_fixed_vec(required_cap)?;
            return Ok(());
        }

        let old_ptr = fixed.as_non_null_ptr();
        let new_cap = fixed.capacity.checked_mul(2).unwrap_or(required_cap).max(required_cap);
        let old_size = fixed.capacity * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_cap.checked_mul(T::SIZE).ok_or_else(|| E::capacity_overflow())?;

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
                        let new_ptr = self.do_alloc_slice_in_another_chunk::<E, T>(new_cap)?.cast();
                        nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                        fixed.initialized.set_ptr(new_ptr);
                    }
                } else {
                    let new_ptr = self.do_alloc_slice::<E, T>(new_cap)?.cast();
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

                        self.chunk.get().set_pos_addr(new_addr.get());
                        fixed.initialized.set_ptr(new_ptr);
                    } else {
                        // The current chunk doesn't have enough space to allocate this layout. We need to allocate in another chunk.
                        let new_ptr = self.do_alloc_slice_in_another_chunk::<E, T>(new_cap)?.cast();
                        nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                        fixed.initialized.set_ptr(new_ptr);
                    }
                } else {
                    let new_ptr = self.do_alloc_slice::<E, T>(new_cap)?.cast();
                    nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), old_size);
                    fixed.initialized.set_ptr(new_ptr);
                }
            }
        }

        fixed.capacity = new_cap;
        Ok(())
    }

    fn shrink_fixed_vec_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        let old_ptr = fixed.as_non_null_ptr();
        let old_size = fixed.capacity * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = fixed.len() * T::SIZE; // its less than the capacity so this can't overflow

        unsafe {
            let is_last = if UP {
                nonnull::byte_add(old_ptr, old_size).cast() == self.chunk.get().pos()
            } else {
                old_ptr.cast() == self.chunk.get().pos()
            };

            if is_last {
                // we can only do something if this is the last allocation

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
                        nonnull::copy::<u8>(old_ptr.cast(), new_ptr.cast(), new_size);
                    } else {
                        nonnull::copy_nonoverlapping::<u8>(old_ptr.cast(), new_ptr.cast(), new_size);
                    }

                    self.chunk.get().set_pos(new_ptr.cast());
                    fixed.initialized.set_ptr(new_ptr);
                }

                fixed.capacity = fixed.len();
            }
        }
    }
}
