//! Concrete chunks are chunks with a dedicated header that gets written to the
//! start/end of the memory block.
//!
//! They each have a `HEADER_LAYOUT` member.

mod claimed_chunk;
mod const_constructor_chunk;
mod non_zst_first_allocated_chunk;
mod non_zst_non_first_allocated_chunk;
mod zst_allocated_chunk;
mod zst_first_allocated_chunk;

pub(crate) use claimed_chunk::ClaimedChunk;
pub(crate) use const_constructor_chunk::ConstConstructorChunk;
pub(crate) use non_zst_first_allocated_chunk::NonZstFirstAllocatedChunk;
pub(crate) use non_zst_non_first_allocated_chunk::NonZstNonFirstAllocatedChunk;
pub(crate) use zst_allocated_chunk::ZstAllocatedChunk;
pub(crate) use zst_first_allocated_chunk::ZstFirstAllocatedChunk;
