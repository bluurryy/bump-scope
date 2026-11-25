use core::{cell::Cell, fmt, mem::transmute, ptr::NonNull};

use crate::{
    chunk_types::{CLAIMED_POS, CONST_CONSTRUCTED_POS, Chunk, dummy_memory_region},
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
    pub(crate) const INVALID: Self = Self {
        header: NonNull::dangling(),
    };

    #[inline(always)]
    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.header.cast()
    }

    #[inline(always)]
    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.pos.get()
    }

    #[expect(unused)]
    pub(crate) fn end(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.end
    }

    #[inline(always)]
    pub(crate) fn is_dummy(self) -> bool {
        dummy_memory_region::is_dummy_pos(self.pos().addr().get())
    }

    #[inline(always)]
    pub(crate) fn is_const_constructed(self) -> bool {
        self.pos() == CONST_CONSTRUCTED_POS
    }

    #[inline(always)]
    pub(crate) fn is_claimed(self) -> bool {
        self.pos() == CLAIMED_POS
    }

    #[inline(always)]
    pub(crate) fn erase<const C: BumpConfig>(chunk: Chunk<C>) -> Self {
        unsafe { transmute(chunk) }
    }
}

impl fmt::Debug for ErasedChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.header, f)
    }
}
