#[test]
#[cfg(not(miri))]
// so the diff doesn't change too often
#[ignore = "make sure this runs on the stable channel"]
fn trybuild() {
    let t = trybuild::TestCases::new();

    // Here we add an empty test.
    //
    // This is a workaround so trybuild uses `cargo build` instead of `cargo check`
    // which triggers const eval errors in `compile_fail`, like `const { panic!("OOF") }`,
    // which we need to test methods like `with_settings`.
    //
    // See <https://github.com/dtolnay/trybuild/issues/258>
    t.pass("tests/trybuild/force_build.rs");
    t.pass("tests/trybuild/pass/**/*.rs");

    t.compile_fail("tests/trybuild/compile_fail/**/*.rs");
}
