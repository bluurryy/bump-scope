use alloc::rc::Rc;
use allocator_api2::alloc::{AllocError, Allocator, Global};
use core::{alloc::Layout, ops::Deref, ptr::NonNull};

use crate::{Bump, BumpAllocator, BumpAllocatorScope};

pub(crate) struct RcBump<A: Allocator + Clone = Global>(Rc<Bump<A>>);

impl<A: Allocator + Clone> Clone for RcBump<A> {
    fn clone(&self) -> Self {
        RcBump(self.0.clone())
    }
}

impl RcBump {
    pub(crate) fn new() -> Self {
        RcBump(Rc::new(Bump::new()))
    }
}

impl<A: Allocator + Clone> RcBump<A> {
    pub(crate) fn new_in(a: A) -> Self {
        RcBump(Rc::new(Bump::new_in(a)))
    }
}

impl<A: Allocator + Clone> Deref for RcBump<A> {
    type Target = Bump<A>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl<A: Allocator + Clone> Allocator for RcBump<A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Bump::allocate(&self.0, layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        Bump::deallocate(&self.0, ptr, layout);
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Bump::allocate_zeroed(&self.0, layout)
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Bump::grow(&self.0, ptr, old_layout, new_layout)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        Bump::grow_zeroed(&self.0, ptr, old_layout, new_layout)
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Bump::shrink(&self.0, ptr, old_layout, new_layout)
    }

    fn by_ref(&self) -> &Self
    where
        Self: Sized,
    {
        self
    }
}

unsafe impl<A: Allocator + Clone> BumpAllocator for RcBump<A> {
    fn stats(&self) -> crate::Stats<'_> {
        <Bump<A> as BumpAllocator>::stats(self)
    }

    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        Bump::allocate_layout(&self.0, layout)
    }

    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        Bump::try_allocate_layout(&self.0, layout)
    }

    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        Bump::allocate_sized(&self.0)
    }

    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        Bump::try_allocate_sized(&self.0)
    }

    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        Bump::allocate_slice(&self.0, len)
    }

    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        Bump::try_allocate_slice(&self.0, len)
    }

    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>>
    where
        Self: Sized,
    {
        Bump::shrink_slice(&self.0, ptr, old_len, new_len)
    }
}

unsafe impl<'a, A: Allocator + Clone> BumpAllocatorScope<'a> for &'a RcBump<A> {}
