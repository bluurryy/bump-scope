# Benchmarks

This crate contains micro-benchmarks to compare `bump-scope`'s up- and downwards allocator and other bump allocator crates. Take these benchmarks with a grain of salt. A smaller number of instructions or branches does not necessarily mean better performance for your application. We benchmark instructions and branches instead of wall-clock time to get consistent, precise and faster results.

There are also criterion benchmarks at [../criterion-benches](../criterion-benches) and we keep track of the generated assembly at [bluurryy/bump-scope-inspect-asm](https://github.com/bluurryy/bump-scope-inspect-asm).

## Results

The benchmarks results in the tables below are shown in the format "instructions executed / branches executed".

These are the results of a benchmark run with 
<!-- spellchecker:off because the commit hash may get flagged -->
<!-- version start -->`rustc 1.92.0 (ded5c06cf 2025-12-08)` on `x86_64-unknown-linux-gnu` using `LLVM version 21.1.3`<!-- version end -->
<!-- spellchecker:on -->.

### Allocation

The following cases benchmark allocating a value or a slice of values (not necessarily using the `Allocator` api).

The `*_aligned` cases use a bump allocator with a sufficient minimum alignment for the allocated type, eliminating the need to align the bump pointer for the allocation.

The `*_overaligned` cases use a bump allocator with a minimum alignment greater than the alignment of the type being allocated, eliminating the need to align the bump pointer for the allocation but requiring to round up the allocation size to keep the bump pointer aligned.

<!-- alloc table start -->

| name                         | bump-scope (up) | bump-scope (down) | bumpalo | blink-alloc |
|------------------------------|-----------------|-------------------|---------|-------------|
| alloc_u8                     | 10 / 1          | 10 / 1            | 10 / 1  | 14 / 3      |
| alloc_u8_overaligned         | 12 / 1          | 11 / 1            | 12 / 1  | — [^1]      |
| alloc_u32                    | 14 / 1          | 11 / 1            | 14 / 2  | 16 / 3      |
| alloc_u32_aligned            | 12 / 1          | 10 / 1            | 12 / 1  | — [^1]      |
| alloc_u32_overaligned        | 13 / 1          | 11 / 1            | 12 / 1  | — [^1]      |
| alloc_big_struct             | 21 / 1          | 20 / 1            | 21 / 2  | 23 / 3      |
| alloc_big_struct_aligned     | 19 / 1          | 19 / 1            | 19 / 1  | — [^1]      |
| alloc_big_struct_overaligned | 20 / 1          | 20 / 1            | 19 / 1  | — [^1]      |
| alloc_u8_slice               | 32 / 3          | 32 / 3            | 31 / 3  | 34 / 5      |
| alloc_u8_slice_overaligned   | 34 / 3          | 33 / 3            | 35 / 4  | — [^1]      |
| alloc_u32_slice              | 45 / 6          | 44 / 6            | 45 / 7  | 52 / 8      |
| alloc_u32_slice_aligned      | 43 / 6          | 43 / 6            | 43 / 6  | — [^1]      |
| alloc_u32_slice_overaligned  | 45 / 6          | 44 / 6            | 45 / 6  | — [^1]      |

<!-- alloc table end -->

The benchmark cases above use the infallible api, panicking if allocating a new chunk from the base allocator fails.

<details>
<summary>Expand this section to see the benchmarks for the fallible api.</summary>

<!-- try alloc table start -->

| name                             | bump-scope (up) | bump-scope (down) | bumpalo | blink-alloc |
|----------------------------------|-----------------|-------------------|---------|-------------|
| try_alloc_u8                     | 10 / 1          | 10 / 1            | 10 / 1  | 14 / 3      |
| try_alloc_u8_overaligned         | 12 / 1          | 11 / 1            | 12 / 1  | — [^1]      |
| try_alloc_u32                    | 14 / 1          | 11 / 1            | 14 / 2  | 16 / 3      |
| try_alloc_u32_aligned            | 12 / 1          | 10 / 1            | 12 / 1  | — [^1]      |
| try_alloc_u32_overaligned        | 13 / 1          | 11 / 1            | 12 / 1  | — [^1]      |
| try_alloc_big_struct             | 21 / 1          | 20 / 1            | 21 / 2  | 23 / 3      |
| try_alloc_big_struct_aligned     | 19 / 1          | 19 / 1            | 19 / 1  | — [^1]      |
| try_alloc_big_struct_overaligned | 20 / 1          | 20 / 1            | 19 / 1  | — [^1]      |
| try_alloc_u8_slice               | 32 / 3          | 32 / 3            | 32 / 3  | 34 / 5      |
| try_alloc_u8_slice_overaligned   | 34 / 3          | 33 / 3            | 36 / 4  | — [^1]      |
| try_alloc_u32_slice              | 45 / 6          | 44 / 6            | 45 / 7  | 48 / 8      |
| try_alloc_u32_slice_aligned      | 43 / 6          | 43 / 6            | 43 / 6  | — [^1]      |
| try_alloc_u32_slice_overaligned  | 45 / 6          | 44 / 6            | 45 / 6  | — [^1]      |

<!-- try alloc table end -->

</details>

### Allocator API

The following cases benchmark the `Allocator` trait implementations. 

<!-- allocator_api table start -->

| name                      | bump-scope (up) | bump-scope (down) | bumpalo | blink-alloc |
|---------------------------|-----------------|-------------------|---------|-------------|
| allocate                  | 14 / 1          | 11 / 1            | 12 / 2  | 14 / 3      |
| grow_same_align           | 19 / 2          | 18 / 2            | 46 / 3  | 18 / 4      |
| grow_smaller_align        | 19 / 2          | 18 / 2            | 46 / 3  | 18 / 4      |
| grow_larger_align         | 19 / 2          | 18 / 2            | 16 / 2  | 18 / 3      |
| shrink_same_align [^2]    | 11 / 2          | 17 / 2            | 12 / 1  | 5 / 1       |
| shrink_smaller_align [^2] | 11 / 2          | 17 / 2            | 12 / 1  | 5 / 1       |
| shrink_larger_align [^2]  | 11 / 2          | 17 / 2            | 5 / 1   | 18 / 3      |
| deallocate                | 6 / 1           | 6 / 1             | 7 / 1   | 6 / 2       |
| deallocate_non_last       | 5 / 1           | 4 / 1             | 5 / 1   | 6 / 2       |

<!-- allocator_api table end -->

The allocator api benchmarks above use a statically known `Layout`. Consumers of allocators don't necessarily use their allocator in a way that makes the `Layout` or even just the alignment statically known to the `Allocator` methods. For instance `Vec::push` doesn't[^3].

If the layout is not statically known then the compiler can not do as many optimizations:

<!-- black_box_allocator_api table start -->

| name                                | bump-scope (up) | bump-scope (down) | bumpalo | blink-alloc |
|-------------------------------------|-----------------|-------------------|---------|-------------|
| black_box_allocate                  | 15 / 1          | 12 / 1            | 27 / 4  | 19 / 3      |
| black_box_grow_same_align           | 25 / 2          | 51 / 7            | 99 / 9  | 31 / 6      |
| black_box_grow_smaller_align        | 25 / 2          | 51 / 7            | 99 / 9  | 31 / 6      |
| black_box_grow_larger_align         | 25 / 2          | 51 / 7            | 73 / 9  | 54 / 8      |
| black_box_shrink_same_align [^2]    | 13 / 2          | 47 / 7            | 45 / 7  | 23 / 3      |
| black_box_shrink_smaller_align [^2] | 13 / 2          | 50 / 9            | 48 / 9  | 23 / 3      |
| black_box_shrink_larger_align [^2]  | 13 / 2          | 47 / 7            | 15 / 2  | 54 / 8      |
| black_box_deallocate                | 6 / 1           | 6 / 1             | 7 / 1   | 6 / 2       |
| black_box_deallocate_non_last       | 5 / 1           | 4 / 1             | 5 / 1   | 6 / 2       |

<!-- black_box_allocator_api table end -->

### Miscellaneous

- **`warm_up`** —  constructs a bump allocator and allocates a `u32`
- **`reset`** —  resets the bump allocator after allocating a `u32`

<!-- misc table start -->

| name    | bump-scope (up) | bump-scope (down) | bumpalo  | blink-alloc |
|---------|-----------------|-------------------|----------|-------------|
| warm_up | 224 / 31        | 230 / 32          | 355 / 44 | 283 / 38    |
| reset   | 26 / 2          | 25 / 2            | 23 / 2   | 26 / 3      |

<!-- misc table end -->

[^1]: `blink-alloc` does not support setting a minimum alignment
[^2]: the shrink implementations differ a lot, `bump-scope` always tries to shrink the allocation, `bumpalo` only shrinks if it can do so with a `copy_nonoverlapping` and `blink-alloc` does not shrink allocations unless required due to alignment
[^3]: tested here: <https://github.com/bluurryy/is-layout-statically-known-to-allocator>

## Reproducing

Install [Valgrind] and [gungraun-runner].

Then run the benchmark with
```sh
cargo bench --bench bench -- --save-summary=json
```
and update the tables above with
```sh
cargo run
```

[Valgrind]: https://gungraun.github.io/gungraun/latest/html/installation/prerequisites.html
[gungraun-runner]: https://gungraun.github.io/gungraun/latest/html/installation/gungraun.html#installation-of-the-benchmark-runner