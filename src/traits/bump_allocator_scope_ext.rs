use crate::{traits::assert_implements, BumpAllocatorExt, BumpAllocatorScope, MutBumpAllocatorScope};

/// A trait as a shorthand for <code>[BumpAllocatorScope] + [BumpAllocatorExt]<'a></code>
///
/// # Safety
///
/// See [`BumpAllocatorScope`] and [`BumpAllocatorExt`].
pub unsafe trait BumpAllocatorScopeExt<'a>: BumpAllocatorScope<'a> + BumpAllocatorExt {}

unsafe impl<'a, B> BumpAllocatorScopeExt<'a> for B where B: ?Sized + BumpAllocatorScope<'a> + BumpAllocatorExt {}

assert_implements! {
    [BumpAllocatorScopeExt<'a> + ?Sized]

    &Bump
    &BumpScope

    &mut Bump
    &mut BumpScope

    dyn BumpAllocatorScope
    &dyn BumpAllocatorScope
    &mut dyn BumpAllocatorScope

    dyn MutBumpAllocatorScope
    &dyn MutBumpAllocatorScope
    &mut dyn MutBumpAllocatorScope
}
