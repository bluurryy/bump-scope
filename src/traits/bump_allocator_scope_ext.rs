use crate::{BumpAllocatorExt, BumpAllocatorScope, traits::assert_implements};

/// A shorthand for <code>[BumpAllocatorScope]<'a> + [BumpAllocatorExt]</code>
pub trait BumpAllocatorScopeExt<'a>: BumpAllocatorScope<'a> + BumpAllocatorExt {}

impl<'a, B> BumpAllocatorScopeExt<'a> for B where B: ?Sized + BumpAllocatorScope<'a> + BumpAllocatorExt {}

assert_implements! {
    [BumpAllocatorScopeExt<'a> + ?Sized]

    &Bump
    &MutBumpScope

    &mut Bump
    &mut MutBumpScope

    dyn BumpAllocatorScope
    &dyn BumpAllocatorScope
    &mut dyn BumpAllocatorScope

    dyn MutBumpAllocatorScope
    &dyn MutBumpAllocatorScope
    &mut dyn MutBumpAllocatorScope
}
