use std::{
    alloc::Layout,
    cell::{Ref, RefCell},
    collections::HashMap,
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

use bump_scope::alloc::{AllocError, Allocator};

#[derive(Default)]
pub(crate) struct InstrumentedAllocator<A: Allocator> {
    allocator: A,
    allocations: RefCell<HashMap<NonNull<u8>, Layout>>,
}

impl<A: Allocator> UnwindSafe for InstrumentedAllocator<A> where A: UnwindSafe {}
impl<A: Allocator> RefUnwindSafe for InstrumentedAllocator<A> where A: RefUnwindSafe {}

impl<A: Allocator> InstrumentedAllocator<A> {
    pub(crate) fn new(allocator: A) -> Self {
        Self {
            allocator,
            allocations: Default::default(),
        }
    }

    pub(crate) fn leaks(&self) -> Ref<'_, HashMap<NonNull<u8>, Layout>> {
        self.allocations.borrow()
    }
}

impl<A: Allocator> Drop for InstrumentedAllocator<A> {
    fn drop(&mut self) {
        for (ptr, layout) in self.allocations.get_mut().drain() {
            unsafe { self.allocator.deallocate(ptr, layout) }
        }
    }
}

unsafe impl<A: Allocator> Allocator for InstrumentedAllocator<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = self.allocator.allocate(layout)?.cast();
        self.allocations.borrow_mut().insert(ptr, layout);
        // don't return a bigger slice than requested
        let slice = unsafe { NonNull::slice_from_raw_parts(ptr, layout.size()) };
        Ok(slice)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let found = self.allocations.borrow_mut().remove(&ptr).expect("foreign ptr");
        unsafe { self.allocator.deallocate(ptr, layout) };
    }
}
