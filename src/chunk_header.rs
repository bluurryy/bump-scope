use core::{cell::Cell, ptr::NonNull};

#[repr(C, align(16))]
pub(crate) struct ChunkHeader<A> {
    pub pos: Cell<NonNull<u8>>,
    pub end: NonNull<u8>,

    pub prev: Option<NonNull<Self>>,
    pub next: Cell<Option<NonNull<Self>>>,

    pub allocator: A,
}
