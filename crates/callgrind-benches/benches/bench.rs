//! Callgrind benchmarks.
//!
//! The functions to benchmark are defined in `benches_lib`.
//! This crate just uses `gungraun` to call them.
//!
//! Putting the benchmark implementations into a separate library and passing function pointers of
//! the library to itself is so that the `entry_point` filter works properly.

macro_rules! benches_library {
    ($name:ident for $($library:ident)*) => {
        paste::paste! {
            pub mod [<bench_ $name>] {
                $(
                    #[::gungraun::library_benchmark(
                        config = ::gungraun::LibraryBenchmarkConfig::default()
                            .tool(::gungraun::Callgrind::default()
                                .entry_point(::gungraun::EntryPoint::Custom("entry_bench_*".to_owned()))
                                .args(["branch-sim=yes"])
                            )

                    )]
                    pub fn [<$name _ $library>]() {
                        benches_lib::[<bench_ $name _ $library>](benches_lib::[<entry_bench_ $name _ $library>]);
                    }
                )*

                ::gungraun::library_benchmark_group!(
                    name = $name;
                    benchmarks =
                        $([<$name _ $library>]),*
                );
            }
        }
    };
}

macro_rules! benches {
    ($($name:ident)*) => {
        paste::paste! {
            $(
                benches_library! {
                    $name for

                    bump_scope_up
                    bump_scope_down
                    bumpalo
                    blink_alloc
                }
            )*

            $(
                use [<bench_ $name>]::$name;
            )*

            ::gungraun::main!(library_benchmark_groups = $($name),*);
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
