use bump_scope::MutBumpScope;

#[expect(unused_assignments)]
fn escape_closure(mut bump: MutBumpScope) {
    let mut escapee = None;

    {
        let mut guard = bump.scope_guard();
        let scope = guard.scope();

        escapee = Some(scope.alloc("escape?"));
    }

    dbg!(escapee);
}

fn main() {}
