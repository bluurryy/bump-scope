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
struct DummyChunkHeader(ChunkHeader);

impl DummyChunkHeader {
    const fn new() -> Self {
        Self(ChunkHeader {
            pos: Cell::new(NonNull::<DummyChunkHeader>::dangling().cast()),
            end: NonNull::<DummyChunkHeader>::dangling().cast(),
            prev: Cell::new(None),
            next: Cell::new(None),
            allocator: (),
        })
    }
}

unsafe impl Sync for DummyChunkHeader {}

static UNALLOCATED_CHUNK_HEADER: DummyChunkHeader = DummyChunkHeader::new();
static DISABLED_CHUNK_HEADER: DummyChunkHeader = DummyChunkHeader::new();

impl ChunkHeader {
    /// Used to initialize an empty bump allocator.
    pub(crate) const UNALLOCATED: NonNull<ChunkHeader> = non_null::from_ref(&UNALLOCATED_CHUNK_HEADER.0);

    /// Temporarily replaces a scope's chunk to make allocations error while a child scope is active.
    pub(crate) const DISABLED: NonNull<ChunkHeader> = non_null::from_ref(&DISABLED_CHUNK_HEADER.0);
}
