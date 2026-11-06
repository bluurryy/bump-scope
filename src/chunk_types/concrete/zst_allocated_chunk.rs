use core::{alloc::Layout, cell::Cell, marker::PhantomData, ptr::NonNull};

use crate::config::{BumpConfig, config};

#[repr(C, align(16))]
struct Header<const C: BumpConfig, A> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<ZstAllocatedChunk<C, A>>,
    next: Cell<Option<ZstAllocatedChunk<C, A>>>,
}

#[repr(transparent)]
pub(crate) struct ZstAllocatedChunk<const C: BumpConfig, A> {
    header: NonNull<Header<C, A>>,
    marker: PhantomData<*const A>,
}

impl<const C: BumpConfig, A> Clone for ZstAllocatedChunk<C, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const C: BumpConfig, A> Copy for ZstAllocatedChunk<C, A> {}

impl<const C: BumpConfig, A> ZstAllocatedChunk<C, A> {
    pub(crate) const HEADER_LAYOUT: Layout = Layout::new::<Header<C, A>>();

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

    #[expect(clippy::unused_self)]
    pub(crate) fn allocator_ptr(self) -> NonNull<A> {
        NonNull::dangling()
    }

    pub(crate) unsafe fn init(ptr: NonNull<u8>, size: usize, prev: Self) -> Self {
        unsafe {
            let header = if config!(upwards) {
                ptr.cast::<Header<C, A>>()
            } else {
                ptr.add(size).cast::<Header<C, A>>().sub(1)
            };

            let pos = Cell::new(if config!(upwards) {
                header.add(1).cast()
            } else {
                header.cast()
            });
            let end = if config!(upwards) { ptr.add(size) } else { ptr };

            header.write(Header {
                pos,
                end,
                prev: Some(prev),
                next: Cell::new(None),
            });

            Self {
                header,
                marker: PhantomData,
            }
        }
    }
}
