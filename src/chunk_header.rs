use crate::polyfill::{cfg_const, nonnull};
use core::{cell::Cell, ptr::NonNull};

#[repr(C, align(16))]
pub(crate) struct ChunkHeader<A> {
    pub pos: Cell<NonNull<u8>>,
    pub end: NonNull<u8>,

    pub prev: Option<NonNull<Self>>,
    pub next: Cell<Option<NonNull<Self>>>,

    pub allocator: A,
}

/// Wraps a [`ChunkHeader`], making it Sync so it can be used as a static.
/// The empty chunk is never mutated, so this is fine.
struct UnallocatedChunkHeader(ChunkHeader<()>);

unsafe impl Sync for UnallocatedChunkHeader {}

static UNALLOCATED_CHUNK_HEADER: UnallocatedChunkHeader = UnallocatedChunkHeader(ChunkHeader {
    pos: Cell::new(NonNull::<UnallocatedChunkHeader>::dangling().cast()),
    end: NonNull::<UnallocatedChunkHeader>::dangling().cast(),
    prev: None,
    next: Cell::new(None),
    allocator: (),
});

cfg_const! {
    #[cfg_const(feature = "nightly-const-refs-to-static")]
    pub(crate) fn unallocated_chunk_header() -> NonNull<ChunkHeader<()>> {
        nonnull::from_ref(&UNALLOCATED_CHUNK_HEADER.0)
    }
}
