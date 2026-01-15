mod header;
mod raw;
mod size;
mod size_config;

pub(crate) use header::ChunkHeader;
pub(crate) use raw::RawChunk;
pub(crate) use size::{ChunkSize, ChunkSizeHint};
pub(crate) use size_config::{ChunkSizeConfig, MIN_CHUNK_ALIGN};

#[cfg(all(test, feature = "std"))]
pub(crate) use size::AssumedMallocOverhead;
