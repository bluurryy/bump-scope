# Changelog

## 0.1.0 (2024-03-28)
- **breaking:** `BumpVec` and `BumpString` now take an `&Bump(Scope)`, `MutBumpVec` and `MutBumpString` take a `&mut Bump(Scope)`

## 0.0.1 (2024-03-27)
- **fixed:** allocating a downwards `Bump` with layout of `[u8; 0]` no longer panics
- **change:** `alloc_iter` and `alloc_fmt` don't require the `alloc` feature anymore

## 0.0.0 (2024-03-26)
