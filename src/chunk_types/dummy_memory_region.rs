use core::ptr::NonNull;

/// The value for a chunk's position that identifies an "const constructor" chunk.
pub(crate) const CONST_CONSTRUCTOR_POS: NonNull<u8> =
    NonNull::new((&raw const DUMMY_MEMORY_REGION.const_constructor_pos).cast_mut().cast::<u8>()).unwrap();

/// The value for a chunk's position that identifies a "claimed" chunk.
pub(crate) const CLAIMED_POS: NonNull<u8> =
    NonNull::new((&raw const DUMMY_MEMORY_REGION.claimed_pos).cast_mut().cast::<u8>()).unwrap();

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
    const_constructor_pos: DummyMemoryRegionSlot,
    claimed_pos: DummyMemoryRegionSlot,
    end_padding: DummyMemoryRegionSlot,
}

static DUMMY_MEMORY_REGION: DummyMemoryRegion = DummyMemoryRegion {
    start_padding: DummyMemoryRegionSlot::new(),
    const_constructor_pos: DummyMemoryRegionSlot::new(),
    claimed_pos: DummyMemoryRegionSlot::new(),
    end_padding: DummyMemoryRegionSlot::new(),
};
