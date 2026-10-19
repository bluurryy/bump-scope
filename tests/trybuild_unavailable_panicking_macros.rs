#[test]
#[cfg(not(miri))]
#[ignore = "make sure this runs on the stable channel so the diff doesn't change too often"]
fn trybuild() {
    if cfg!(miri) {
        panic!("not meant to be run with miri");
    }

    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild_unavailable_panicking_macros/**/*.rs");
}
