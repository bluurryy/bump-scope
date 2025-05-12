#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]

use bump_scope::alloc::Global;

type Bump = bump_scope::Bump<Global, 1, true, false>;

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
