use core::{cell::Cell, ptr::NonNull};

use crate::{
    chunk_header::{unallocated_chunk_header, ChunkHeader},
    polyfill::non_null,
    stats::AnyStats,
    Checkpoint,
};

#[repr(transparent)]
pub struct BumpAllocatorChunks(pub(crate) Cell<NonNull<ChunkHeader>>);

impl BumpAllocatorChunks {
    /// See [`Bump::checkpoint`].
    pub fn checkpoint(&self) -> Checkpoint {
        unsafe { Checkpoint::from_header(self.0.get().cast()) }
    }

    /// See [`Bump::reset_to`].
    ///
    /// # Safety
    ///
    /// - all the safety conditions of [`Bump::reset_to`]
    /// - `self` must be allocated, see [`is_allocated`]
    /// - the checkpoint must have been created when self was allocated, see [`is_allocated`]
    ///
    /// [`is_allocated`]: Self::is_allocated
    pub unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        let ptr = non_null::with_addr(checkpoint.chunk.cast(), checkpoint.address);
        checkpoint.chunk.as_ref().pos.set(ptr);
        self.0.set(checkpoint.chunk.cast());
    }

    /// You need to provide the correct [`chunk_header_size`](BumpAllocator::chunk_header_size) for this bump allocator
    /// or else the sizes returned by the methods of [`AnyStats`] and [`AnyChunk`] will
    /// be incorrect.
    ///
    /// [`AnyChunk`]: crate::stats::AnyChunk
    pub fn stats(&self, chunk_header_size: usize) -> AnyStats<'_> {
        unsafe { AnyStats::from_header_unchecked(self.0.get(), chunk_header_size) }
    }

    /// Returns `true` when the bump allocator has no allocated chunk.
    ///
    /// This can only happen when the bump allocator is not [`GUARANTEED_ALLOCATED`](crate#guaranteed_allocated-parameter).
    pub fn is_allocated(&self) -> bool {
        self.0.get() != unallocated_chunk_header()
    }
}
