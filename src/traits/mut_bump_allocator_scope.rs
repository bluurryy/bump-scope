use crate::{BumpAllocatorScope, MutBumpAllocator};

/// A shorthand for <code>[MutBumpAllocator] + [BumpAllocatorScope]<'a></code>
pub trait MutBumpAllocatorScope<'a>: MutBumpAllocator + BumpAllocatorScope<'a> {}

impl<'a, B: MutBumpAllocator + BumpAllocatorScope<'a> + ?Sized> MutBumpAllocatorScope<'a> for B {}
