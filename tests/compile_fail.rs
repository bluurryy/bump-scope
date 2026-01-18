#[test]
#[cfg(not(miri))]
// so the diff doesn't change too often
#[ignore = "make sure this runs on the stable channel"]
fn mustnt_compile() {
    let t = trybuild::TestCases::new();

    // Here we add an empty test.
    //
    // This is a workaround so trybuild uses `cargo build` instead of `cargo check`
    // which triggers const eval errors in `compile_fail`, like `const { panic!("OOF") }`,
    // which we use in `settings_conversion`.
    //
    // See <https://github.com/dtolnay/trybuild/issues/258>
    t.pass("tests/force_build.rs");
    t.pass("pass/**/*.rs");

    t.compile_fail("tests/compile_fail/**/*.rs");
}
