use core::{
    alloc::Layout,
    cell::Cell,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

use crate::settings::BumpAllocatorSettings;

#[repr(C, align(16))]
struct Header<A, S> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<ZstFirstAllocatedChunk<A, S>>,
    next: Cell<Option<ZstFirstAllocatedChunk<A, S>>>,
}

#[repr(transparent)]
pub(crate) struct ZstFirstAllocatedChunk<A, S> {
    header: NonNull<Header<A, S>>,
    #[expect(clippy::type_complexity)]
    marker: PhantomData<(*const A, fn() -> S)>,
}

impl<A, S> Clone for ZstFirstAllocatedChunk<A, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for ZstFirstAllocatedChunk<A, S> {}

impl<A, S> ZstFirstAllocatedChunk<A, S>
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
