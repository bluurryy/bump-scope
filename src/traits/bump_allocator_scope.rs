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

    /// Forwards to [`BumpScope::with_settings`].
    fn with_settings<NewS>(self) -> BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings;

    /// Forwards to [`BumpScope::borrow_with_settings`].
    fn borrow_with_settings<NewS>(&self) -> &BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings;

    /// Forwards to [`BumpScope::borrow_mut_with_settings`].
    fn borrow_mut_with_settings<NewS>(&mut self) -> &mut BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings;
}

impl<'a, A, S> BumpAllocatorScope<'a> for BumpScope<'a, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline]
    fn stats(&self) -> Stats<'a, Self::Allocator, Self::Settings> {
        self.chunk.get().stats()
    }

    #[inline]
    fn with_settings<NewS>(self) -> BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        BumpScope::with_settings(self)
    }

    #[inline]
    fn borrow_with_settings<NewS>(&self) -> &BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        BumpScope::borrow_with_settings(self)
    }

    #[inline]
    fn borrow_mut_with_settings<NewS>(&mut self) -> &mut BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings,
        Self: Sized,
    {
        BumpScope::borrow_mut_with_settings(self)
    }
}
