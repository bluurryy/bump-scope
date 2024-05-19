#![feature(pointer_is_aligned_to, strict_provenance, allocator_api)]
#![allow(clippy::cargo_common_metadata)]

use std::{alloc::Layout, cell::Cell, ops::Deref, ptr::NonNull, rc::Rc};

use bump_scope::allocator_api2::alloc::{AllocError, Allocator};

pub use arbitrary;

pub mod allocator_api;
pub mod bumping;

#[derive(Debug, Clone)]
struct RcAllocator<A> {
    inner: Rc<A>,
}

impl<A> RcAllocator<A> {
    pub fn new(inner: Rc<A>) -> Self {
        Self { inner }
    }
}

impl<A> Deref for RcAllocator<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

unsafe impl<A> Allocator for RcAllocator<A>
where
    A: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.inner.deallocate(ptr, layout)
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate_zeroed(layout)
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.grow(ptr, old_layout, new_layout)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.grow_zeroed(ptr, old_layout, new_layout)
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.shrink(ptr, old_layout, new_layout)
    }
}

#[derive(Debug, Clone)]
struct MaybeFailingAllocator<A> {
    pub inner: A,
    pub fails: Cell<bool>,
}

impl<A> MaybeFailingAllocator<A> {
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            fails: Cell::new(false),
        }
    }
}

unsafe impl<A> Allocator for MaybeFailingAllocator<A>
where
    A: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        self.inner.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.inner.deallocate(ptr, layout)
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        self.inner.allocate_zeroed(layout)
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        self.inner.grow(ptr, old_layout, new_layout)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        self.inner.grow_zeroed(ptr, old_layout, new_layout)
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        self.inner.shrink(ptr, old_layout, new_layout)
    }
}

#[cfg(fuzzing_repro)]
pub use std::{dbg, eprint, eprintln};

#[cfg(not(fuzzing_repro))]
#[macro_export]
macro_rules! dbg {
    ($($tt:tt)*) => {};
}

#[cfg(not(fuzzing_repro))]
#[macro_export]
macro_rules! eprint {
    ($($tt:tt)*) => {};
}

#[cfg(not(fuzzing_repro))]
#[macro_export]
macro_rules! eprintln {
    ($($tt:tt)*) => {};
}
