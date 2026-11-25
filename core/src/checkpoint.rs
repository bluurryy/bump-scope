use core::num::NonZeroUsize;

use crate::{
    chunk_types::{Chunk, ErasedChunk},
    config::BumpConfig,
};

/// This is returned from `checkpoint` and used for `reset_to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checkpoint {
    pub(crate) chunk: ErasedChunk,
    pub(crate) address: NonZeroUsize,
}

impl Checkpoint {
    /// An invalid checkpoint.
    pub const INVALID: Self = {
        Self {
            chunk: ErasedChunk::INVALID,
            address: NonZeroUsize::MAX,
        }
    };

    pub(crate) fn new<const C: BumpConfig>(chunk: Chunk<C>) -> Self {
        let address = chunk.pos().addr();
        let chunk = ErasedChunk::erase(chunk);
        Checkpoint { chunk, address }
    }
}
