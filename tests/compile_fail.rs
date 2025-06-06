#[cfg_attr(miri, ignore)]
#[test]
fn mustnt_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/**/*.rs");
}
