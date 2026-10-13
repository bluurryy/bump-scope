use core::{
    alloc::{Allocator, Layout},
    cell::Cell,
    marker::PhantomData,
    mem::transmute,
    ptr::NonNull,
};

use crate::{
    SizedTypeProperties,
    chunk_size::ChunkSize,
    chunk_types::{
        r#abstract::AllocatedChunk,
        concrete::{NonZstFirstAllocatedChunk, ZstFirstAllocatedChunk},
    },
    error_behavior::ErrorBehavior,
    settings::BumpAllocatorSettings,
};

#[repr(C, align(16))]
struct Header<A, S> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<FirstAllocatedChunk<A, S>>,
    next: Cell<Option<FirstAllocatedChunk<A, S>>>,
}

// Abstract chunk, either
#[repr(transparent)]
pub(crate) struct FirstAllocatedChunk<A, S> {
    header: NonNull<Header<A, S>>,
    #[expect(clippy::type_complexity)]
    marker: PhantomData<(*const A, fn() -> S)>,
}

impl<A, S> Clone for FirstAllocatedChunk<A, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for FirstAllocatedChunk<A, S> {}

impl<A, S> FirstAllocatedChunk<A, S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) const HEADER_LAYOUT: Layout = if A::IS_ZST {
        ZstFirstAllocatedChunk::<A, S>::HEADER_LAYOUT
    } else {
        NonZstFirstAllocatedChunk::<A, S>::HEADER_LAYOUT
    };

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

    pub(crate) fn kind(self) -> FirstAllocatedChunkKind<A, S> {
        unsafe {
            if A::IS_ZST {
                FirstAllocatedChunkKind::Zst(transmute(self))
            } else {
                FirstAllocatedChunkKind::NonZst(transmute(self))
            }
        }
    }

    #[expect(dead_code)]
    pub(crate) fn allocator_ptr(self) -> NonNull<A> {
        match self.kind() {
            FirstAllocatedChunkKind::Zst(chunk) => chunk.allocator_ptr(),
            FirstAllocatedChunkKind::NonZst(chunk) => chunk.allocator_ptr(),
        }
    }

    pub(crate) fn reset(self) {
        AllocatedChunk::from_first(self).reset();
    }

    pub(crate) fn read_allocator(self) -> A {
        match self.kind() {
            FirstAllocatedChunkKind::Zst(chunk) => chunk.read_allocator(),
            FirstAllocatedChunkKind::NonZst(chunk) => chunk.read_allocator(),
        }
    }

    pub(crate) unsafe fn assume_first(chunk: AllocatedChunk<A, S>) -> Self {
        unsafe { transmute(chunk) }
    }

    pub(crate) fn from_zst(zst: ZstFirstAllocatedChunk<A, S>) -> Self {
        unsafe { transmute(zst) }
    }

    pub(crate) fn from_non_zst(non_zst: NonZstFirstAllocatedChunk<A, S>) -> Self {
        unsafe { transmute(non_zst) }
    }

    pub(crate) fn new<E>(chunk_size: impl ChunkSize, allocator: A) -> Result<Self, E>
    where
        A: Allocator,
        E: ErrorBehavior,
    {
        let (ptr, size) = AllocatedChunk::<A, S>::allocate(chunk_size, &allocator)?;
        unsafe { Ok(FirstAllocatedChunk::init(ptr, size, allocator)) }
    }

    #[inline(always)]
    pub(crate) unsafe fn init(ptr: NonNull<u8>, size: usize, allocator: A) -> FirstAllocatedChunk<A, S> {
        if A::IS_ZST {
            FirstAllocatedChunk::from_zst(unsafe { ZstFirstAllocatedChunk::init(ptr, size, allocator) })
        } else {
            FirstAllocatedChunk::from_non_zst(unsafe { NonZstFirstAllocatedChunk::init(ptr, size, allocator) })
        }
    }
}

pub(crate) enum FirstAllocatedChunkKind<A, S> {
    Zst(ZstFirstAllocatedChunk<A, S>),
    NonZst(NonZstFirstAllocatedChunk<A, S>),
}
