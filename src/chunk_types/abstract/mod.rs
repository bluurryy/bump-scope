//! Abstract chunks describe common behavior of different concrete chunks.

mod allocated_chunk;
mod chunk;
mod erased_chunk;
mod first_allocated_chunk;
mod non_zst_allocated_chunk;

pub(crate) use allocated_chunk::AllocatedChunk;
pub(crate) use chunk::{Chunk, ChunkKind};
pub(crate) use erased_chunk::ErasedChunk;
pub(crate) use first_allocated_chunk::FirstAllocatedChunk;
pub(crate) use non_zst_allocated_chunk::NonZstAllocatedChunk;
