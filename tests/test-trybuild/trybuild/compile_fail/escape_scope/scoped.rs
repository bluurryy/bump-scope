use bump_scope::{Bump, BumpAllocator};

fn escape_closure(bump: &mut Bump) {
    let mut escapee = None;

    bump.scoped(|scope| {
        escapee = Some(scope.alloc("escape?").into_ref());
    });

    dbg!(escapee);
}

fn main() {}
