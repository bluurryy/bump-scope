use bump_scope::{BumpAllocator, BumpScope};

fn escape_closure(mut bump: BumpScope) {
    let mut escapee = None;

    bump.scoped(|scope| {
        escapee = Some(scope.alloc("escape?").into_ref());
    });

    dbg!(escapee);
}

fn main() {}
