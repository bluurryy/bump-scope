//! These test cases were taken from the rust standard library.
#![cfg(test)]
#![feature(
    assert_matches,
    allocator_api,
    inplace_iteration,
    try_reserve_kind,
    drain_keep_rest,
    slice_partition_dedup,
    try_with_capacity,
    iter_array_chunks,
    iter_next_chunk,
    iter_advance_by,
    slice_ptr_get,
    default_field_values
)]
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
