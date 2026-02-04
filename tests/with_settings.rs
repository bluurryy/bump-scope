#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::mem;

use bump_scope::{
    Bump, BumpScope,
    alloc::Global as A,
    settings::{BumpAllocatorSettings, BumpSettings},
};

use crate::common::InstrumentedAllocator;

#[test]
#[should_panic = "bump allocator is unallocated"]
fn test_panic_with_settings_unallocated() {
    type S = BumpSettings<1, true, false>;
    type NewS = <S as BumpAllocatorSettings>::WithGuaranteedAllocated<true>;
    let bump: Bump<A, S> = Bump::unallocated();
    let _: Bump<A, NewS> = bump.with_settings();
}

#[test]
#[should_panic = "bump allocator is claimed"]
fn test_panic_with_settings_claimed() {
    type S = BumpSettings;
    type NewS = <S as BumpAllocatorSettings>::WithClaimable<false>;
    let allocator = InstrumentedAllocator::new(A);

    let payload = std::panic::catch_unwind(|| {
        let bump: Bump<&InstrumentedAllocator<A>, S> = Bump::new_in(&allocator);
        mem::forget(bump.claim());
        let _: Bump<&InstrumentedAllocator<A>, NewS> = bump.with_settings();
    })
    .unwrap_err();

    assert_eq!(allocator.leaks().len(), 1);

    std::panic::resume_unwind(payload);
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
