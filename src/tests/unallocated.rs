use allocator_api2::alloc::Global;

type Bump = crate::Bump<Global, 1, true, false>;

#[test]
fn allocated() {
    let bump: Bump = Bump::new();
    drop(bump);
}

#[test]
fn unallocated() {
    let bump: Bump = Bump::unallocated();
    drop(bump);
}

#[test]
fn allocated_by_usage() {
    let bump: Bump = Bump::unallocated();
    bump.alloc_str("Hello, World!");
    drop(bump);
}

#[test]
fn guaranteed_allocated() {
    let bump: Bump = Bump::unallocated();
    let bump = bump.guaranteed_allocated();
    assert!(bump.stats().size() > 0);
    drop(bump);
}

#[test]
fn allocated_reserve_bytes() {
    let bump: Bump = Bump::new();
    bump.reserve_bytes(1024);
    assert!(bump.stats().capacity() >= 1024);
    drop(bump);
}

#[test]
fn unallocated_reserve_bytes() {
    let bump: Bump = Bump::unallocated();
    bump.reserve_bytes(1024);
    assert!(bump.stats().capacity() >= 1024);
    drop(bump);
}
