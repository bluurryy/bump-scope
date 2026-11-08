#![cfg(feature = "std")]

use bump_scope::{Bump, BumpAllocator as _};

thread_local! {
    static BUMP: Bump = const { Bump::const_new() };
}

fn main() {
    BUMP.with(|bump| {
        let hello = bump.alloc_str("hello").into_ref();
        assert_eq!(bump.allocated(), 5);

        bump.claim().scoped(|bump| {
            let world = bump.alloc_str("world").into_ref();
            assert_eq!(bump.allocated(), 10);
            println!("{hello} {world}");
        });

        assert_eq!(bump.allocated(), 5);
    });
}
