pub(crate) use std::{
    alloc::Layout,
    cell::{Ref, RefCell},
    collections::HashMap,
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

use bump_scope::alloc::{AllocError, Allocator};

/// An allocator wrapper that makes sure any allocation will be aligned *just*
/// as much as it is required and not more.
pub(crate) struct DisaligningAllocator<A: Allocator> {
    pub(crate) allocator: A,
    chunk_size: usize,
}

impl<A: Allocator + Default> Default for DisaligningAllocator<A> {
    fn default() -> Self {
        Self {
            allocator: A::default(),
            chunk_size: 1024,
        }
    }
}

impl<A: Allocator> UnwindSafe for DisaligningAllocator<A> where A: UnwindSafe {}
impl<A: Allocator> RefUnwindSafe for DisaligningAllocator<A> where A: RefUnwindSafe {}

impl<A: Allocator> DisaligningAllocator<A> {
    /// The allocator will only be able to handle allocations with an alignment `< max` and
    /// a size `< (max - layout.align())`.
    ///
    /// # Panics
    /// Panics if `max` is not a power of two.
    pub(crate) fn new(max: usize, allocator: A) -> Self {
        assert!(max.is_power_of_two());

        Self {
            chunk_size: max,
            allocator,
        }
    }

    fn chunk_layout(&self) -> Layout {
        Layout::from_size_align(self.chunk_size, self.chunk_size).unwrap()
    }
}

unsafe impl<A: Allocator> Allocator for DisaligningAllocator<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        assert!(layout.align() < self.chunk_size);
        assert!(layout.size() < (self.chunk_size - layout.align()));

        let slice = self.allocator.allocate(self.chunk_layout())?;

        unsafe {
            let ptr = slice.cast::<u8>().add(layout.align());
            Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, _: Layout) {
        unsafe {
            let ptr = ptr.as_ptr();
            let ptr = ptr.with_addr(down_align(ptr.addr(), self.chunk_size));
            let ptr = NonNull::new_unchecked(ptr);
            self.allocator.deallocate(ptr, self.chunk_layout());
        }
    }
}

fn down_align(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}
