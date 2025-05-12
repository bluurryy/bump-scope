use core::{alloc::Layout, ptr::NonNull};

use crate::{
    alloc::{AllocError, Allocator},
    polyfill::nonnull,
    BumpAllocator,
};

/// Wraps an bump allocator and does nothing on [`deallocate`](Allocator::deallocate).
///
/// This type only implements [`Allocator`] for wrapped types that implement [`BumpAllocator`], so you don't accidentally leak memory.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WithoutDealloc<A>(pub A);

impl<A> WithoutDealloc<A> {
    /// Wraps `self` in [`WithoutShrink`] so that [`shrink`] becomes a no-op.
    ///
    /// [`shrink`]: Allocator::shrink
    pub fn without_shrink(self) -> WithoutShrink<Self> {
        WithoutShrink(self)
    }
}

unsafe impl<A: BumpAllocator> Allocator for WithoutDealloc<A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.allocate(layout)
    }

    #[inline(always)]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.allocate_zeroed(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let _ = (ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.grow(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.0.grow_zeroed(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.shrink(ptr, old_layout, new_layout)
    }
}

/// Wraps an bump allocator and does nothing on [`shrink`](Allocator::shrink).
///
/// This type only implements [`Allocator`] for wrapped types that implement [`BumpAllocator`], so you don't accidentally leak memory.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WithoutShrink<A>(pub A);

impl<A> WithoutShrink<A> {
    /// Wraps `self` in [`WithoutDealloc`] so that [`deallocate`] becomes a no-op.
    ///
    /// [`deallocate`]: Allocator::deallocate
    pub fn without_dealloc(self) -> WithoutDealloc<Self> {
        WithoutDealloc(self)
    }
}

unsafe impl<A: BumpAllocator> Allocator for WithoutShrink<A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.allocate(layout)
    }

    #[inline(always)]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.allocate_zeroed(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.0.deallocate(ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.grow(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.0.grow_zeroed(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        #[cold]
        #[inline(never)]
        unsafe fn shrink_unfit<A: BumpAllocator>(
            this: &WithoutShrink<A>,
            ptr: NonNull<u8>,
            old_layout: Layout,
            new_layout: Layout,
        ) -> Result<NonNull<[u8]>, AllocError> {
            let new_ptr = this.0.allocate(new_layout)?.cast::<u8>();
            nonnull::copy_nonoverlapping(ptr, new_ptr, old_layout.size());
            Ok(nonnull::slice_from_raw_parts(new_ptr, new_layout.size()))
        }

        if nonnull::is_aligned_to(ptr, new_layout.align()) {
            Ok(nonnull::slice_from_raw_parts(ptr, new_layout.size()))
        } else {
            // expected to virtually never occur
            shrink_unfit(self, ptr, old_layout, new_layout)
        }
    }
}
