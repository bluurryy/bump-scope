# Benchmarks

This crate contains micro-benchmarks to compare `bump-scope`'s up- and downwards allocator and other bump allocator crates. Take these benchmarks with a grain of salt. A smaller number of instructions or branches does not necessarily mean better performance for your application. We benchmark instructions and branches instead of wall-clock time to get consistent, precise and faster results.

There are also criterion benchmarks at [../criterion-benches](../criterion-benches) and we keep track of the generated assembly at [bluurryy/bump-scope-inspect-asm](https://github.com/bluurryy/bump-scope-inspect-asm).

## Results

The benchmarks results in the tables below are shown in the format "instructions executed / branches executed / branch predictor misses".

These are the results of a benchmark run with <!-- version start -->`rustc 1.87.0 (17067e9ac 2025-05-09)` on `x86_64-unknown-linux-gnu` using `LLVM version 20.1.1`<!-- version end -->.

### Allocation

The following cases benchmark allocating a value or a slice of values (not necessarily using the `Allocator` api).

The `*_aligned` cases use a bump allocator with a sufficient minimum alignment for the allocated type (if supported).

<!-- alloc table start -->

| name                            | bump-scope (up) | bump-scope (down) | bumpalo    | blink-alloc |
|---------------------------------|-----------------|-------------------|------------|-------------|
| alloc_u8                        | 10 / 1 / 0      | 10 / 1 / 0        | 11 / 2 / 1 | 16 / 4 / 0  |
| alloc_u32                       | 14 / 1 / 0      | 11 / 1 / 0        | 15 / 3 / 0 | 18 / 4 / 0  |
| (try_) alloc_u32_aligned        | 12 / 1 / 0      | 10 / 1 / 0        | 13 / 2 / 0 | — [^1]      |
| try_alloc_u32                   | 14 / 1 / 0      | 11 / 1 / 1        | 15 / 3 / 1 | 18 / 4 / 0  |
| alloc_big_struct                | 21 / 1 / 0      | 20 / 1 / 0        | 22 / 3 / 0 | 25 / 4 / 1  |
| (try_) alloc_big_struct_aligned | 19 / 1 / 0      | 19 / 1 / 0        | 20 / 2 / 0 | — [^1]      |
| try_alloc_big_struct            | 21 / 1 / 0      | 20 / 1 / 0        | 22 / 3 / 0 | 25 / 4 / 0  |
| alloc_slice_copy                | 45 / 6 / 2      | 44 / 6 / 1        | 46 / 8 / 2 | 57 / 9 / 2  |
| alloc_slice_copy_aligned        | 43 / 6 / 1      | 43 / 6 / 1        | 44 / 7 / 2 | — [^1]      |
| try_alloc_slice_copy            | 47 / 7 / 2      | 45 / 7 / 2        | 46 / 8 / 2 | 53 / 9 / 2  |
| try_alloc_slice_copy_aligned    | 43 / 6 / 1      | 44 / 7 / 2        | 44 / 7 / 2 | — [^1]      |

<!-- alloc table end -->

### Allocator API

The following cases benchmark the `Allocator` trait implementations. 

<!-- allocator_api table start -->

| name                      | bump-scope (up) | bump-scope (down) | bumpalo    | blink-alloc |
|---------------------------|-----------------|-------------------|------------|-------------|
| allocate                  | 15 / 2 / 0      | 13 / 2 / 0        | 13 / 3 / 0 | 16 / 4 / 1  |
| grow_same_align           | 19 / 2 / 0      | 19 / 2 / 1        | 53 / 4 / 3 | 18 / 4 / 1  |
| grow_smaller_align        | 19 / 2 / 0      | 19 / 2 / 1        | 53 / 4 / 3 | 18 / 4 / 1  |
| grow_larger_align         | 19 / 2 / 0      | 19 / 2 / 1        | 17 / 3 / 0 | 20 / 4 / 1  |
| shrink_same_align [^2]    | 11 / 2 / 1      | 17 / 2 / 1        | 12 / 1 / 1 | 5 / 1 / 0   |
| shrink_smaller_align [^2] | 11 / 2 / 1      | 17 / 2 / 1        | 12 / 1 / 1 | 5 / 1 / 0   |
| shrink_larger_align [^2]  | 11 / 2 / 1      | 17 / 2 / 1        | 5 / 1 / 1  | 20 / 4 / 0  |
| deallocate                | 6 / 1 / 1       | 6 / 1 / 1         | 7 / 1 / 1  | 6 / 2 / 1   |
| deallocate_non_last       | 5 / 1 / 0       | 4 / 1 / 0         | 5 / 1 / 0  | 6 / 2 / 0   |

<!-- allocator_api table end -->

The allocator api benchmarks above use a statically known `Layout`. If the layout is not statically known for instance if the
allocator api function call is not inlined then the compiler can do less optimizations:

<!-- black_box_allocator_api table start -->

| name                                | bump-scope (up) | bump-scope (down) | bumpalo     | blink-alloc |
|-------------------------------------|-----------------|-------------------|-------------|-------------|
| black_box_allocate                  | 16 / 2 / 0      | 14 / 2 / 0        | 26 / 5 / 1  | 23 / 4 / 0  |
| black_box_grow_same_align           | 25 / 2 / 0      | 53 / 7 / 4        | 99 / 11 / 4 | 31 / 6 / 1  |
| black_box_grow_smaller_align        | 25 / 2 / 0      | 53 / 7 / 4        | 99 / 11 / 4 | 31 / 6 / 1  |
| black_box_grow_larger_align         | 25 / 2 / 1      | 53 / 7 / 4        | 63 / 10 / 4 | 57 / 9 / 4  |
| black_box_shrink_same_align [^2]    | 13 / 2 / 1      | 47 / 7 / 4        | 45 / 7 / 2  | 23 / 3 / 0  |
| black_box_shrink_smaller_align [^2] | 13 / 2 / 1      | 50 / 9 / 4        | 48 / 9 / 1  | 23 / 3 / 1  |
| black_box_shrink_larger_align [^2]  | 13 / 2 / 1      | 47 / 7 / 4        | 15 / 2 / 1  | 57 / 9 / 4  |
| black_box_deallocate                | 6 / 1 / 1       | 6 / 1 / 1         | 7 / 1 / 1   | 6 / 2 / 0   |
| black_box_deallocate_non_last       | 5 / 1 / 0       | 4 / 1 / 0         | 5 / 1 / 0   | 6 / 2 / 1   |

<!-- black_box_allocator_api table end -->

### Miscellaneous

- **`warm_up`** —  constructs a bump allocator and allocates a `u32`
- **`reset`** —  resets the bump allocator after allocating a `u32`

<!-- misc table start -->

| name    | bump-scope (up) | bump-scope (down) | bumpalo       | blink-alloc   |
|---------|-----------------|-------------------|---------------|---------------|
| warm_up | 227 / 31 / 10   | 233 / 32 / 11     | 358 / 43 / 15 | 284 / 38 / 14 |
| reset   | 26 / 2 / 2      | 25 / 2 / 2        | 23 / 2 / 1    | 26 / 3 / 2    |

<!-- misc table end -->

[^1]: `blink-alloc` does not support setting a minimum alignment
[^2]: the shrink implementations differ a lot, `bump-scope` always tries to shrink the allocation, `bumpalo` only shrinks if it can do so with a `copy_nonoverlapping` and `blink-alloc` does not shrink allocations unless required due to alignment

## Reproducing

Install [Valgrind](https://iai-callgrind.github.io/iai-callgrind/latest/html/installation/prerequisites.html) and [iai-callgrind-runner](https://iai-callgrind.github.io/iai-callgrind/latest/html/installation/iai_callgrind.html).

Then run the benchmark with
```bash
cargo bench --bench bench -- --save-summary=json
```
and update the tables above with
```bash
cargo run
```