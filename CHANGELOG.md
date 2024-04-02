# Changelog

## Unreleased
- **added:** `BumpVec::into_fixed_vec` and `FixedBumpVec::into_vec`

## 0.1.4 (2024-04-02)
- **added:** `String::shrink_to_fit`

## 0.1.3 (2024-04-02)
- **fix:** `aligned` and `scoped_aligned` not aligning

## 0.1.2 (2024-03-29)
- **added:** `BumpVec::shrink_to_fit`
- **fix:** unsoundness when allocating large slices

## 0.1.1 (2024-03-28)
- **fix:** `BumpVec` and `BumpString` growing

## 0.1.0 (2024-03-28)
- **breaking:** `BumpVec` and `BumpString` now take an `&Bump(Scope)`, `MutBumpVec` and `MutBumpString` take a `&mut Bump(Scope)`

## 0.0.1 (2024-03-27)
- **fixed:** allocating a downwards `Bump` with layout of `[u8; 0]` no longer panics
- **change:** `alloc_iter` and `alloc_fmt` don't require the `alloc` feature anymore

## 0.0.0 (2024-03-26)
