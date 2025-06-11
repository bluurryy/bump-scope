# Benchmarks

This crate contains micro-benchmarks to compare `bump-scope`'s up- and downwards allocator with other bump allocator crates and keep track of regressions. Take these benchmarks with a big grain of salt. A smaller number does not necessarily mean better performance. I've opted to benchmark instructions and branches instead of wall-clock time because I could get neither precision nor consistency with regular time based benchmarks.

## Results

The benchmarks results in the table below are shown in the format "instruction count / branch count".

The following cases are tested:
- **`alloc_u8`** —  allocate a `u8`
- **`alloc_u32`** —  allocate a `u32`
- **`alloc_u32_aligned`** —  allocate a `u32` with a minimum alignment of `4`
- **`try_alloc_u32`** —  fallibly allocate a `u32`
- **`try_alloc_u32_aligned`** —  fallibly allocate a `u32` with a minimum alignment of `4`
- **`allocate_u32`** —  use the `Allocator` trait to allocate space for a `u32`
- **`allocate`** —  allocate space for the `Layout` of an `u32` (the layout is not statically known, which inhibits optimizations `allocate_u32` was able to do)
- **`grow_same_align`** —  grow an allocation to the same alignment
- **`grow_smaller_align`** —  grow an allocation to a smaller alignment
- **`grow_larger_align`** —  grow an allocation to a larger alignment
- **`shrink_same_align`** —  shrink an allocation to the same alignment
- **`shrink_smaller_align`** —  shrink an allocation to a smaller alignment
- **`shrink_larger_align`** —  shrink an allocation to a larger alignment
- **`deallocate`** —  deallocate a `u32` which is the most recent allocation
- **`deallocate_fail`** —  attempt to deallocate a `u32` which is NOT the most recent allocation
- **`warm_up`** —  construct a bump allocator and allocate a `u32`
- **`reset`** —  reset the bump allocator after allocating a `u32`

<!-- table start -->

| name                      | bump-scope (up) | bump-scope (down) | bumpalo  | blink-alloc |
|---------------------------|-----------------|-------------------|----------|-------------|
| alloc_u8                  | 10 / 1          | 10 / 1            | 11 / 2   | 16 / 4      |
| alloc_u32                 | 14 / 1          | 11 / 1            | 15 / 3   | 18 / 4      |
| alloc_u32_aligned         | 12 / 1          | 10 / 1            | 13 / 2   | 18 / 4 [^1] |
| try_alloc_u32             | 14 / 1          | 11 / 1            | 15 / 3   | 18 / 4      |
| try_alloc_u32_aligned     | 12 / 1          | 10 / 1            | 13 / 2   | 18 / 4 [^1] |
| allocate_u32              | 15 / 2          | 13 / 2            | 13 / 3   | 16 / 4      |
| allocate                  | 16 / 2          | 14 / 2            | 26 / 5   | 23 / 4      |
| grow_same_align           | 19 / 2          | 19 / 2            | 53 / 4   | 18 / 4      |
| grow_smaller_align        | 19 / 2          | 19 / 2            | 53 / 4   | 18 / 4      |
| grow_larger_align         | 19 / 2          | 19 / 2            | 17 / 3   | 20 / 4      |
| shrink_same_align [^2]    | 11 / 2          | 17 / 2            | 12 / 1   | 5 / 1       |
| shrink_smaller_align [^2] | 11 / 2          | 17 / 2            | 12 / 1   | 5 / 1       |
| shrink_larger_align [^2]  | 11 / 2          | 17 / 2            | 5 / 1    | 20 / 4      |
| deallocate                | 5 / 1           | 6 / 1             | 7 / 1    | 6 / 2       |
| deallocate_fail           | 5 / 1           | 4 / 1             | 4 / 1    | 6 / 2       |
| warm_up                   | 227 / 31        | 233 / 32          | 358 / 43 | 284 / 38    |
| reset                     | 26 / 2          | 25 / 2            | 23 / 2   | 26 / 3      |


<!-- table end -->

[^1]: `blink-alloc` does not support setting a minimum alignment
[^2]: the shrink implementations differ a lot, `bump-scope` always tries to shrink the allocation, `bumpalo` only shrinks if it can do so with a `copy_nonoverlapping` and `blink-alloc` does not shrink allocations unless required due to alignment

## Reproducing

Install [Valgrind](https://iai-callgrind.github.io/iai-callgrind/latest/html/installation/prerequisites.html) and [iai-callgrind-runner](https://iai-callgrind.github.io/iai-callgrind/latest/html/installation/iai_callgrind.html).

Then run the benchmark with
```bash
cargo bench --bench bench -- --save-summary=json
```
and update the table above with
```bash
cargo run
```