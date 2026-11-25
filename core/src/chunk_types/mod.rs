//! Chunk types behaves somewhat like a `ManuallyDrop<T>`s in the sense that they has safe to
//! use methods that assume the chunk is still live.

mod allocated_chunk;
mod chunk;
mod dummy_chunk;
mod dummy_memory_region;
mod erased_chunk;

pub(crate) use allocated_chunk::AllocatedChunk;
pub(crate) use chunk::{Chunk, ChunkKind};
pub(crate) use dummy_chunk::DummyChunk;
pub(crate) use dummy_memory_region::{CLAIMED_POS, CONST_CONSTRUCTED_POS};
pub(crate) use erased_chunk::ErasedChunk;
