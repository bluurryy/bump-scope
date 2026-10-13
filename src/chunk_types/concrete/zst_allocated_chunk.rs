use core::{alloc::Layout, cell::Cell, marker::PhantomData, ptr::NonNull};

use crate::settings::BumpAllocatorSettings;

#[repr(C, align(16))]
struct Header<A, S> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<ZstAllocatedChunk<A, S>>,
    next: Cell<Option<ZstAllocatedChunk<A, S>>>,
}

#[repr(transparent)]
pub(crate) struct ZstAllocatedChunk<A, S> {
    header: NonNull<Header<A, S>>,
    #[expect(clippy::type_complexity)]
    marker: PhantomData<(*const A, fn() -> S)>,
}

impl<A, S> Clone for ZstAllocatedChunk<A, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for ZstAllocatedChunk<A, S> {}

impl<A, S> ZstAllocatedChunk<A, S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) const HEADER_LAYOUT: Layout = Layout::new::<Header<A, S>>();

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
            let header = if S::UP {
                ptr.cast::<Header<A, S>>()
            } else {
                ptr.add(size).cast::<Header<A, S>>().sub(1)
            };

            let pos = Cell::new(if S::UP { header.add(1).cast() } else { header.cast() });
            let end = if S::UP { ptr.add(size) } else { ptr };

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
