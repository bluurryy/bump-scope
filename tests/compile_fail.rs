// this doesn't require nightly, but nightly can have a different error message
#[cfg_attr(any(miri, not(feature = "nightly-tests")), ignore)]
#[test]
fn mustnt_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/**/*.rs");
}
