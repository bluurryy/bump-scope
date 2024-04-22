#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
#![cfg(feature = "nightly-const-refs-to-static")]

use bump_scope::allocator_api2::alloc::Global;

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
