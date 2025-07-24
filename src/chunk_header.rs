use core::{cell::Cell, ptr::NonNull};

use crate::polyfill::non_null;

/// The chunk header that lives at
/// - the start of the allocation when upwards bumping
/// - the end of the allocation when downwards bumping
///
/// All non-`Cell` fields are immutable.
#[repr(C, align(16))]
pub(crate) struct ChunkHeader<A = ()> {
    pub(crate) pos: Cell<NonNull<u8>>,
    pub(crate) end: NonNull<u8>,

    pub(crate) prev: Cell<Option<NonNull<Self>>>,
    pub(crate) next: Cell<Option<NonNull<Self>>>,

    pub(crate) allocator: A,
}

/// Wraps a [`ChunkHeader`], making it Sync so it can be used as a static.
/// The empty chunk is never mutated, so this is fine.
struct UnallocatedChunkHeader(ChunkHeader);

unsafe impl Sync for UnallocatedChunkHeader {}

static UNALLOCATED_CHUNK_HEADER: UnallocatedChunkHeader = UnallocatedChunkHeader(ChunkHeader {
    pos: Cell::new(NonNull::<UnallocatedChunkHeader>::dangling().cast()),
    end: NonNull::<UnallocatedChunkHeader>::dangling().cast(),
    prev: Cell::new(None),
    next: Cell::new(None),
    allocator: (),
});

pub(crate) const fn unallocated_chunk_header() -> NonNull<ChunkHeader> {
    non_null::from_ref(&UNALLOCATED_CHUNK_HEADER.0)
}
