# Changelog

## Unreleased
- **breaking:** `BumpPool::new` is now no longer const, you can the same const constructor with `BumpPool::new_in(Global)`.
- **breaking:** you can no longer be generic over `GUARANTEED_ALLOCATED` in some ways due to the `BaseAllocator` bound
- **added:** any allocator that implements `Default` can now be used as a base allocator (before it was just `Global`)
- **added:** `bump` method in `BumpVec` and `BumpString` to return the bump allocator

## 0.4.0 (2024-05-19)
- **breaking:** renamed `Stats::to_stats` to `to_guaranteed_stats`
- **breaking:** removed deprecated `BumpBox::into_fixed_vec` and `into_fixed_string`.
- **added:** `impl From<GuaranteedAllocatedStats> for Stats`
- **added:** `BumpBox::<[MaybeUninit<T>]>::init_fill_iter`
- **added:** `BumpBox::deallocate_in`

## 0.3.1 (2024-05-01)
- **fixed:** crash in debug mode when using `alloc_iter_mut(_rev)` or calling `into_(boxed_)slice` on a `MutBumpVec(Rev)` ([#16](https://github.com/bluurryy/bump-scope/issues/16))
- **added:** optimization to not align the bump pointer when the size happens to be a multiple of `MIN_ALIGN` ([#12](https://github.com/bluurryy/bump-scope/issues/12))

## 0.3.0 (2024-04-22)
- **breaking:** renamed `Stats` to `GuaranteedAllocatedStats`
- **breaking:** renamed `MaybeUnallocatedStats` to `Stats`
- **breaking:** `stats` now always returns `Stats` and is always available
- **breaking:** renamed `into_init` and `as_init(_mut)` to `into_guaranteed_allocated` and `as_guaranteed_allocated(_mut)`
- **added:** `guaranteed_allocated_stats` which returns `GuaranteedAllocatedStats`
- **added:** make `BumpPool::new` `const`

## 0.2.1 (2024-04-21)
- **fixed:** docs and changelog

## 0.2.0 (2024-04-21)
- **breaking:** adds the `INIT` const param to signify whether the bump has an allocated chunk
- **added:** `Bump::uninit` to create a `Bump` without allocation (and `const` with feature `nightly-const-refs-to-static`) ([#7](https://github.com/bluurryy/bump-scope/issues/7))
- **breaking:** `BumpVec::into_iter` returns `IntoIter<'a, T>` instead of `IntoIter<'b, T>` ([#8](https://github.com/bluurryy/bump-scope/issues/8))

## 0.1.8 (2024-04-11)
- **added:** `serde::Serialize` implementations for `BumpBox`, strings and vectors
- **added:** `serde::DeserializeSeed` implementations for strings and vectors

## 0.1.7 (2024-04-07)
- **added:** `BumpPool` along with `BumpPoolGuard`
- **added:** implement `Send` and `Sync` for `BumpBox`, `FixedBumpVec` and `FixedBumpString`

## 0.1.6 (2024-04-07)
- **fixed:** ZST allocation with respect to `drop`, `clone` and `default` calls
- **fixed:** `alloc_with` and `alloc_slice_fill_with` not calling `f` for ZSTs

## 0.1.5 (2024-04-05)
- **added:** `BumpVec::into_fixed_vec` and `FixedBumpVec::into_vec`
- **added:** fallible `FixedBumpVec` api
- **added:** `FixedBumpString`
- **added:** `from_init` and `from_uninit` for `FixedBumpVec` and `FixedBumpString`
- **deprecated:** `BumpBox::into_fixed_vec` and `BumpBox::into_fixed_string`

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
- **breaking:** `BumpVec` and `BumpString` now take an `&Bump(Scope)`, `MutBumpVec` and `MutBumpString` take a `&mut Bump(Scope)` ([#3](https://github.com/bluurryy/bump-scope/issues/3))

## 0.0.1 (2024-03-27)
- **fixed:** allocating a downwards `Bump` with layout of `[u8; 0]` no longer panics
- **change:** `alloc_iter` and `alloc_fmt` don't require the `alloc` feature anymore

## 0.0.0 (2024-03-26)
