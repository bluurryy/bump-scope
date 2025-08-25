#[test]
#[cfg(not(miri))]
// so the diff doesn't change too often
#[ignore = "make sure this runs on the stable channel"]
fn mustnt_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/**/*.rs");
}
