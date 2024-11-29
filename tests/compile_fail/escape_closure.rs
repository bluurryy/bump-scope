#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
use bump_scope::Bump;

fn escape_closure(bump: &mut Bump) {
    let mut escapee = None;

    bump.scoped(|scope| {
        escapee = Some(scope.alloc("escape?"));
    });

    dbg!(escapee);
}

fn main() {}
