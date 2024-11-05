# Changelog

## Unreleased
- **added:** `BumpPool`'s `(try_)get_with_size` and `(try_)get_with_capacity`.
- **added:** `BumpPoolGuard`'s `pool` field is now `pub`.

## 0.10.8 (2024-10-30)
- **added:** `FromUtf8Error` now implements `Clone`, `PartialEq`, `Eq`, `Display` and `Error`.

## 0.10.7 (2024-10-30)
- **added:** minimum capacity when growing vectors and strings (just like `Vec`)
- **added:** `(try_)from_utf8_lossy_in` for strings
- **added:** `(try_)from_utf16(_lossy)_in` for strings

## 0.10.6 (2024-10-26)
- **fixed:** potential UB when using `BumpVec::splice`
- **added:** default generic allocator parameter for `bump_vec::Splice`

## 0.10.5 (2024-10-19)
- **added:** `splice` to `BumpVec`
- **added:** `reserve_exact` to vectors and strings
- **added:** `spare_capacity_mut` and `from_iter(_exact)_in` to vectors

## 0.10.4 (2024-10-18)
- **improved:** increased performance of `(Mut)BumpString::from_str_in`

## 0.10.3 (2024-10-15)
- **added:** made `owned_str` module `pub`

## 0.10.2 (2024-10-15)
- **added:** missing string methods to `BumpBox<str>`: `len`, `is_empty`, `set_len`, `retain`, `clear`, `as(_mut)_ptr`, `remove`, `as_mut_bytes`, `from_utf8(_unchecked)`
- **deprecated:** `BumpBox`'s `into_boxed_str(_unchecked)` in favor of `from_utf8(_unchecked)`
- **added:** `Default` for `FixedBumpString`
- **added:** string methods `pop`, `truncate`, `retain`, `drain`
- **added:** made more `len` and `is_empty` methods `const`
- **added:** `split_off` to suitable vector and string types
- **added:** impl `Add<&str>` for `BumpString`
- **added:** `reserve` methods to `FixedBumpString`
- **added:** impl `Extend<char>` and `Extend<&char>` for suitable string types
- **added:** impl `Clone` for `BumpVec` and `BumpString`

## 0.10.1 (2024-10-14)
- **added:** `MutBumpVecRev::{ append, into_flattened, unchecked_push(_with), as_non_null_{ptr, slice} }`
- **fixed:** `MutBumpVecRev::extend_from_within_clone` doing nothing for ZSTs
- **fixed:** potential UB in `MutBumpVecRev::extend_from_slice_clone` when clone panics

## 0.10.0 (2024-10-07)
- **breaking:** upgraded `zerocopy` dependency to version `0.8.2`
- **breaking:** removed deprecated methods `FixedBumpVec::{ layout, as_(mut_)boxed_slice }`, 

## 0.9.2 (2024-09-28)
- **improved:** removed a duplicate call to `shrink_to_fit` in `alloc_iter` and `alloc_fmt`
- **deprecated:** `FixedBumpVec`'s `layout` and `as(_mut)_boxed_slice` 

## 0.9.1 (2024-09-23)
- **improved:** removed a branch when doing fallible sized allocations like `try_alloc_*` ([#34](https://github.com/bluurryy/bump-scope/pull/34))

## 0.9.0 (2024-09-18)
- **added:** `from_parts` and `into_parts` to `BumpVec` and `BumpString`
- **added:** `FixedBumpString::into_string` returning a `BumpString`
- **breaking:** `BumpVec` and `BumpString` now deallocate on drop, and shrink when calling `into_(boxed_)slice`
- **breaking:** `BumpVec::into_iter` now returns `bump_vec::IntoIter` which deallocates on drop
- **breaking:** `IntoIter`, `Drain` and `ExtractIf` have been moved to the `owned_slice` module

## 0.8.2 (2024-09-15)
- **added:** `NoDrop` blanket impl for `[T]` is more general, replacing `Copy` bound with `NoDrop`

## 0.8.1 (2024-09-15)
- **added:** implementation of `NoDrop` for `BumpBox`, `FixedBumpVec`, `FixedBumpString` and `BumpScope`

## 0.8.0 (2024-09-03)
- **fixed:** panic message on formatting failure to mention that, instead of a wrong "capacity overflow"
- **fixed:** `(try_)alloc_try_with` UB when allocating inside the provided closure, memory leak when using an *unallocated* bump allocator
- **breaking:** `(try_)alloc_try_with` now requires a guaranteed allocated `Bump(Scope)`
- **added:** `(try_)alloc_try_with_mut` as an optimized version of `(try_)alloc_try_with`
- **improved:** `reset_to` now takes a `&` instead of `&mut`
- **fixed:** potential UB when using `alloc_iter_mut*` or `MutBump*` collections

## 0.7.0 (2024-08-31)
- **breaking:** allow returning values from closure arguments of `scoped`, `aligned` and `scoped_aligned`

## 0.6.0 (2024-08-25)
- **breaking:** `(Fixed)BumpString::as_mut_vec` now return `&mut` as they should instead of `&`.
- **breaking:** removed `GUARANTEED_ALLOCATED` parameter from `BumpPool(Guard)`
- **breaking:** removed deprecated `(try_)alloc_slice_zeroed` in favor of `(try_)alloc_zeroed_slice`

## 0.5.9 (2024-08-25)
- **improved:** removed a branch when bumping downwards (#25)
- **fixed:** UB when using a base allocator with an alignment greater than `2 * size_of::<usize>()` ([#32](https://github.com/bluurryy/bump-scope/pull/32))

## 0.5.8 (2024-08-22)
- **fixed:** `Mut*` collections' `into(_boxed)_slice` as well as `alloc_iter_mut(_rev)` and `alloc_fmt_mut` to not take up more than the necessary space

## 0.5.7 (2024-08-18)
- **added**: `from_utf8_unchecked` for all string types
- **added**: `BumpString::into_fixed_string`
- **added**: `BumpBox<[T]>::partition`

## 0.5.6 (2024-08-17)
- **fixed:** `alloc_iter` and `alloc_fmt` to not take up more than the necessary space

## 0.5.5 (2024-07-26)
- **fixed:** Rust Analyzer breaking for other structs with a default `Global` allocator parameter

## 0.5.4 (2024-07-26)
- **fixed**: Rust Analyzer breaking for `Bump`

## 0.5.3 (2024-06-28)
- **added**: `extend_zeroed` for vectors and strings
- **added**: `resize_zeroed` for vectors

## 0.5.2 (2024-06-09)
- **deprecated**: `alloc_slice_zeroed` in favor of `alloc_zeroed_slice`

## 0.5.1 (2024-06-08)
- **added:** `zerocopy` feature that adds `alloc_zeroed`, `alloc_slice_zeroed` and `BumpBox::init_zeroed`

## 0.5.0 (2024-05-21)
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
