mod sealed {
    #[allow(clippy::wildcard_imports)]
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
    bump_down, polyfill::nonnull, up_align_usize_unchecked, Bump, BumpScope, ErrorBehavior, FixedBumpVec, MinimumAlignment,
    SizedTypeProperties, Stats, SupportedMinimumAlignment,
};

use allocator_api2::alloc::Allocator;

// This trait is intentionally not implemented for every `T: Sealed` so the implementors show up in the docs.
/// Any immutable bump scope.
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
        self.generic_fixed_vec_grow(fixed, additional)
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

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScopeRef<'a> for BumpScope<'a, A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
}

#[allow(private_bounds)]
impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Sealed<'a> for &BumpScope<'a, A, MIN_ALIGN, UP>
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
        BumpScope::generic_fixed_vec_grow(self, fixed, additional)
    }

    fn shrink_fixed_vec_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        BumpScope::generic_vec_shrink_to_fit(self, fixed);
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScopeRef<'a> for &BumpScope<'a, A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
}

#[allow(private_bounds)]
impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Sealed<'a> for &'a Bump<A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    type Stats<'b> = Stats<'b, UP>;
    type Allocator = A;

    fn stats(&self) -> Self::Stats<'a> {
        self.as_scope().stats()
    }

    fn allocator(&self) -> &Self::Allocator {
        self.as_scope().allocator()
    }

    fn alloc_fixed_vec<T, E>(&self, len: usize) -> Result<FixedBumpVec<'a, T>, E>
    where
        E: ErrorBehavior,
    {
        self.as_scope().generic_alloc_fixed_vec(len)
    }

    fn grow_fixed_vec<T, E>(&self, fixed: &mut FixedBumpVec<'a, T>, additional: usize) -> Result<(), E>
    where
        E: ErrorBehavior,
    {
        self.as_scope().grow_fixed_vec(fixed, additional)
    }

    fn shrink_fixed_vec_to_fit<T>(&self, fixed: &mut FixedBumpVec<'a, T>) {
        self.as_scope().shrink_fixed_vec_to_fit(fixed);
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScopeRef<'a> for &'a Bump<A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
}
