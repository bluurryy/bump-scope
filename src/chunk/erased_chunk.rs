use core::{fmt, ptr::NonNull};

use crate::chunk::ChunkHeader;

/// Represents a chunk of the bump allocator without type information.
///
/// This type behaves somewhat like a `ManuallyDrop<T>` in the sense that it has safe to
/// use methods that assume the chunk is still live.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ErasedChunk {
    header: NonNull<ChunkHeader>,
}

impl ErasedChunk {
    #[inline(always)]
    pub(crate) unsafe fn from_raw(header: NonNull<ChunkHeader>) -> Self {
        Self { header }
    }

    #[inline(always)]
    pub(crate) fn header(self) -> NonNull<ChunkHeader> {
        self.header
    }

    #[inline(always)]
    pub(crate) fn end(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.end
    }

    #[inline(always)]
    pub(crate) fn is_claimed(self) -> bool {
        unsafe { self.header.as_ref().is_claimed() }
    }

    #[inline(always)]
    pub(crate) fn is_unallocated(self) -> bool {
        unsafe { self.header.as_ref().is_unallocated() }
    }
}

impl fmt::Debug for ErasedChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.header, f)
    }
}
