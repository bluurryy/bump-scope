use core::{cell::Cell, marker::PhantomData, mem::transmute, ptr::NonNull};

use crate::{
    chunk_types::concrete::{NonZstFirstAllocatedChunk, NonZstNonFirstAllocatedChunk},
    config::{BumpConfig, config},
};

#[repr(C, align(16))]
struct Header<A, const C: BumpConfig> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<NonZstAllocatedChunk<A, C>>,
    next: Cell<Option<NonZstAllocatedChunk<A, C>>>,

    allocator_ptr: NonNull<A>,
}

#[repr(transparent)]
pub(crate) struct NonZstAllocatedChunk<A, const C: BumpConfig> {
    header: NonNull<Header<A, C>>,
    marker: PhantomData<*const A>,
}

impl<A, const C: BumpConfig> Clone for NonZstAllocatedChunk<A, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const C: BumpConfig> Copy for NonZstAllocatedChunk<A, C> {}

impl<A, const C: BumpConfig> NonZstAllocatedChunk<A, C> {
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

    pub(crate) fn kind(self) -> NonZstAllocatedChunkKind<A, C> {
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
                allocator_ptr: prev.allocator_ptr(),
            });

            Self {
                header,
                marker: PhantomData,
            }
        }
    }
}

pub(crate) enum NonZstAllocatedChunkKind<A, const C: BumpConfig> {
    First(#[expect(dead_code)] NonZstFirstAllocatedChunk<A, C>),
    NonFirst(#[expect(dead_code)] NonZstNonFirstAllocatedChunk<A, C>),
}
