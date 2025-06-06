use bump_scope::Bump;

#[allow(unused_assignments)]
fn escape_closure(bump: &mut Bump) {
    let mut escapee = None;

    {
        let mut guard = bump.scope_guard();
        let scope = guard.scope();

        escapee = Some(scope.alloc("escape?"));
    }

    dbg!(escapee);
}

fn main() {}
