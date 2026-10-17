use core::{alloc::Layout, cell::Cell, marker::PhantomData, ptr::NonNull};

use crate::config::{BumpConfig, config};

#[repr(C, align(16))]
struct Header<A, const C: BumpConfig> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<ZstAllocatedChunk<A, C>>,
    next: Cell<Option<ZstAllocatedChunk<A, C>>>,
}

#[repr(transparent)]
pub(crate) struct ZstAllocatedChunk<A, const C: BumpConfig> {
    header: NonNull<Header<A, C>>,
    marker: PhantomData<*const A>,
}

impl<A, const C: BumpConfig> Clone for ZstAllocatedChunk<A, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const C: BumpConfig> Copy for ZstAllocatedChunk<A, C> {}

impl<A, const C: BumpConfig> ZstAllocatedChunk<A, C> {
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

    #[expect(clippy::unused_self)]
    pub(crate) fn allocator_ptr(self) -> NonNull<A> {
        NonNull::dangling()
    }

    pub(crate) unsafe fn init(ptr: NonNull<u8>, size: usize, prev: Self) -> Self {
        unsafe {
            let header = if config!(upwards) {
                ptr.cast::<Header<A, C>>()
            } else {
                ptr.add(size).cast::<Header<A, C>>().sub(1)
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
