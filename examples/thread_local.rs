#![cfg(feature = "alloc")]
#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]

use bump_scope::{
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type Bump = bump_scope::Bump<Global, <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>>;

thread_local! {
    static BUMP: Bump = const { Bump::new_in(Global) };
}

fn main() {
    BUMP.with(|bump| {
        let hello = bump.alloc_str("hello");
        assert_eq!(bump.stats().allocated(), 5);

        bump.claim().scoped(|bump| {
            let world = bump.alloc_str("world");
            assert_eq!(bump.stats().allocated(), 10);
            println!("{hello} {world}");
        });

        assert_eq!(bump.stats().allocated(), 5);
    });
}
