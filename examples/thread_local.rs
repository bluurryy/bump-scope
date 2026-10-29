#![cfg(feature = "std")]

use std::alloc::Global;

use bump_scope::{
    BumpAllocator as _,
    config::{BumpConfig, BumpConfigBuilder},
};

const BUMP_CONFIG: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();

type Bump = bump_scope::Bump<Global, BUMP_CONFIG>;

thread_local! {
    static BUMP: Bump = const { Bump::const_new() };
}

fn main() {
    BUMP.with(|bump| {
        let hello = bump.alloc_str("hello").into_ref();
        assert_eq!(bump.stats().allocated(), 5);

        bump.claim().scoped(|bump| {
            let world = bump.alloc_str("world").into_ref();
            assert_eq!(bump.stats().allocated(), 10);
            println!("{hello} {world}");
        });

        assert_eq!(bump.stats().allocated(), 5);
    });
}
