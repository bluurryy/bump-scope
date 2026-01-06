mod bump_allocator;
mod bump_allocator_core;
mod bump_allocator_core_scope;
mod bump_allocator_scope;
pub(crate) mod bump_allocator_typed;
pub(crate) mod bump_allocator_typed_scope;
mod mut_bump_allocator_core;
mod mut_bump_allocator_core_scope;
pub(crate) mod mut_bump_allocator_typed;
pub(crate) mod mut_bump_allocator_typed_scope;

pub use bump_allocator::BumpAllocator;
pub use bump_allocator_core::BumpAllocatorCore;
pub use bump_allocator_core_scope::BumpAllocatorCoreScope;
pub use bump_allocator_scope::BumpAllocatorScope;
pub use bump_allocator_typed::BumpAllocatorTyped;
pub use bump_allocator_typed_scope::BumpAllocatorTypedScope;
pub use mut_bump_allocator_core::MutBumpAllocatorCore;
pub use mut_bump_allocator_core_scope::MutBumpAllocatorCoreScope;
pub use mut_bump_allocator_typed::MutBumpAllocatorTyped;
pub use mut_bump_allocator_typed_scope::MutBumpAllocatorTypedScope;

macro_rules! assert_dyn_compatible {
    ($($tt:tt)*) => {
        const _: () = {
            #[expect(dead_code)]
            fn assert_dyn_compatible(_: &dyn $($tt)*) {}
        };
    };
}

pub(crate) use assert_dyn_compatible;

macro_rules! assert_implements {
    ([$($what:tt)*] $($ty:ty)*) => {
        #[cfg(test)]
        const _: () = {
            #[expect(unused_imports)]
            use crate::{
                alloc::Allocator,
                traits::{
                    BumpAllocatorCoreScope,
                    MutBumpAllocatorCore,
                    MutBumpAllocatorCoreScope,
                }
            };

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
