use crate::{BumpAllocatorExt, MutBumpAllocator};

/// A shorthand for <code>[MutBumpAllocator] + [BumpAllocatorExt]</code>
pub trait MutBumpAllocatorExt: MutBumpAllocator + BumpAllocatorExt {}

impl<A: MutBumpAllocator + BumpAllocatorExt + ?Sized> MutBumpAllocatorExt for A {}
