use bump_scope::{Bump, BumpScope};

#[expect(unused_assignments)]
fn multiple_scopes(bump: &mut Bump) {
    fn use_scope<'a>(scope: &'_ mut BumpScope<'a>) -> &'a str {
        scope.alloc_str("foo").into_ref()
    }

    let mut guard = bump.scope_guard();

    let a = use_scope(guard.scope());
    let b = use_scope(guard.scope());

    dbg!(a);
    dbg!(b);
}

fn main() {}
