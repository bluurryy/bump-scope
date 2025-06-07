# Benchmarks

instructions / branches

| name                  | bump-scope | bumpalo  | blink-alloc |
| --------------------- | ---------- | -------- | ----------- |
| alloc_u8              | 10 / 1     | 11 / 2   | 16 / 4      |
| alloc_u32             | 14 / 1     | 15 / 3   | 18 / 4      |
| alloc_u32_aligned     | 12 / 1     | 13 / 2   | 18 / 4 [^1] |
| try_alloc_u32         | 14 / 1     | 15 / 3   | 18 / 4      |
| try_alloc_u32_aligned | 12 / 1     | 13 / 2   | 0 / 0 [^1]  |
| allocate_u32          | 15 / 2     | 13 / 3   | 16 / 4      |
| allocate              | 16 / 2     | 26 / 5   | 23 / 4      |
| grow_same_align       | 19 / 2     | 53 / 4   | 18 / 4      |
| grow_smaller_align    | 19 / 2     | 53 / 4   | 18 / 4      |
| grow_larger_align     | 19 / 2     | 17 / 3   | 20 / 4      |
| shrink_same_align     | 11 / 2     | 12 / 1   | 5 / 1       |
| shrink_smaller_align  | 11 / 2     | 12 / 1   | 5 / 1       |
| shrink_larger_align   | 0 / 0      | 5 / 1    | 20 / 4      |
| warm_up               | 227 / 31   | 358 / 43 | 283 / 38    |
| reset                 | 26 / 2     | 23 / 2   | 26 / 3      |

[^1]: `blink_alloc` does not support setting a minimum alignment

# Reproducing

[Valgrind](https://valgrind.org/) must be installed and its header files accessible. If you have installed `Valgrind` using OS-specific package manager, the paths to the headers are likely to be resolved automatically by [`cc`](https://docs.rs/cc/latest/cc/index.html).

In case of manual installation, you can set the path to the `Valgrind` headers location through the `DEP_VALGRIND` environment variable. For example:

```bash
DEP_VALGRIND=/home/linuxbrew/.linuxbrew/include cargo build --benches --release
```

Now just write
```bash
cargo bench
```
and the markdown formatted table seen above will be printed to stdout.