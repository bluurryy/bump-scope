use core::{cell::Cell, ptr::NonNull};

use crate::{
    chunk_header::{unallocated_chunk_header, ChunkHeader},
    polyfill::non_null,
    stats::AnyStats,
    Checkpoint,
};

/// Returned from [`BumpAllocator::chunks`].
///
/// [`BumpAllocator::chunks`]: crate::BumpAllocator::chunks
#[repr(transparent)]
pub struct BumpAllocatorChunks(pub(crate) Cell<NonNull<ChunkHeader>>);

impl BumpAllocatorChunks {
    /// Creates a checkpoint of the current bump position.
    ///
    /// The bump position can be reset to this checkpoint with [`reset_to`].
    ///
    /// [`reset_to`]: BumpAllocatorChunks::reset_to
    pub fn checkpoint(&self) -> Checkpoint {
        unsafe { Checkpoint::from_header(self.0.get().cast()) }
    }

    /// Resets the bump position to a previously created checkpoint.
    /// The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    ///
    /// - the checkpoint must have been created by this bump allocator
    /// - the bump allocator must not have been [`reset`] since creation of this checkpoint
    /// - there must be no references to allocations made since creation of this checkpoint
    /// - `self` must be allocated, see [`is_allocated`]
    /// - the checkpoint must have been created when self was allocated, see [`is_allocated`]
    ///
    /// [`is_allocated`]: Self::is_allocated
    /// [`reset`]: crate::Bump::reset
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate alloc;
    /// # use bump_scope::{Bump, BumpAllocator};
    /// # use alloc::alloc::Layout;
    /// fn test(bump: impl BumpAllocator) {
    ///     let checkpoint = bump.chunks().checkpoint();
    ///     
    ///     {
    ///         let hello = bump.allocate(Layout::new::<[u8;5]>()).unwrap();
    ///         assert_eq!(bump.chunks().stats(bump.chunk_header_size()).allocated(), 5);
    ///         # _ = hello;
    ///     }
    ///     
    ///     unsafe { bump.chunks().reset_to(checkpoint); }
    ///     assert_eq!(bump.chunks().stats(bump.chunk_header_size()).allocated(), 0);
    /// }
    ///
    /// test(<Bump>::new());
    /// ```
    pub unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        let ptr = non_null::with_addr(checkpoint.chunk.cast(), checkpoint.address);
        checkpoint.chunk.as_ref().pos.set(ptr);
        self.0.set(checkpoint.chunk.cast());
    }

    /// You need to provide the correct [`chunk_header_size`] for this bump allocator
    /// or else the sizes returned by the methods of [`AnyStats`] and [`AnyChunk`] will
    /// be incorrect.
    ///
    /// [`chunk_header_size`]: crate::BumpAllocator::chunk_header_size
    /// [`AnyChunk`]: crate::stats::AnyChunk
    pub fn stats(&self, chunk_header_size: usize) -> AnyStats<'_> {
        unsafe { AnyStats::from_header_unchecked(self.0.get(), chunk_header_size) }
    }

    /// Returns `true` when the bump allocator has an allocated chunk.
    ///
    /// This can only return `false` when the bump allocator is not [`GUARANTEED_ALLOCATED`].
    ///
    /// [`GUARANTEED_ALLOCATED`]: crate#guaranteed_allocated-parameter
    pub fn is_allocated(&self) -> bool {
        self.0.get() != unallocated_chunk_header()
    }
}
