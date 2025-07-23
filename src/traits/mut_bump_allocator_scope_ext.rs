use crate::{MutBumpAllocatorExt, MutBumpAllocatorScope};

/// A trait as a shorthand for <code>[MutBumpAllocatorScope]<'a> + [MutBumpAllocatorExt]</code>
pub trait MutBumpAllocatorScopeExt<'a>: MutBumpAllocatorScope<'a> + MutBumpAllocatorExt {}

impl<'a, A: MutBumpAllocatorScope<'a> + MutBumpAllocatorExt + ?Sized> MutBumpAllocatorScopeExt<'a> for A {}
