use std::{
    alloc::{AllocError, Allocator, Layout},
    cell::RefCell,
    collections::{HashMap, HashSet},
    ptr::NonNull,
};

pub struct ExpectLeaks<A: Allocator> {
    allocations: RefCell<HashMap<NonNull<u8>, Layout>>,
    allocator: A,
    how_many: usize,
}

impl<A: Allocator + Default> ExpectLeaks<A> {
    pub fn new(how_many: usize) -> Self {
        Self {
            allocations: Default::default(),
            allocator: Default::default(),
            how_many,
        }
    }
}

impl<A: Allocator> ExpectLeaks<A> {
    pub fn new_in(how_many: usize, allocator: A) -> Self {
        Self {
            allocations: Default::default(),
            allocator,
            how_many,
        }
    }
}

unsafe impl<A: Allocator> Allocator for ExpectLeaks<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = self.allocator.allocate(layout)?;
        assert!(self.allocations.borrow_mut().insert(ptr.cast(), layout).is_none());
        Ok(ptr)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        assert!(self.allocations.borrow_mut().remove(&ptr).is_some());
        unsafe { self.allocator.deallocate(ptr, layout) };
    }
}

impl<A: Allocator> Drop for ExpectLeaks<A> {
    fn drop(&mut self) {
        assert_eq!(self.how_many, self.allocations.get_mut().len());

        // do actually deallocate what would have been leaks, so miri is happy
        for (ptr, layout) in self.allocations.get_mut().drain() {
            unsafe { self.allocator.deallocate(ptr, layout) };
        }
    }
}
