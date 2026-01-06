use crate::traits::{BumpAllocatorTyped, MutBumpAllocatorCore};

/// A shorthand for <code>[MutBumpAllocatorCore] + [BumpAllocatorTyped]</code>
pub trait MutBumpAllocatorTyped: MutBumpAllocatorCore + BumpAllocatorTyped {}

impl<A: MutBumpAllocatorCore + BumpAllocatorTyped + ?Sized> MutBumpAllocatorTyped for A {}
