# Benchmarks

instructions / branches

<!-- table start -->

| name                  | bump-scope (up) | bump-scope (down) | bumpalo  | blink-alloc |
|-----------------------|-----------------|-------------------|----------|-------------|
| alloc_u8              | 10 / 1          | 10 / 1            | 11 / 2   | 16 / 4      |
| alloc_u32             | 14 / 1          | 11 / 1            | 15 / 3   | 18 / 4      |
| alloc_u32_aligned     | 12 / 1          | 10 / 1            | 13 / 2   | 0 / 0       |
| try_alloc_u32         | 14 / 1          | 11 / 1            | 15 / 3   | 18 / 4      |
| try_alloc_u32_aligned | 12 / 1          | 10 / 1            | 13 / 2   | 0 / 0       |
| allocate_u32          | 14 / 2          | 13 / 2            | 13 / 3   | 16 / 4      |
| allocate              | 16 / 2          | 14 / 2            | 26 / 5   | 23 / 4      |
| grow_same_align       | 19 / 2          | 18 / 2            | 51 / 4   | 18 / 4      |
| grow_smaller_align    | 18 / 2          | 19 / 2            | 53 / 4   | 18 / 4      |
| grow_larger_align     | 19 / 2          | 19 / 2            | 16 / 3   | 20 / 4      |
| shrink_same_align     | 11 / 2          | 17 / 2            | 12 / 1   | 5 / 1       |
| shrink_smaller_align  | 11 / 2          | 17 / 2            | 12 / 1   | 5 / 1       |
| shrink_larger_align   | 11 / 2          | 17 / 2            | 5 / 1    | 20 / 4      |
| warm_up               | 227 / 31        | 233 / 32          | 358 / 43 | 284 / 38    |
| reset                 | 26 / 2          | 25 / 2            | 23 / 2   | 26 / 3      |

<!-- table end -->

# Reproducing

Install [Valgrind](https://iai-callgrind.github.io/iai-callgrind/latest/html/installation/prerequisites.html) and [iai-callgrind-runner](https://iai-callgrind.github.io/iai-callgrind/latest/html/installation/iai_callgrind.html).

Then run the benchmark with
```bash
cargo bench --bench bench -- --save-summary=json
```
and update the table above with
```bash
cargo run
```