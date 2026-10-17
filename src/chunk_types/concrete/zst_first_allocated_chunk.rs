use core::{
    alloc::Layout,
    cell::Cell,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

use crate::config::{BumpConfig, config};

#[repr(C, align(16))]
struct Header<A, const C: BumpConfig> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<ZstFirstAllocatedChunk<A, C>>,
    next: Cell<Option<ZstFirstAllocatedChunk<A, C>>>,
}

#[repr(transparent)]
pub(crate) struct ZstFirstAllocatedChunk<A, const C: BumpConfig> {
    header: NonNull<Header<A, C>>,
    marker: PhantomData<*const A>,
}

impl<A, const C: BumpConfig> Clone for ZstFirstAllocatedChunk<A, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const C: BumpConfig> Copy for ZstFirstAllocatedChunk<A, C> {}

impl<A, const C: BumpConfig> ZstFirstAllocatedChunk<A, C> {
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

    #[expect(dead_code, clippy::unused_self)]
    pub(crate) fn allocator_ptr(self) -> NonNull<A> {
        NonNull::dangling()
    }

    #[expect(clippy::unused_self)]
    pub(crate) fn read_allocator(self) -> A {
        // SAFETY: a zst does not have an invalid value; we logically "stored"
        // an instance of `A` in this chunk which we now "read"
        #[allow(clippy::uninit_assumed_init)]
        unsafe {
            MaybeUninit::<A>::uninit().assume_init()
        }
    }

    pub(crate) unsafe fn init(ptr: NonNull<u8>, size: usize, allocator: A) -> Self {
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
                prev: None,
                next: Cell::new(None),
            });

            // We logically "store" the allocator in the chunk, which for a zst
            // just means that we don't drop it.
            _ = ManuallyDrop::new(allocator);

            Self {
                header,
                marker: PhantomData,
            }
        }
    }
}
