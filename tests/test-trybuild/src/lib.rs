#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();

    // Here we add an empty test.
    //
    // This is a workaround so trybuild uses `cargo build` instead of `cargo check`
    // which triggers const eval errors in `compile_fail`, like `const { panic!("OOF") }`,
    // which we need to test methods like `change_config`.
    //
    // See <https://github.com/dtolnay/trybuild/issues/258>
    t.pass("trybuild/force_build.rs");

    t.compile_fail("trybuild/compile_fail/**/*.rs");
}
