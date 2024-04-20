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
struct EmptyChunkHeader(ChunkHeader<()>);

unsafe impl Sync for EmptyChunkHeader {}

static EMPTY_CHUNK_HEADER: EmptyChunkHeader = EmptyChunkHeader(ChunkHeader {
    pos: Cell::new(NonNull::<EmptyChunkHeader>::dangling().cast()),
    end: NonNull::<EmptyChunkHeader>::dangling().cast(),
    prev: None,
    next: Cell::new(None),
    allocator: (),
});

cfg_const! {
    #[cfg_const(feature = "nightly-const-refs-to-static")]
    pub(crate) fn empty_chunk_header() -> NonNull<ChunkHeader<()>> {
        nonnull::from_ref(&EMPTY_CHUNK_HEADER.0)
    }
}
