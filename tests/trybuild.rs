#[test]
#[cfg(not(miri))]
#[ignore = "make sure this runs on the stable channel so the diff doesn't change too often"]
fn trybuild() {
    if cfg!(miri) {
        panic!("not meant to be run with miri");
    }

    let t = trybuild::TestCases::new();

    // Here we add an empty test.
    //
    // This is a workaround so trybuild uses `cargo build` instead of `cargo check`
    // which triggers const eval errors in `compile_fail`, like `const { panic!("OOF") }`,
    // which we need to test methods like `change_config`.
    //
    // See <https://github.com/dtolnay/trybuild/issues/258>
    t.pass("tests/trybuild/force_build.rs");

    t.compile_fail("tests/trybuild/compile_fail/**/*.rs");
}
