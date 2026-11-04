//! Minimal string implementation for internal usage.
//! Would be replaced by `alloc::String` if that becomes generic over its allocator.
//!
//! For now we add a `ErrorBehavior` generic, just because it makes for simpler code.

use core::{
    alloc::Allocator,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use alloc::{boxed::Box, vec::Vec};

use crate::{error_behavior::ErrorBehavior, extra::imp_vec_extend};

#[repr(transparent)]
pub(crate) struct BumpVec<T, A, E>
where
    A: Allocator,
    E: ErrorBehavior,
{
    vec: Vec<T, A>,
    marker: PhantomData<fn() -> E>,
}

impl<T, A, E> BumpVec<T, A, E>
where
    A: Allocator,
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
    pub(crate) fn into_boxed_slice(self) -> Box<[T], A> {
        self.vec.into_boxed_slice()
    }
}

impl<T, A, E> Deref for BumpVec<T, A, E>
where
    A: Allocator,
    E: ErrorBehavior,
{
    type Target = Vec<T, A>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl<T, A, E> DerefMut for BumpVec<T, A, E>
where
    A: Allocator,
    E: ErrorBehavior,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
