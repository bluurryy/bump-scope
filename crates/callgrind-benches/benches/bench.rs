//! Callgrind benchmarks.
//!
//! The functions to benchmark are defined in `benches_lib`.
//! This crate just uses `iai-callgrind` to call them.
//!
//! Putting the benchmark implementations into a separate library and passing function pointers of
//! the library to itself is so that the `entry_point` filter works properly.

macro_rules! benches_library {
    ($library:ident $name:ident) => {
        paste::paste! {
             #[::iai_callgrind::library_benchmark(
                config = ::iai_callgrind::LibraryBenchmarkConfig::default()
                    .entry_point(concat!("entry_bench_", stringify!($name), "_", stringify!($library)).to_owned())
                    .callgrind_args(["branch-sim=yes"])

            )]
            pub fn $library() {
                benches_lib::[<bench_ $name _ $library>](benches_lib::[<entry_bench_ $name _ $library>]);
            }
        }
    };
}

macro_rules! benches {
    ($($name:ident)*) => {
        paste::paste! {
            $(
                pub mod [<bench_ $name>] {
                    benches_library! {
                        bump_scope_up $name
                    }

                    benches_library! {
                        bump_scope_down $name
                    }

                    benches_library! {
                        bumpalo $name
                    }

                    benches_library! {
                        blink_alloc $name
                    }

                    ::iai_callgrind::library_benchmark_group!(
                        name = $name;
                        benchmarks =
                            bump_scope_up,
                            bump_scope_down,
                            bumpalo,
                            blink_alloc,
                    );
                }
            )*

            $(
                use [<bench_ $name>]::$name;
            )*

            ::iai_callgrind::main!(library_benchmark_groups = $($name),*);
        }

    };
}

benches! {
    alloc_u8
    alloc_u8_overaligned
    try_alloc_u8
    try_alloc_u8_overaligned

    alloc_u32
    alloc_u32_aligned
    alloc_u32_overaligned
    try_alloc_u32
    try_alloc_u32_aligned
    try_alloc_u32_overaligned

    alloc_big_struct
    alloc_big_struct_aligned
    alloc_big_struct_overaligned
    try_alloc_big_struct
    try_alloc_big_struct_aligned
    try_alloc_big_struct_overaligned

    alloc_u8_slice
    alloc_u8_slice_overaligned
    try_alloc_u8_slice
    try_alloc_u8_slice_overaligned

    alloc_u32_slice
    alloc_u32_slice_aligned
    alloc_u32_slice_overaligned
    try_alloc_u32_slice
    try_alloc_u32_slice_aligned
    try_alloc_u32_slice_overaligned

    allocate
    grow_same_align
    grow_smaller_align
    grow_larger_align
    shrink_same_align
    shrink_smaller_align
    shrink_larger_align
    deallocate
    deallocate_non_last

    black_box_allocate
    black_box_grow_same_align
    black_box_grow_smaller_align
    black_box_grow_larger_align
    black_box_shrink_same_align
    black_box_shrink_smaller_align
    black_box_shrink_larger_align
    black_box_deallocate
    black_box_deallocate_non_last

    warm_up
    reset
}
