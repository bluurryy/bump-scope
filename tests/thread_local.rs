#![cfg(feature = "alloc")]
#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]

use bump_scope::{
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type Bump = bump_scope::Bump<Global, <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>>;

thread_local! {
    static BUMP: Bump = const { Bump::unallocated() };
}

#[test]
fn main() {
    BUMP.with(|bump| {
        let hello = bump.alloc_str("hello");
        assert_eq!(hello, "hello");
    });
}
