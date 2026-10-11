mod allocated_chunk;
#[expect(clippy::module_inception)]
mod chunk;
mod erased_chunk;
mod first_allocated_chunk;
mod header;
mod size;
mod size_config;
mod unallocated_chunk;

pub(crate) use allocated_chunk::AllocatedChunk;
pub(crate) use chunk::Chunk;
pub(crate) use erased_chunk::ErasedChunk;
pub(crate) use first_allocated_chunk::FirstAllocatedChunk;
pub(crate) use header::{
    AllocatedChunkHeader, ChunkHeader, DummyChunkHeader, NonZstAllocatedChunkHeader, NonZstFirstAllocatedChunkHeader,
    UnallocatedChunkHeader,
};
pub(crate) use size::{ChunkSize, ChunkSizeCapacity, ChunkSizeHint, ChunkSizeMinimum, align_allocation_size};
pub(crate) use size_config::{ChunkSizeConfig, MIN_CHUNK_ALIGN};
pub(crate) use unallocated_chunk::UnallocatedChunk;

use crate::settings::BumpAllocatorSettings;

pub(crate) enum ChunkClass<A, S: BumpAllocatorSettings> {
    Claimed,
    Unallocated(UnallocatedChunk<A, S>),
    Allocated(AllocatedChunk<A, S>),
}
