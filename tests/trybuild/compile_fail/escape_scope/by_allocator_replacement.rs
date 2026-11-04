use bump_scope::{Bump, BumpAllocator, Unmanaged, extra::box_replace_allocator};

#[expect(unused_variables, unused_assignments)]
fn escape_by_allocator_replacement(bump: &mut Bump) {
    let unman: Unmanaged<'static> = Unmanaged::VALUE;
    let unshackled;

    {
        let mut guard = bump.scope_guard();
        let scope = guard.scope();

        let boxed = scope.alloc(3i32).into_box();
        unshackled = box_replace_allocator(boxed, unman);
    }
}

fn main() {}
