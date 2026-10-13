use core::{
    alloc::Layout,
    cell::Cell,
    marker::PhantomData,
    mem::offset_of,
    ptr::{self, NonNull},
};

use crate::settings::BumpAllocatorSettings;

#[repr(C, align(16))]
struct Header<A, S> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<NonZstFirstAllocatedChunk<A, S>>,
    next: Cell<Option<NonZstFirstAllocatedChunk<A, S>>>,

    allocator_ptr: NonNull<A>,
    allocator: A,
}

// Abstract chunk, either
#[repr(transparent)]
pub(crate) struct NonZstFirstAllocatedChunk<A, S> {
    header: NonNull<Header<A, S>>,
    #[expect(clippy::type_complexity)]
    marker: PhantomData<(*const A, fn() -> S)>,
}

impl<A, S> Clone for NonZstFirstAllocatedChunk<A, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for NonZstFirstAllocatedChunk<A, S> {}

impl<A, S> NonZstFirstAllocatedChunk<A, S>
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

    #[expect(dead_code)]
    pub(crate) fn allocator_ptr(self) -> NonNull<A> {
        unsafe { self.header.as_ref() }.allocator_ptr
    }

    pub(crate) fn read_allocator(self) -> A {
        unsafe { ptr::read(&raw const self.header.as_ref().allocator) }
    }

    pub(crate) unsafe fn init(ptr: NonNull<u8>, size: usize, allocator: A) -> Self {
        unsafe {
            let header = if S::UP {
                ptr.cast::<Header<A, S>>()
            } else {
                ptr.add(size).cast::<Header<A, S>>().sub(1)
            };

            let allocator_ptr = header.byte_add(offset_of!(Header<A, S>, allocator)).cast();
            let pos = Cell::new(if S::UP { header.add(1).cast() } else { header.cast() });
            let end = if S::UP { ptr.add(size) } else { ptr };

            header.cast::<Header<A, S>>().write(Header {
                pos,
                end,
                prev: None,
                next: Cell::new(None),
                allocator_ptr,
                allocator,
            });

            Self {
                header,
                marker: PhantomData,
            }
        }
    }
}
