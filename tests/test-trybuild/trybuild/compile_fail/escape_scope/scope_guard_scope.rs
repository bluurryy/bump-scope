use bump_scope::{BumpAllocator, BumpScope};

#[expect(unused_assignments)]
fn escape_closure(mut bump: BumpScope) {
    let mut escapee = None;

    {
        let mut guard = bump.begin_scope();
        let scope = guard.scope();

        escapee = Some(scope.alloc("escape?").into_ref());
    }

    dbg!(escapee);
}

fn main() {}
