use allocator_api2::alloc::Global;

type Bump = crate::Bump<Global, 1, true, false>;

#[test]
fn initialized() {
    let bump: Bump = Bump::new();
    drop(bump);
}

#[test]
fn uninitialized() {
    let bump: Bump = Bump::uninit();
    drop(bump);
}

#[test]
fn initialized_by_usage() {
    let bump: Bump = Bump::uninit();
    bump.alloc_str("Hello World!");
    drop(bump);
}
