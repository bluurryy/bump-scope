use bump_scope::{Bump, BumpAllocator as _};

fn main() {
    let bump = Bump::<0>::new();
    _ = bump.alloc_str("hello");
}
