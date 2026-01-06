use crate::traits::{BumpAllocatorCoreScope, MutBumpAllocatorCore};

/// A shorthand for <code>[MutBumpAllocatorCore] + [BumpAllocatorCoreScope]<'a></code>
pub trait MutBumpAllocatorCoreScope<'a>: MutBumpAllocatorCore + BumpAllocatorCoreScope<'a> {}

impl<'a, B: MutBumpAllocatorCore + BumpAllocatorCoreScope<'a> + ?Sized> MutBumpAllocatorCoreScope<'a> for B {}
