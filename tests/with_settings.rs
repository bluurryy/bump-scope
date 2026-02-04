#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::mem;

use bump_scope::{
    Bump, BumpScope,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type A = bump_scope::alloc::Global;

#[test]
#[should_panic = "bump allocator is unallocated"]
fn test_panic_with_settings_unallocated() {
    type S = BumpSettings;
    type NewS = <S as BumpAllocatorSettings>::WithGuaranteedAllocated<true>;
    let bump: Bump<A, S> = Bump::new();
    let _: Bump<A, NewS> = bump.with_settings();
}

#[test]
#[should_panic = "bump allocator is claimed"]
fn test_panic_with_settings_claimed() {
    type S = BumpSettings;
    type NewS = <S as BumpAllocatorSettings>::WithClaimable<false>;
    let bump: Bump<A, S> = Bump::new();
    mem::forget(bump.claim());
    let _: Bump<A, NewS> = bump.with_settings();
}

#[test]
#[should_panic = "bump allocator is claimed"]
fn test_panic_scope_with_settings_claimed() {
    type S = BumpSettings;
    type NewS = <S as BumpAllocatorSettings>::WithClaimable<false>;
    let bump: Bump<A, S> = Bump::new();

    bump.claim().scoped(|bump| {
        let bump: BumpScope<A, S> = bump.by_value();
        mem::forget(bump.claim());
        let _: BumpScope<A, NewS> = bump.with_settings();
    });
}
