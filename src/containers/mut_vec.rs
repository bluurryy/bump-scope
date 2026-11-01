use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::ManuallyDrop,
    ptr::{self, NonNull},
    slice,
};

use alloc::{boxed::Box, vec::Vec};

use crate::{SizedTypeProperties, error_behavior::ErrorBehavior, extra::imp_vec_extend, traits::Sealed};

// We don't want the vector to try to deallocate since we didn't allocate anything in the first place
pub(crate) struct MutVec<'a, T, A: Sealed>(ManuallyDrop<Vec<T, AllocatorWrapper<'a, A>>>);

impl<'a, T, A: Sealed> MutVec<'a, T, A> {
    pub(crate) fn new<E>(min_cap: usize, alloc: &'a mut A) -> Result<Self, E>
    where
        E: ErrorBehavior,
    {
        let alloc = AllocatorWrapper(alloc);

        if T::IS_ZST {
            return Ok(Self(ManuallyDrop::new(Vec::new_in(alloc))));
        }

        let slice = alloc.0.spec_prepare_slice::<E, T>(min_cap)?;
        let cap = slice.len();
        let ptr = slice.as_ptr().cast::<T>();
        let vec = unsafe { Vec::from_raw_parts_in(ptr, 0, cap, alloc) };
        Ok(Self(ManuallyDrop::new(vec)))
    }

    #[expect(dead_code)]
    pub(crate) fn push<E>(&mut self, value: T) -> Result<(), E>
    where
        E: ErrorBehavior,
    {
        self.reserve(1)?;
        let len = self.0.len();
        let ptr = self.0.as_mut_ptr();
        unsafe {
            ptr.add(len).write(value);
            self.0.set_len(len + 1);
        }
        Ok(())
    }

    pub(crate) fn extend<E>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<(), E>
    where
        E: ErrorBehavior,
    {
        imp_vec_extend(&mut self.0, iter)
    }

    pub(crate) fn into_boxed_slice(mut self) -> Box<[T], &'a A> {
        unsafe {
            let vec = ManuallyDrop::take(&mut self.0);
            let (ptr, len, cap, AllocatorWrapper(alloc)) = Vec::into_raw_parts_with_allocator(vec);
            let slice = alloc.spec_commit_slice(NonNull::new_unchecked(ptr), len, cap);
            Box::from_raw_in(slice.as_ptr(), alloc)
        }
    }

    #[expect(dead_code)]
    pub(crate) fn reserve<E>(&mut self, additional: usize) -> Result<(), E>
    where
        E: ErrorBehavior,
    {
        E::vec_reserve(&mut self.0, additional)
    }

    pub(crate) fn reserve_if_fallible<E>(&mut self, additional: usize) -> Result<(), E>
    where
        E: ErrorBehavior,
    {
        E::vec_reserve_if_fallible(&mut self.0, additional)
    }

    pub(crate) fn extend_from_slice<E>(&mut self, slice: &[T]) -> Result<(), E>
    where
        E: ErrorBehavior,
        T: Clone,
    {
        self.reserve_if_fallible(slice.len())?;
        self.0.extend_from_slice(slice);
        Ok(())
    }

    pub(crate) fn iter(&self) -> slice::Iter<'_, T> {
        self.0.iter()
    }

    pub(crate) fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}

impl<T, A: Sealed> Drop for MutVec<'_, T, A> {
    fn drop(&mut self) {
        let len = self.0.len();
        let ptr = self.0.as_mut_ptr();
        unsafe { ptr::slice_from_raw_parts_mut(ptr, len).drop_in_place() };
    }
}

struct AllocatorWrapper<'a, A>(&'a A);

unsafe impl<A: Sealed> Allocator for AllocatorWrapper<'_, A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.allocate(layout)
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.allocate_zeroed(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        _ = (ptr, layout);
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.0.grow(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.0.grow_zeroed(ptr, old_layout, new_layout) }
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.0.shrink(ptr, old_layout, new_layout) }
    }
}
