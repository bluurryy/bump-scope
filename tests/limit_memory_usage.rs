#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
#![cfg(feature = "alloc")]

use std::{alloc::Layout, cell::Cell, ptr::NonNull};

use bump_scope::{
    Bump,
    alloc::{AllocError, Allocator, Global},
};

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

    fn add(&self, size: usize) -> Result<usize, AllocError> {
        let new = match self.current.get().checked_add(size) {
            Some(some) => some,
            None => return Err(AllocError),
        };

        if new > self.limit {
            return Err(AllocError);
        }

        Ok(new)
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
        let new = self.add(layout.size())?;
        let ptr = self.allocator.allocate(layout)?;
        self.current.set(new);
        Ok(ptr)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe {
            self.sub(layout.size());
            self.allocator.deallocate(ptr, layout);
        }
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let new = self.add(layout.size())?;
        let ptr = self.allocator.allocate_zeroed(layout)?;
        self.current.set(new);
        Ok(ptr)
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            let new = self.add(new_layout.size() - old_layout.size())?;
            let ptr = self.allocator.grow(ptr, old_layout, new_layout)?;
            self.current.set(new);
            Ok(ptr)
        }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            let new = self.add(new_layout.size() - old_layout.size())?;
            let ptr = self.allocator.grow_zeroed(ptr, old_layout, new_layout)?;
            self.current.set(new);
            Ok(ptr)
        }
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            let ptr = self.allocator.shrink(ptr, old_layout, new_layout)?;
            self.sub(old_layout.size() - new_layout.size());
            Ok(ptr)
        }
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
