#![cfg(feature = "panic-on-alloc")]
#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]

use core::{
    alloc::Layout,
    cell::Cell,
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
};
use std::sync::{Mutex, PoisonError};

use bump_scope::alloc::{AllocError, Allocator};

#[repr(C, align(16))]
struct StaticAllocator<const SIZE: usize> {
    memory: Cell<[MaybeUninit<u8>; SIZE]>,
    taken: Cell<bool>,
}

impl<const SIZE: usize> StaticAllocator<SIZE> {
    const fn new() -> Self {
        Self {
            memory: Cell::new([MaybeUninit::uninit(); SIZE]),
            taken: Cell::new(false),
        }
    }

    fn check_align(&self, align: usize) -> Result<(), AllocError> {
        if align <= mem::align_of::<Self>() {
            Ok(())
        } else {
            Err(AllocError)
        }
    }

    fn check_layout(&self, layout: Layout) -> Result<(), AllocError> {
        self.check_align(layout.align())?;

        if layout.size() <= SIZE { Ok(()) } else { Err(AllocError) }
    }

    fn memory_ptr(&self) -> NonNull<[u8]> {
        let ptr = self.memory.as_ptr().cast::<u8>();
        let slice = ptr::slice_from_raw_parts_mut(ptr, SIZE);
        NonNull::new(slice).unwrap()
    }
}

unsafe impl<const SIZE: usize> Allocator for StaticAllocator<SIZE> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.taken.get() {
            return Err(AllocError);
        }

        self.check_layout(layout)?;
        self.taken.set(true);
        Ok(self.memory_ptr())
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        self.taken.set(false);
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = self.allocate(layout)?;
        // SAFETY: `alloc` returns a valid memory block
        unsafe { ptr.cast::<u8>().write_bytes(0, ptr.len()) }
        Ok(ptr)
    }

    unsafe fn grow(&self, _ptr: NonNull<u8>, _old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.check_layout(new_layout)?;
        Ok(self.memory_ptr())
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.check_layout(new_layout)?;

        let zero_ptr = unsafe { ptr.add(old_layout.size()) };
        let zero_len = new_layout.size() - old_layout.size();

        unsafe {
            zero_ptr.write_bytes(0, zero_len);
        }

        Ok(self.memory_ptr())
    }

    unsafe fn shrink(
        &self,
        _ptr: NonNull<u8>,
        _old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.check_align(new_layout.align())?;
        Ok(self.memory_ptr())
    }
}

type Bump<A> = bump_scope::Bump<A, 1, true, true, true>;

fn on_stack() {
    let memory = StaticAllocator::<1024>::new();

    let bump = Bump::new_in(&memory);
    assert_eq!(bump.stats().size(), 1024);

    let str = bump.alloc_str("It works!");
    println!("{str}");

    bump.try_alloc_layout(Layout::new::<[u8; 2048]>()).unwrap_err();
}

fn on_static() {
    static MEMORY: Mutex<StaticAllocator<1024>> = Mutex::new(StaticAllocator::new());
    let guard = MEMORY.lock().unwrap_or_else(PoisonError::into_inner);
    let memory = &*guard;

    let bump = Bump::new_in(memory);
    assert_eq!(bump.stats().size(), 1024);

    let str = bump.alloc_str("It works!");
    println!("{str}");

    bump.try_alloc_layout(Layout::new::<[u8; 2048]>()).unwrap_err();
}

#[test]
fn main() {
    on_stack();
    on_static();
}
