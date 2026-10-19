use crate::traits::{BumpAllocator, MutBumpAllocatorCore};

/// A shorthand for <code>[MutBumpAllocatorCore] + [BumpAllocator]</code>
pub trait MutBumpAllocator: MutBumpAllocatorCore + BumpAllocator {}

impl<A: MutBumpAllocatorCore + BumpAllocator + ?Sized> MutBumpAllocator for A {}
