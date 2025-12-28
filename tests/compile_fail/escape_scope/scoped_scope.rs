use bump_scope::MutBumpScope;

fn escape_closure(mut bump: MutBumpScope) {
    let mut escapee = None;

    bump.scoped(|scope| {
        escapee = Some(scope.alloc("escape?"));
    });

    dbg!(escapee);
}

fn main() {}
