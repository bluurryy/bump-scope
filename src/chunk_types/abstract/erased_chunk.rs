use core::{cell::Cell, fmt, mem::transmute, ptr::NonNull};

use crate::{
    chunk_types::{r#abstract::Chunk, dummy_memory_region},
    config::BumpConfig,
};

#[repr(C, align(16))]
struct Header {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ErasedChunk {
    header: NonNull<Header>,
}

impl ErasedChunk {
    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.header.cast()
    }

    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.pos.get()
    }

    pub(crate) fn end(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.end
    }

    pub(crate) fn is_const_constructor(self) -> bool {
        self.pos() == dummy_memory_region::CONST_CONSTRUCTOR_POS
    }

    pub(crate) fn is_claimed(self) -> bool {
        self.pos() == dummy_memory_region::CLAIMED_POS
    }

    #[inline(always)]
    pub(crate) fn erase<A, const C: BumpConfig>(chunk: Chunk<A, C>) -> Self {
        unsafe { transmute(chunk) }
    }
}

impl fmt::Debug for ErasedChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.header, f)
    }
}
