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
    pub(crate) const NEW: Self = Self(PhantomData);
}

unsafe impl Allocator for Unmanaged<'_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        _ = layout;
        Err(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        _ = (ptr, layout);
    }
}
