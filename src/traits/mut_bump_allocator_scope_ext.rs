use crate::{alloc::Allocator, MutBumpAllocatorExt, MutBumpAllocatorScope};

/// A trait as a shorthand for <code>[MutBumpAllocatorScope]<'a> + [MutBumpAllocatorExt]</code>
pub trait MutBumpAllocatorScopeExt<'a>: MutBumpAllocatorScope<'a> + MutBumpAllocatorExt {}

impl<'a, A: MutBumpAllocatorScope<'a> + MutBumpAllocatorExt> MutBumpAllocatorScopeExt<'a> for A where
    for<'b> &'b mut A: Allocator
{
}
