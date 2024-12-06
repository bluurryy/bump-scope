use crate::{BumpAllocatorScope, MutBumpAllocator};

/// Shorthand for <code>[MutBumpAllocator] + [BumpAllocatorScope]<'a></code>
pub trait MutBumpAllocatorScope<'a>: MutBumpAllocator + BumpAllocatorScope<'a> {}
impl<'a, T: MutBumpAllocator + BumpAllocatorScope<'a>> MutBumpAllocatorScope<'a> for T {}
