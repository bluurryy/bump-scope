#![cfg(feature = "std")]

mod common;

use std::{alloc::Global, mem::forget};

use bump_scope::{
    Bump, BumpScope,
    config::{BumpConfig, BumpConfigBuilder},
};

use crate::common::ExpectLeaks;

mod test_bump {
    use super::*;

    #[test]
    #[should_panic = "bump allocator is unallocated"]
    fn decrease_const_constructible() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let input: Bump<Global, IN> = Bump::const_new();
        let _: Bump<Global, OUT> = input.change_config();
    }

    #[test]
    #[should_panic = "bump allocator is unallocated"]
    fn decrease_const_constructible_while_claimed() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let input: Bump<Global, IN> = Bump::const_new();
        forget(input.claim());
        // here, we're not able to tell whether the bump allocator is "unallocated", so we need to panic
        let _: Bump<Global, OUT> = input.change_config();
    }

    #[test]
    #[should_panic = "bump allocator is claimed"]
    fn decrease_claimable() {
        const IN: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().claimable(false).build();
        let base = ExpectLeaks::<Global>::new(1);
        let input: Bump<_, IN> = Bump::new_in(&base);
        forget(input.claim());
        let _: Bump<_, OUT> = input.change_config();
    }
}

mod test_bump_scope {
    use super::*;

    #[test]
    #[should_panic = "bump allocator is claimed"]
    fn decrease_claimable() {
        const IN: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().claimable(false).build();
        let mut input_bump: Bump<Global, IN> = Bump::new();
        let input: BumpScope<_, IN> = input_bump.as_mut_scope().by_value();
        forget(input.claim());
        let _: BumpScope<_, OUT> = input.change_config();
    }
}

mod test_borrow {
    use super::*;

    #[test]
    #[should_panic = "bump allocator is unallocated"]
    fn decrease_const_constructible() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let input: Bump<Global, IN> = Bump::const_new();
        let _: &Bump<Global, OUT> = input.borrow_change_config();
    }

    #[test]
    #[should_panic = "bump allocator is unallocated"]
    fn decrease_const_constructible_while_claimed() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let input: Bump<Global, IN> = Bump::const_new();
        forget(input.claim());
        // here, we're not able to tell whether the bump allocator is "unallocated", so we need to panic
        let _: &Bump<Global, OUT> = input.borrow_change_config();
    }
}

// test_borrow_mut intentionally left out, there are no panics for that

mod test_scoped_with_config {
    use super::*;

    #[test]
    #[should_panic = "bump allocator is unallocated"]
    fn decrease_const_constructible() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let mut input: Bump<Global, IN> = Bump::const_new();
        input.scoped_with_config::<OUT, _>(|_| ());
    }

    #[test]
    #[should_panic = "bump allocator is unallocated"]
    fn decrease_const_constructible_while_claimed() {
        const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
        let mut input: Bump<Global, IN> = Bump::const_new();
        forget(input.claim());
        // here, we're not able to tell whether the bump allocator is "unallocated", so we need to panic
        input.scoped_with_config::<OUT, _>(|_| ());
    }

    #[test]
    #[should_panic = "bump allocator is claimed"]
    fn decrease_claimable() {
        const IN: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
        const OUT: BumpConfig = BumpConfigBuilder::new().claimable(false).build();
        let base = ExpectLeaks::<Global>::new(1);
        let mut input: Bump<_, IN> = Bump::new_in(&base);
        forget(input.claim());
        input.scoped_with_config::<OUT, _>(|_| ());
    }
}
