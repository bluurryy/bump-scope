use bump_scope::BumpScope;

#[allow(unused_assignments)]
fn escape_closure(mut bump: BumpScope) {
  let mut escapee = None;

  {
    let mut guard = bump.scope_guard();
    let scope = guard.scope();

    escapee = Some(scope.alloc("escape?"));
  }

  dbg!(escapee);
}

fn main() {}