use core::{alloc::Allocator, ptr::NonNull};

use alloc::{boxed::Box, fmt};

use crate::{
    BoxU, NoDrop, Unmanaged,
    extra::{box_leak, replace_allocator},
    traits::BumpAllocatorScope,
};

/// TODO
#[must_use = "TODO"]
#[repr(transparent)]
pub struct BumpAllocation<T: ?Sized, A: Allocator> {
    boxed: Box<T, A>,
}

impl<T: fmt::Debug + ?Sized, A: Allocator> fmt::Debug for BumpAllocation<T, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Debug::fmt(&*self.boxed, f)
    }
}

impl<'a, T: ?Sized, A: Allocator> BumpAllocation<T, A> {
    pub(crate) fn new(boxed: Box<T, A>) -> Self {
        Self { boxed }
    }
}

impl<'a, T: ?Sized, A: BumpAllocatorScope<'a>> BumpAllocation<T, A> {
    pub(crate) fn wrap(boxu: BoxU<'a, T>, alloc: A) -> Self {
        Self {
            boxed: replace_allocator(boxu, alloc).0,
        }
    }

    /// TODO
    pub fn into_ref(self) -> &'a T
    where
        T: NoDrop,
    {
        self.leak()
    }

    /// TODO
    pub fn into_mut(self) -> &'a mut T
    where
        T: NoDrop,
    {
        self.leak()
    }

    /// TODO
    pub fn into_box(self) -> Box<T, A> {
        self.boxed
    }

    /// TODO
    pub fn into_boxu(self) -> Box<T, Unmanaged<'a>> {
        replace_allocator(self.boxed, Unmanaged::VALUE).0
    }

    /// TODO
    pub fn into_raw(self) -> NonNull<T> {
        let ptr = Box::into_raw_with_allocator(self.boxed).0;
        unsafe { NonNull::new_unchecked(ptr) }
    }

    /// TODO
    pub fn leak(self) -> &'a mut T {
        box_leak(self.boxed)
    }
}

pub struct IntoRef<T: ?Sized, A> {
    ptr: *const T,
    #[expect(unused)]
    alloc: A,
}

impl<T: ?Sized, A> IntoRef<T, A> {
    pub(crate) unsafe fn new(ptr: *const T, alloc: A) -> Self {
        Self { ptr, alloc }
    }
}

impl<'a, T: ?Sized, A> IntoRef<T, A>
where
    A: BumpAllocatorScope<'a>,
{
    /// TODO
    pub fn into_ref(self) -> &'a T
    where
        T: NoDrop,
    {
        unsafe { &*self.ptr }
    }
}
