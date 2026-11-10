use std::mem;

use bump_scope::{
    Bump, BumpScope,
    config::{BumpConfig, BumpConfigBuilder},
};

use crate::common::InstrumentedAllocator;

#[test]
#[should_panic = "can't change config: `const_constructible` becomes false and `self` has no chunks"]
fn test_panic_change_config_unallocated() {
    const OLD_C: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
    const NEW_C: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
    let bump: Bump<OLD_C> = Bump::const_new();
    let _: Bump<NEW_C> = bump.change_config();
}

#[test]
#[should_panic = "can't change config: `claimable` becomes false and `self.is_claimed()`"]
fn test_panic_change_config_claimed() {
    const OLD_C: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
    const NEW_C: BumpConfig = BumpConfigBuilder::new().claimable(false).build();
    let allocator = InstrumentedAllocator::new();

    let payload = std::panic::catch_unwind(|| {
        let bump: Bump<OLD_C, &InstrumentedAllocator> = Bump::new_in(&allocator);
        mem::forget(bump.claim());
        let _: Bump<NEW_C, &InstrumentedAllocator> = bump.change_config();
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
    let bump: Bump<OLD_C> = Bump::new();

    bump.claim().scoped(|bump| {
        let bump: BumpScope<OLD_C> = bump.by_value();
        mem::forget(bump.claim());
        let _: BumpScope<NEW_C> = bump.change_config();
    });
}
