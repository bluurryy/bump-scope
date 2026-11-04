use bump_scope::{Bump, BumpAllocator};

#[expect(unused_assignments)]
fn escape_closure(bump: &mut Bump) {
    let mut escapee = None;

    {
        let mut guard = bump.scope_guard();
        let scope = guard.scope();

        escapee = Some(scope.alloc("escape?").into_ref());
    }

    dbg!(escapee);
}

fn main() {}
