#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{alloc::Global as A, mem};

use bump_scope::{
    Bump, BumpScope,
    config::{BumpConfig, BumpConfigBuilder},
};

use crate::common::InstrumentedAllocator;

#[test]
#[should_panic = "bump allocator is unallocated"]
fn test_panic_change_config_unallocated() {
    const OLD_C: BumpConfig = BumpConfigBuilder::new().guaranteed_allocated(false).build();
    const NEW_C: BumpConfig = BumpConfigBuilder::new().guaranteed_allocated(true).build();
    let bump: Bump<A, OLD_C> = Bump::unallocated();
    let _: Bump<A, NEW_C> = bump.change_config();
}

#[test]
#[should_panic = "bump allocator is claimed"]
fn test_panic_change_config_claimed() {
    const OLD_C: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
    const NEW_C: BumpConfig = BumpConfigBuilder::new().claimable(false).build();
    let allocator = InstrumentedAllocator::new(A);

    let payload = std::panic::catch_unwind(|| {
        let bump: Bump<&InstrumentedAllocator<A>, OLD_C> = Bump::new_in(&allocator);
        mem::forget(bump.claim());
        let _: Bump<&InstrumentedAllocator<A>, NEW_C> = bump.change_config();
    })
    .unwrap_err();

    assert_eq!(allocator.leaks().len(), 1);

    std::panic::resume_unwind(payload);
}

#[test]
#[should_panic = "`claim` is only available when `claimable` is enabled"]
fn test_panic_scope_change_config_claimed() {
    const OLD_C: BumpConfig = BumpConfigBuilder::new().claimable(false).build();
    const NEW_C: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
    let bump: Bump<A, OLD_C> = Bump::new();

    bump.claim().scoped(|bump| {
        let bump: BumpScope<A, OLD_C> = bump.by_value();
        mem::forget(bump.claim());
        let _: BumpScope<A, NEW_C> = bump.change_config();
    });
}
