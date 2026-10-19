use std::alloc::Global;

use bump_scope::Bump;

fn main() {
    _ = Bump::<Global>::const_new();
}
