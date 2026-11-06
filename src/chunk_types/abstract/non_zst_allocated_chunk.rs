use core::{cell::Cell, marker::PhantomData, mem::transmute, ptr::NonNull};

use crate::{
    chunk_types::concrete::{NonZstFirstAllocatedChunk, NonZstNonFirstAllocatedChunk},
    config::{BumpConfig, config},
};

#[repr(C, align(16))]
struct Header<const C: BumpConfig, A> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<NonZstAllocatedChunk<C, A>>,
    next: Cell<Option<NonZstAllocatedChunk<C, A>>>,

    allocator_ptr: NonNull<A>,
}

#[repr(transparent)]
pub(crate) struct NonZstAllocatedChunk<const C: BumpConfig, A> {
    header: NonNull<Header<C, A>>,
    marker: PhantomData<*const A>,
}

impl<const C: BumpConfig, A> Clone for NonZstAllocatedChunk<C, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const C: BumpConfig, A> Copy for NonZstAllocatedChunk<C, A> {}

impl<const C: BumpConfig, A> NonZstAllocatedChunk<C, A> {
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

    pub(crate) fn allocator_ptr(self) -> NonNull<A> {
        unsafe { self.header.as_ref() }.allocator_ptr
    }

    pub(crate) fn kind(self) -> NonZstAllocatedChunkKind<C, A> {
        unsafe {
            if self.prev().is_some() {
                NonZstAllocatedChunkKind::NonFirst(transmute(self))
            } else {
                NonZstAllocatedChunkKind::First(transmute(self))
            }
        }
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
                allocator_ptr: prev.allocator_ptr(),
            });

            Self {
                header,
                marker: PhantomData,
            }
        }
    }
}

pub(crate) enum NonZstAllocatedChunkKind<const C: BumpConfig, A> {
    First(#[expect(dead_code)] NonZstFirstAllocatedChunk<C, A>),
    NonFirst(#[expect(dead_code)] NonZstNonFirstAllocatedChunk<C, A>),
}
