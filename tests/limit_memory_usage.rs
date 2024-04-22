#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
#![cfg(feature = "alloc")]

use std::{alloc::Layout, cell::Cell, ptr::NonNull};

use allocator_api2::alloc::{AllocError, Allocator};

use allocator_api2::alloc::Global;

use bump_scope::Bump;

struct Limited<A> {
    current: Cell<usize>,
    limit: usize,
    allocator: A,
}

impl<A> Limited<A> {
    pub fn new_in(limit: usize, allocator: A) -> Self {
        Self {
            current: Cell::new(0),
            limit,
            allocator,
        }
    }

    fn add(&self, size: usize) -> Result<(), AllocError> {
        let new = self.current.get() + size;

        if new > self.limit {
            return Err(AllocError);
        }

        self.current.set(new);

        Ok(())
    }

    fn sub(&self, size: usize) {
        self.current.set(self.current.get() - size);
    }
}

unsafe impl<A> Allocator for Limited<A>
where
    A: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.add(layout.size())?;
        self.allocator.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.sub(layout.size());
        self.allocator.deallocate(ptr, layout)
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.add(layout.size())?;
        self.allocator.allocate_zeroed(layout)
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.add(new_layout.size() - old_layout.size())?;
        self.allocator.grow(ptr, old_layout, new_layout)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.add(new_layout.size() - old_layout.size())?;
        self.allocator.grow_zeroed(ptr, old_layout, new_layout)
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.sub(old_layout.size() - new_layout.size());
        self.allocator.shrink(ptr, old_layout, new_layout)
    }
}

#[test]
fn main() {
    let allocator = Limited::new_in(1024, Global);

    let bump = Bump::<_, 1, true>::with_size_in(1024, &allocator);

    // limit is reached, trying to allocate any new chunk will fail
    // note that a bump `with_size` of 1024 results in a capacity of (1024 - SOME_HEADER_DATA_SIZE)
    bump.try_reserve_bytes(1024).unwrap_err();
}
