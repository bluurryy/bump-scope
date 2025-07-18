//! These test cases were taken from the rust standard library.
#![cfg(test)]
#![feature(allocator_api, slice_partition_dedup, iter_next_chunk, iter_advance_by)]
#![expect(
    clippy::assign_op_pattern,
    clippy::byte_char_slices,
    clippy::char_lit_as_u8,
    clippy::manual_range_patterns,
    clippy::non_canonical_clone_impl,
    clippy::op_ref,
    clippy::redundant_slicing,
    clippy::reversed_empty_ranges,
    clippy::uninlined_format_args,
    clippy::manual_is_multiple_of
)]

mod bump_string;
mod bump_vec;
mod mut_bump_vec;
mod mut_bump_vec_rev;
