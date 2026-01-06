use crate::{
    BaseAllocator, BumpScope,
    settings::BumpAllocatorSettings,
    stats::Stats,
    traits::{BumpAllocator, MutBumpAllocatorCoreScope},
};

/// A bump allocator scope.
pub trait BumpAllocatorScope<'a>: BumpAllocator + MutBumpAllocatorCoreScope<'a> {
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    fn stats(&self) -> Stats<'a, Self::Allocator, Self::Settings>;
}

impl<'a, A, S> BumpAllocatorScope<'a> for BumpScope<'a, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    fn stats(&self) -> Stats<'a, Self::Allocator, Self::Settings> {
        self.chunk.get().stats()
    }
}
