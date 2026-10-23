use core::{
    alloc::{AllocError, Allocator, Layout},
    ptr::NonNull,
};

/// TODO
#[non_exhaustive]
pub struct Unmanaged;

unsafe impl Allocator for &Unmanaged {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        _ = layout;
        Err(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        _ = (ptr, layout);
    }
}
