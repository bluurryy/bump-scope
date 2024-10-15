//! These test cases were taken from the rust standard library.
#![allow(
    clippy::pedantic,
    clippy::style,
    clippy::op_ref,
    clippy::redundant_slicing,
    clippy::reversed_empty_ranges,
    clippy::manual_range_patterns,
    clippy::non_canonical_clone_impl,
    clippy::char_lit_as_u8
)]

mod bump_string;
mod bump_vec;
mod mut_bump_vec;
mod mut_bump_vec_rev;
