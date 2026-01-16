use core::{cell::Cell, ptr::NonNull};

use crate::{polyfill::non_null, settings::BumpAllocatorSettings};

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
/// The dummy chunk is never mutated, so this is fine.
struct DummyChunkHeader(ChunkHeader);

unsafe impl Sync for DummyChunkHeader {}
/// We create a dummy chunks with a negative capacity, so all allocations will fail.
///
/// The pointers used for `pos` and `end` are chosen to be pointers into the same static dummy chunk.
///
/// It's irrelevant where the pointers point to, they just need to:
/// - be aligned to [`MIN_CHUNK_ALIGN`]
/// - denote a negative capacity (currently guaranteed to be -16)
/// - point to some existing object, not a dangling pointer since a dangling pointer could
///   theoretically be a valid pointer to some other chunk
static UNALLOCATED_CHUNK_HEADER_UP: DummyChunkHeader = DummyChunkHeader(ChunkHeader {
    // SAFETY: Due to `align(16)`, `ChunkHeader`'s size is `>= 16`, so a `byte_add` of 16 is in bounds.
    // We could also use `.add(1)` here, but we currently guarantee a capacity of -16
    pos: Cell::new(unsafe { UNALLOCATED_UP.cast().byte_add(16) }),
    end: UNALLOCATED_UP.cast(),
    prev: Cell::new(None),
    next: Cell::new(None),
    allocator: (),
});

/// See [`UNALLOCATED_CHUNK_HEADER_UP`].
static UNALLOCATED_CHUNK_HEADER_DOWN: DummyChunkHeader = DummyChunkHeader(ChunkHeader {
    pos: Cell::new(UNALLOCATED_DOWN.cast()),
    // SAFETY: Due to `align(16)`, `ChunkHeader`'s size is `>= 16`, so a `byte_add` of 16 is in bounds.
    // We could also use `.add(1)` here, but we currently guarantee a capacity of -16
    end: unsafe { UNALLOCATED_DOWN.cast().byte_add(16) },
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
