// this doesn't require nightly, but nightly can have a different error message
#[test]
#[cfg(all(not(any(miri, feature = "panic-on-alloc")), feature = "alloc"))]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild_unavailable_panicking_macros/**/*.rs");
}
