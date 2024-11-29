// I don't understand what's going on but trybuild doesn't work with that feature.
// It errors because of a missing `feature(allocator_api)` in hashbrown.
#![cfg(not(feature = "nightly-allocator-api"))]

#[cfg_attr(miri, ignore)]
#[test]
fn mustnt_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
