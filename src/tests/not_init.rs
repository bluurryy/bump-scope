use allocator_api2::alloc::Global;

type Bump = crate::Bump<Global, 1, true, false>;

#[test]
fn init() {
    let bump: Bump = Bump::new();
    drop(bump);
}

#[test]
fn uninit() {
    let bump: Bump = Bump::uninit();
    drop(bump);
}

#[test]
fn init_by_usage() {
    let bump: Bump = Bump::uninit();
    bump.alloc_str("Hello World!");
    drop(bump);
}

#[test]
fn into_init() {
    let bump: Bump = Bump::uninit();
    let bump = bump.into_init();
    assert!(bump.stats().size() > 0);
    drop(bump);
}
