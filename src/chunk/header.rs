//! A chunk header lives at
//! - the start of the chunk when upwards bumping
//! - the end of the chunk when downwards bumping
//!
//! All non-`Cell` fields of a chunk header are immutable.

use core::{cell::Cell, ptr::NonNull};

/// The special chunk header that's only in the first allocated chunk.
/// It extends [`AllocatedChunkHeader`] with a field storing the allocator itself.
#[repr(C, align(16))]
pub(crate) struct NonZstFirstAllocatedChunkHeader<A> {
    pub(crate) pos: Cell<NonNull<u8>>,
    pub(crate) end: NonNull<u8>,

    pub(crate) prev: Cell<Option<NonNull<Self>>>,
    pub(crate) next: Cell<Option<NonNull<Self>>>,

    pub(crate) allocator_ptr: NonNull<A>,
    pub(crate) allocator: A,
}

/// The chunk header existing in every allocated chunk, extending `ChunkHeader` with
/// intrusive doubly linked list fields and its allocator.
#[repr(C, align(16))]
pub(crate) struct NonZstAllocatedChunkHeader<A> {
    pub(crate) pos: Cell<NonNull<u8>>,
    pub(crate) end: NonNull<u8>,

    pub(crate) prev: Cell<Option<NonNull<Self>>>,
    pub(crate) next: Cell<Option<NonNull<Self>>>,

    pub(crate) allocator_ptr: NonNull<A>,
}

/// Like [`AllocatedChunkHeader`], just without the `allocator` field.
#[repr(C, align(16))]
pub(crate) struct AllocatedChunkHeader {
    pub(crate) pos: Cell<NonNull<u8>>,
    pub(crate) end: NonNull<u8>,

    pub(crate) prev: Cell<Option<NonNull<Self>>>,
    pub(crate) next: Cell<Option<NonNull<Self>>>,
}

/// The chunk header existing in every chunk, describing the remaining region of memory that you can bump allocate in.
#[repr(C, align(16))]
pub(crate) struct ChunkHeader {
    pub(crate) pos: Cell<NonNull<u8>>,
    pub(crate) end: NonNull<u8>,
}

/// The chunk header for dummy chunks, note how `pos` is not a cell. Dummy chunks, being statics must not be written to anyhow
/// (even if the value stays the same, it's still a data race) so removing the `Cell` makes it so we can catch accidental
/// writes earlier via miri. // TODO: it's a presumption; make sure miri actually catches that
#[repr(C, align(16))]
pub(crate) struct DummyChunkHeader {
    pub(crate) pos: NonNull<u8>,
    pub(crate) end: NonNull<u8>,
}

/// The chunk header for the "unallocated" dummy chunk, extending the [`DummyChunkHeader`] with a default constructor for the base allocator.
#[repr(C, align(16))]
pub(crate) struct UnallocatedChunkHeader<T> {
    pub(crate) pos: NonNull<u8>,
    pub(crate) end: NonNull<u8>,
    pub(crate) default: fn() -> T,
}

unsafe impl Sync for DummyChunkHeader {}

#[repr(C, align(16))]
struct DummyMemoryRegionSlot([u8; 16]);

impl DummyMemoryRegionSlot {
    const fn new() -> Self {
        Self([0; _])
    }
}

#[repr(C, align(16))]
struct DummyMemoryRegion {
    start_padding: DummyMemoryRegionSlot,
    unallocated_pos: DummyMemoryRegionSlot,
    claimed_pos: DummyMemoryRegionSlot,
    end_padding: DummyMemoryRegionSlot,
}

static DUMMY_MEMORY_REGION: DummyMemoryRegion = DummyMemoryRegion {
    start_padding: DummyMemoryRegionSlot::new(),
    unallocated_pos: DummyMemoryRegionSlot::new(),
    claimed_pos: DummyMemoryRegionSlot::new(),
    end_padding: DummyMemoryRegionSlot::new(),
};

macro_rules! slot {
    ($slot:ident) => {
        unsafe { NonNull::new_unchecked((&raw const DUMMY_MEMORY_REGION.$slot).cast_mut().cast::<u8>()) }
    };
    ($slot:ident + 16) => {
        unsafe { NonNull::new_unchecked((&raw const DUMMY_MEMORY_REGION.$slot).cast_mut().cast::<u8>()).byte_add(16) }
    };
    ($slot:ident - 16) => {
        unsafe { NonNull::new_unchecked((&raw const DUMMY_MEMORY_REGION.$slot).cast_mut().cast::<u8>()).byte_sub(16) }
    };
}

impl ChunkHeader {
    #[inline(always)]
    pub(crate) fn is_unallocated(&self) -> bool {
        self.pos.get() == slot!(unallocated_pos)
    }

    #[inline(always)]
    pub(crate) fn is_claimed(&self) -> bool {
        self.pos.get() == slot!(claimed_pos)
    }
}

impl<T: Default + 'static> UnallocatedChunkHeader<T> {
    const UP_STATIC: &'static UnallocatedChunkHeader<T> = &UnallocatedChunkHeader {
        pos: slot!(unallocated_pos),
        end: slot!(unallocated_pos - 16),
        default: T::default,
    };

    const DOWN_STATIC: &'static UnallocatedChunkHeader<T> = &UnallocatedChunkHeader {
        pos: slot!(unallocated_pos),
        end: slot!(unallocated_pos + 16),
        default: T::default,
    };

    pub const UP: NonNull<UnallocatedChunkHeader<T>> = NonNull::from_ref(Self::UP_STATIC);
    pub const DOWN: NonNull<UnallocatedChunkHeader<T>> = NonNull::from_ref(Self::DOWN_STATIC);
}

impl DummyChunkHeader {
    const CLAIMED_UP_STATIC: &'static DummyChunkHeader = &DummyChunkHeader {
        pos: slot!(claimed_pos),
        end: slot!(claimed_pos - 16),
    };

    const CLAIMED_DOWN_STATIC: &'static DummyChunkHeader = &DummyChunkHeader {
        pos: slot!(claimed_pos),
        end: slot!(claimed_pos + 16),
    };

    pub const CLAIMED_UP: NonNull<DummyChunkHeader> = NonNull::from_ref(Self::CLAIMED_UP_STATIC);
    pub const CLAIMED_DOWN: NonNull<DummyChunkHeader> = NonNull::from_ref(Self::CLAIMED_DOWN_STATIC);
}
