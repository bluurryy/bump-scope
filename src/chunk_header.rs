use core::{cell::Cell, num::NonZero, ptr::NonNull};

use crate::{bumping::MIN_CHUNK_ALIGN, polyfill::non_null, settings::BumpAllocatorSettings};

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

const UNALLOCATED_START: NonNull<u8> = non_null::without_provenance(NonZero::new(MIN_CHUNK_ALIGN * 2).unwrap());
const UNALLOCATED_END: NonNull<u8> = non_null::without_provenance(NonZero::new(MIN_CHUNK_ALIGN).unwrap());

static UNALLOCATED_CHUNK_HEADER_UP: UnallocatedChunkHeader = UnallocatedChunkHeader(ChunkHeader {
    pos: Cell::new(UNALLOCATED_START),
    end: UNALLOCATED_END,
    prev: Cell::new(None),
    next: Cell::new(None),
    allocator: (),
});

static UNALLOCATED_CHUNK_HEADER_DOWN: UnallocatedChunkHeader = UnallocatedChunkHeader(ChunkHeader {
    pos: Cell::new(UNALLOCATED_END),
    end: UNALLOCATED_START,
    prev: Cell::new(None),
    next: Cell::new(None),
    allocator: (),
});

const UNALLOCATED_UP: NonNull<ChunkHeader> = non_null::from_ref(&UNALLOCATED_CHUNK_HEADER_UP.0);
const UNALLOCATED_DOWN: NonNull<ChunkHeader> = non_null::from_ref(&UNALLOCATED_CHUNK_HEADER_DOWN.0);

impl ChunkHeader {
    pub(crate) const fn unallocated<S: BumpAllocatorSettings>() -> NonNull<ChunkHeader> {
        if S::UP { UNALLOCATED_UP } else { UNALLOCATED_DOWN }
    }
}
