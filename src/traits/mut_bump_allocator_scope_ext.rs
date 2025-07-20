use crate::{alloc::Allocator, MutBumpAllocatorExt, MutBumpAllocatorScope};

/// A trait as a shorthand for <code>[MutBumpAllocatorScope] + [MutBumpAllocatorExt]<'a></code>
pub unsafe trait MutBumpAllocatorScopeExt<'a>: MutBumpAllocatorScope<'a> + MutBumpAllocatorExt {}

unsafe impl<'a, A: MutBumpAllocatorScope<'a> + MutBumpAllocatorExt> MutBumpAllocatorScopeExt<'a> for A where
    for<'b> &'b mut A: Allocator
{
}
