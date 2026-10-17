use core::{alloc::Layout, cell::Cell, marker::PhantomData, ptr::NonNull};

use crate::config::BumpConfig;

#[repr(C, align(16))]
struct Header<A, const C: BumpConfig> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<NonZstNonFirstAllocatedChunk<A, C>>,
    next: Cell<Option<NonZstNonFirstAllocatedChunk<A, C>>>,

    allocator_ptr: NonNull<A>,
}

#[repr(transparent)]
pub(crate) struct NonZstNonFirstAllocatedChunk<A, const C: BumpConfig> {
    header: NonNull<Header<A, C>>,
    marker: PhantomData<*const A>,
}

impl<A, const C: BumpConfig> Clone for NonZstNonFirstAllocatedChunk<A, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const C: BumpConfig> Copy for NonZstNonFirstAllocatedChunk<A, C> {}

impl<A, const C: BumpConfig> NonZstNonFirstAllocatedChunk<A, C> {
    pub(crate) const HEADER_LAYOUT: Layout = Layout::new::<Header<A, C>>();

    #[expect(dead_code)]
    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.header.cast()
    }

    #[expect(dead_code)]
    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.pos.get()
    }

    #[expect(dead_code)]
    pub(crate) fn end(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.end
    }

    #[expect(dead_code)]
    pub(crate) fn prev(self) -> Option<Self> {
        unsafe { self.header.as_ref() }.prev
    }

    #[expect(dead_code)]
    pub(crate) fn next(self) -> Option<Self> {
        unsafe { self.header.as_ref() }.next.get()
    }

    #[expect(dead_code)]
    pub(crate) fn set_pos(self, pos: NonNull<u8>) {
        unsafe { self.header.as_ref() }.pos.set(pos);
    }

    #[expect(dead_code)]
    pub(crate) fn set_next(self, next: Option<Self>) {
        unsafe { self.header.as_ref() }.next.set(next);
    }

    #[expect(dead_code)]
    pub(crate) fn allocator_ptr(self) -> NonNull<A> {
        unsafe { self.header.as_ref() }.allocator_ptr
    }
}
