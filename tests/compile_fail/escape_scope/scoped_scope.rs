use bump_scope::BumpScope;

fn escape_closure(bump: BumpScope) {
    let mut escapee = None;

    bump.scoped(|scope| {
        escapee = Some(scope.alloc("escape?"));
    });

    dbg!(escapee);
}

fn main() {}
