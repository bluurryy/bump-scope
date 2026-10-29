use core::{
    alloc::{AllocError, Allocator, Layout},
    marker::PhantomData,
    ptr::NonNull,
};

/// TODO
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct Unmanaged<'a>(PhantomData<&'a ()>);

impl Unmanaged<'_> {
    pub(crate) const VALUE: Self = Self(PhantomData);
}

unsafe impl Allocator for Unmanaged<'_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        _ = layout;
        Err(AllocError)
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        _ = layout;
        Err(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        _ = (ptr, layout);
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        _ = (ptr, old_layout, new_layout);
        Err(AllocError)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        _ = (ptr, old_layout, new_layout);
        Err(AllocError)
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if new_layout.align() <= old_layout.align() {
            Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()))
        } else {
            Err(AllocError)
        }
    }
}
