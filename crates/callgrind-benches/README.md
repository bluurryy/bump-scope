# Benchmarks

This crate contains micro-benchmarks to compare `bump-scope`'s up- and downwards allocator with other bump allocator crates and to keep track of regressions. Take these benchmarks with a grain of salt. A smaller number does not necessarily mean better performance. We benchmark instructions and branches instead of wall-clock time to get more consistent results.

There are also criterion benchmarks at [../../benches](../../benches) and we keep track of the generated assembly at [bluurryy/bump-scope-inspect-asm](https://github.com/bluurryy/bump-scope-inspect-asm).

## Results

The benchmarks results in the tables below are shown in the format "instruction count / branch count".

### Allocation

The following cases benchmark allocating a value or a slice of values (not necessarily using the `Allocator` api).

The `*_aligned` cases use a bump allocator with a sufficient minimum alignment for the allocated type (if supported).

<!-- alloc table start -->

| name                            | bump-scope (up) | bump-scope (down) | bumpalo | blink-alloc |
|---------------------------------|-----------------|-------------------|---------|-------------|
| alloc_u8                        | 10 / 1          | 10 / 1            | 11 / 2  | 16 / 4      |
| (try_) alloc_u32                | 14 / 1          | 11 / 1            | 15 / 3  | 18 / 4      |
| (try_) alloc_u32_aligned        | 12 / 1          | 10 / 1            | 13 / 2  | — [^1]      |
| (try_) alloc_big_struct         | 21 / 1          | 20 / 1            | 22 / 3  | 25 / 4      |
| (try_) alloc_big_struct_aligned | 19 / 1          | 19 / 1            | 20 / 2  | — [^1]      |
| alloc_slice_copy                | 45 / 6          | 44 / 6            | 46 / 8  | 57 / 9      |
| alloc_slice_copy_aligned        | 43 / 6          | 43 / 6            | 44 / 7  | — [^1]      |
| try_alloc_slice_copy            | 47 / 7          | 45 / 7            | 46 / 8  | 53 / 9      |
| try_alloc_slice_copy_aligned    | 43 / 6          | 44 / 7            | 44 / 7  | — [^1]      |

<!-- alloc table end -->

### Allocator API

The following cases benchmark the `Allocator` trait implementations. 

<!-- allocator_api table start -->

| name                      | bump-scope (up) | bump-scope (down) | bumpalo | blink-alloc |
|---------------------------|-----------------|-------------------|---------|-------------|
| allocate                  | 15 / 2          | 13 / 2            | 13 / 3  | 16 / 4      |
| grow_same_align           | 19 / 2          | 19 / 2            | 53 / 4  | 18 / 4      |
| grow_smaller_align        | 19 / 2          | 19 / 2            | 53 / 4  | 18 / 4      |
| grow_larger_align         | 19 / 2          | 19 / 2            | 17 / 3  | 20 / 4      |
| shrink_same_align [^2]    | 11 / 2          | 17 / 2            | 12 / 1  | 5 / 1       |
| shrink_smaller_align [^2] | 11 / 2          | 17 / 2            | 12 / 1  | 5 / 1       |
| shrink_larger_align [^2]  | 11 / 2          | 17 / 2            | 5 / 1   | 20 / 4      |
| deallocate                | 6 / 1           | 6 / 1             | 7 / 1   | 6 / 2       |
| deallocate_non_last       | 5 / 1           | 4 / 1             | 5 / 1   | 6 / 2       |

<!-- allocator_api table end -->

The allocator api benchmarks above use a statically known `Layout`. If the layout is not statically known for instance if the
allocator api function call is not inlined then the compiler can do less optimizations:

<!-- black_box_allocator_api table start -->

| name                                | bump-scope (up) | bump-scope (down) | bumpalo | blink-alloc |
|-------------------------------------|-----------------|-------------------|---------|-------------|
| black_box_allocate                  | 16 / 2          | 14 / 2            | 26 / 5  | 23 / 4      |
| black_box_grow_same_align           | 25 / 2          | 53 / 7            | 99 / 11 | 31 / 6      |
| black_box_grow_smaller_align        | 25 / 2          | 53 / 7            | 99 / 11 | 31 / 6      |
| black_box_grow_larger_align         | 25 / 2          | 53 / 7            | 63 / 10 | 57 / 9      |
| black_box_shrink_same_align [^2]    | 13 / 2          | 47 / 7            | 45 / 7  | 23 / 3      |
| black_box_shrink_smaller_align [^2] | 13 / 2          | 50 / 9            | 48 / 9  | 23 / 3      |
| black_box_shrink_larger_align [^2]  | 13 / 2          | 47 / 7            | 15 / 2  | 57 / 9      |
| black_box_deallocate                | 6 / 1           | 6 / 1             | 7 / 1   | 6 / 2       |
| black_box_deallocate_non_last       | 5 / 1           | 4 / 1             | 5 / 1   | 6 / 2       |

<!-- black_box_allocator_api table end -->

### Miscellaneous

- **`warm_up`** —  constructs a bump allocator and allocates a `u32`
- **`reset`** —  resets the bump allocator after allocating a `u32`

<!-- misc table start -->

| name    | bump-scope (up) | bump-scope (down) | bumpalo  | blink-alloc |
|---------|-----------------|-------------------|----------|-------------|
| warm_up | 227 / 31        | 233 / 32          | 358 / 43 | 284 / 38    |
| reset   | 26 / 2          | 25 / 2            | 23 / 2   | 26 / 3      |

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