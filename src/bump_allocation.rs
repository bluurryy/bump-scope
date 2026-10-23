use core::{alloc::Allocator, ptr::NonNull};

use alloc::boxed::Box;

use crate::{Unmanaged, extra::box_replace_allocator, traits::BumpAllocatorScope};

/// TODO
#[repr(transparent)]
pub struct BumpAllocation<T: ?Sized, A: Allocator> {
    boxed: Box<T, A>,
}

impl<'a, T: ?Sized, A: BumpAllocatorScope<'a>> BumpAllocation<T, A> {
    pub(crate) fn new(boxed: Box<T, A>) -> Self {
        Self { boxed }
    }

    /// TODO
    pub fn into_ref(self) -> &'a T {
        self.leak()
    }

    /// TODO
    pub fn into_mut(self) -> &'a mut T {
        self.leak()
    }

    /// TODO
    pub fn into_box(self) -> Box<T, A> {
        self.boxed
    }

    /// TODO
    pub fn into_boxu(self) -> Box<T, &'a Unmanaged> {
        box_replace_allocator(self.boxed, &Unmanaged).0
    }

    /// TODO
    pub fn into_raw(self) -> NonNull<T> {
        let ptr = Box::into_raw_with_allocator(self.boxed).0;
        unsafe { NonNull::new_unchecked(ptr) }
    }

    /// TODO
    pub fn leak(self) -> &'a mut T {
        // We can't use `Box::leak` here, since that requires a 'static Allocator.
        // SAFETY: The `BumpAllocatorScope<'a>` trait guarantees that allocations live for `'a`.
        unsafe { &mut *Box::into_raw_with_allocator(self.boxed).0 }
    }
}
