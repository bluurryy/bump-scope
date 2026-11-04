//! Minimal string implementation for internal usage.
//! Would be replaced by `alloc::String` if that becomes generic over its allocator.
//!
//! For now we add a `ErrorBehavior` generic, just because it makes for simpler code.

use core::{
    marker::PhantomData,
    mem::{forget, replace},
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    SizedTypeProperties,
    error_behavior::ErrorBehavior,
    extra::{imp_vec_extend, imp_vec_into_boxed_slice_without_shrink},
    traits::BumpAllocatorPrivate,
};

#[repr(transparent)]
pub(crate) struct BumpVec<T, A, E>
where
    A: BumpAllocatorPrivate,
    E: ErrorBehavior,
{
    vec: Vec<T, A>,
    marker: PhantomData<fn() -> E>,
}

impl<T, A, E> BumpVec<T, A, E>
where
    A: BumpAllocatorPrivate,
    E: ErrorBehavior,
{
    #[inline]
    pub(crate) fn from_vec(vec: Vec<T, A>) -> Self {
        Self {
            vec,
            marker: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn reserve_if_fallible(&mut self, additional: usize) -> Result<(), E> {
        E::vec_reserve_if_fallible(&mut self.vec, additional)
    }

    #[inline]
    pub(crate) fn extend_from_slice(&mut self, slice: &[T]) -> Result<(), E>
    where
        T: Clone,
    {
        self.reserve_if_fallible(slice.len())?;
        self.vec.extend_from_slice(slice);
        Ok(())
    }

    #[inline]
    pub(crate) fn extend(&mut self, iter: impl IntoIterator<Item = T>) -> Result<(), E> {
        imp_vec_extend(self, iter)
    }

    #[inline]
    pub(crate) fn into_boxed_slice(mut self) -> Box<[T], A> {
        self.shrink_to_fit();
        imp_vec_into_boxed_slice_without_shrink(self.vec)
    }

    #[inline]
    pub(crate) fn shrink_to_fit(&mut self) {
        // SAFETY: Nothing in this block can panic.
        // We mess with the vectors memory by destructuring it and reconstructing
        // when we're done.
        unsafe {
            let (mut ptr, len, mut cap, alloc) = Vec::into_raw_parts_with_allocator(ptr::read(&raw mut self.vec));

            if !T::IS_ZST
                && len < cap
                && let Some(new_ptr) = alloc.spec_shrink_slice(NonNull::new_unchecked(ptr), cap, len)
            {
                ptr = new_ptr.as_ptr();
                cap = len;
            }

            forget(replace(&mut self.vec, Vec::from_raw_parts_in(ptr, len, cap, alloc)));
        }
    }
}

impl<T, A, E> Deref for BumpVec<T, A, E>
where
    A: BumpAllocatorPrivate,
    E: ErrorBehavior,
{
    type Target = Vec<T, A>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl<T, A, E> DerefMut for BumpVec<T, A, E>
where
    A: BumpAllocatorPrivate,
    E: ErrorBehavior,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
