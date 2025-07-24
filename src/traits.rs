use crate::alloc::Allocator;

mod bump_allocator;
pub(crate) mod bump_allocator_ext;
mod bump_allocator_scope;
pub(crate) mod bump_allocator_scope_ext;
mod mut_bump_allocator;
pub(crate) mod mut_bump_allocator_ext;
mod mut_bump_allocator_scope;
pub(crate) mod mut_bump_allocator_scope_ext;

pub use bump_allocator::BumpAllocator;
pub use bump_allocator_ext::BumpAllocatorExt;
pub use bump_allocator_scope::BumpAllocatorScope;
pub use bump_allocator_scope_ext::BumpAllocatorScopeExt;
pub use mut_bump_allocator::MutBumpAllocator;
pub use mut_bump_allocator_ext::MutBumpAllocatorExt;
pub use mut_bump_allocator_scope::MutBumpAllocatorScope;
pub use mut_bump_allocator_scope_ext::MutBumpAllocatorScopeExt;

macro_rules! assert_dyn_compatible {
    ($($tt:tt)*) => {
        const _: () = {
            #[allow(dead_code)]
            fn assert_dyn_compatible(_: &dyn $($tt)*) {}
        };
    };
}

pub(crate) use assert_dyn_compatible;

macro_rules! assert_implements {
    ([$($what:tt)*] $($ty:ty)*) => {
        const _: () = {
            #[allow(dead_code)]
            type A = crate::alloc::NoopAllocator;
            #[allow(dead_code)]
            type Bump = crate::Bump<A>;
            #[allow(dead_code)]
            type BumpScope<'a> = crate::BumpScope<'a, A>;
            #[allow(clippy::extra_unused_lifetimes)]
            const fn implements<'a, What: $($what)*>() {}
            $(
                #[allow(clippy::mut_mut)]
                implements::<$ty>();
            )*
        };
    };
}

pub(crate) use assert_implements;

assert_implements! {
    [Allocator + ?Sized]

    Bump
    &Bump
    &&Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &BumpScope
    &&BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}
