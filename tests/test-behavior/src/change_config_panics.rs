use std::{alloc::Global, mem::forget};

use bump_scope::{
    AsMutBumpScope as _, Bump, BumpScope,
    config::{BumpConfig, BumpConfigBuilder},
};

use crate::common::ExpectLeaks;

mod test_bump {
    use super::*;

    #[test]
    #[should_panic = "can't change config: `const_constructible` becomes false and `self` has no chunks"]
    fn disable_const_constructible() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let input: Bump<IN> = Bump::const_new();
        let _: Bump<OUT> = input.change_config();
    }

    #[test]
    #[should_panic = "can't change config: `const_constructible` becomes false and `self` has no chunks"]
    fn disable_const_constructible_while_claimed() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let input: Bump<IN> = Bump::const_new();
        forget(input.claim());
        // here, we're not able to tell whether the bump allocator is "unallocated", so we need to panic
        let _: Bump<OUT> = input.change_config();
    }

    #[test]
    #[should_panic = "can't change config: `claimable` becomes false and `self.is_claimed()`"]
    fn disable_claimable() {
        const IN: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().claimable(false).build();
        let base = ExpectLeaks::<Global>::new(1);
        let input: Bump<IN, _> = Bump::new_in(&base);
        forget(input.claim());
        let _: Bump<OUT, _> = input.change_config();
    }
}

mod test_bump_scope {
    use super::*;

    #[test]
    #[should_panic = "can't change config: `claimable` becomes false and `self.is_claimed()`"]
    fn disable_claimable() {
        const IN: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().claimable(false).build();
        let mut input_bump: Bump<IN> = Bump::new();
        let input: BumpScope<IN> = input_bump.as_mut_bump_scope().by_value();
        forget(input.claim());
        let _: BumpScope<OUT> = input.change_config();
    }
}

mod test_borrow {
    use super::*;

    #[test]
    #[should_panic = "can't change config: `const_constructible` becomes false and `self` has no chunks"]
    fn disable_const_constructible() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let input: Bump<IN> = Bump::const_new();
        let _: &Bump<OUT> = input.ref_change_config();
    }

    #[test]
    #[should_panic = "can't change config: `const_constructible` becomes false and `self` has no chunks"]
    fn disable_const_constructible_while_claimed() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let input: Bump<IN> = Bump::const_new();
        forget(input.claim());
        // here, we're not able to tell whether the bump allocator is "unallocated", so we need to panic
        let _: &Bump<OUT> = input.ref_change_config();
    }
}

// test_borrow_mut intentionally left out, there are no panics for that

mod test_scoped_with_config {
    use super::*;

    #[test]
    #[should_panic = "can't change config: `const_constructible` becomes false and `self` has no chunks"]
    fn disable_const_constructible() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let mut input: Bump<IN> = Bump::const_new();
        input.scoped_with_config::<OUT, _>(|_| ());
    }

    #[test]
    #[should_panic = "can't change config: `const_constructible` becomes false and `self` has no chunks"]
    fn disable_const_constructible_while_claimed() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let mut input: Bump<IN> = Bump::const_new();
        forget(input.claim());
        // here, we're not able to tell whether the bump allocator is "unallocated", so we need to panic
        input.scoped_with_config::<OUT, _>(|_| ());
    }

    #[test]
    #[should_panic = "can't change config: `claimable` becomes false and `self.is_claimed()`"]
    fn disable_claimable() {
        const IN: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().claimable(false).build();
        let base = ExpectLeaks::<Global>::new(1);
        let mut input: Bump<IN, _> = Bump::new_in(&base);
        forget(input.claim());
        input.scoped_with_config::<OUT, _>(|_| ());
    }
}
