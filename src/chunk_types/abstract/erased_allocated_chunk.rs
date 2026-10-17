use core::{cell::Cell, fmt, mem::transmute, ptr::NonNull};

use crate::{chunk_types::r#abstract::AllocatedChunk, config::BumpConfig};

#[repr(C, align(16))]
struct Header {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<ErasedAllocatedChunk>,
    next: Cell<Option<ErasedAllocatedChunk>>,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ErasedAllocatedChunk {
    header: NonNull<Header>,
}

impl ErasedAllocatedChunk {
    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.header.cast()
    }

    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.pos.get()
    }

    pub(crate) fn end(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.end
    }

    pub(crate) fn prev(self) -> Option<Self> {
        unsafe { self.header.as_ref() }.prev
    }

    pub(crate) fn next(self) -> Option<Self> {
        unsafe { self.header.as_ref() }.next.get()
    }

    #[inline(always)]
    pub(crate) fn erase<A, const C: BumpConfig>(chunk: AllocatedChunk<A, C>) -> Self {
        unsafe { transmute(chunk) }
    }
}

impl fmt::Debug for ErasedAllocatedChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.header, f)
    }
}
