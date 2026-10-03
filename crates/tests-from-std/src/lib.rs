//! These test cases were taken from the rust standard library.
#![cfg(test)]
#![feature(slice_partition_dedup, iter_next_chunk, iter_advance_by)]
#![expect(
    clippy::assign_op_pattern,
    clippy::byte_char_slices,
    clippy::char_lit_as_u8,
    clippy::manual_range_patterns,
    clippy::op_ref,
    clippy::redundant_slicing,
    clippy::reversed_empty_ranges,
    clippy::uninlined_format_args,
    clippy::manual_is_multiple_of,
    clippy::redundant_closure
)]

mod bump_string;
mod bump_vec;
mod mut_bump_vec;
mod mut_bump_vec_rev;

macro_rules! struct_with_counted_drop {
    ($struct_name:ident $(( $( $elt_ty:ty ),+ ))?, $drop_counter:ident $( => $drop_stmt:expr )? ) => {
        thread_local! {static $drop_counter: ::core::cell::Cell<u32> = ::core::cell::Cell::new(0);}

        #[derive(Clone, Debug, PartialEq)]
        struct $struct_name $(( $( $elt_ty ),+ ))?;

        impl ::std::ops::Drop for $struct_name {
            fn drop(&mut self) {
                $drop_counter.set($drop_counter.get() + 1);

                $($drop_stmt(self))?
            }
        }
    };
    ($struct_name:ident $(( $( $elt_ty:ty ),+ ))?, $drop_counter:ident[ $drop_key:expr,$key_ty:ty ] $( => $drop_stmt:expr )? ) => {
        thread_local! {
            static $drop_counter: ::core::cell::RefCell<::std::collections::HashMap<$key_ty, u32>> =
                ::core::cell::RefCell::new(::std::collections::HashMap::new());
        }

        #[derive(Clone, Debug, PartialEq)]
        struct $struct_name $(( $( $elt_ty ),+ ))?;

        impl ::std::ops::Drop for $struct_name {
            fn drop(&mut self) {
                $drop_counter.with_borrow_mut(|counter| {
                    *counter.entry($drop_key(self)).or_default() += 1;
                });

                $($drop_stmt(self))?
            }
        }
    };
}

pub(crate) use struct_with_counted_drop;
