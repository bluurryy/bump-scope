# Changelog

## 0.16.3 (2025-02-21)
- **added:** `capacity` field to the `Debug` representation of `Stats` and `Bump(Scope(Guard(Root)))`

## 0.16.2 (2025-02-21)
- **changed:** simpler and shorter `Debug` output for `Bump(Scope(Guard(Root)))` and `Chunk(NextIter, PrevIter)`
- **docs:** removed `rust-patterns` from `package.categories`, added `no-std::no-alloc`
- **docs:** documentation improvements

## 0.16.1 (2025-01-22)
- **fixed:** double-dropping elements when calling `into_flattened`
- **added:** `(try_)replace_range` and `as_(mut_)ptr` to strings
- **docs:** small improvements

## 0.16.0 (2025-01-10)
- **breaking:** replaced `OwnedSlice`'s `owned_slice_ptr` method with `owned_slice_ref` which returns a reference instead of a pointer
- **fixed:** `Write` impl for `FixedBumpVec<u8>` not actually writing
- **added:** `unsize_bump_box` to unsize `BumpBox`es on stable
- **docs:** improved docs for `split_off`

## 0.15.2 (2025-01-06)
- **fixed:** `append` implementation of `MutBumpVecRev` now accounts for `take_owned_slice` panicking
- **added:** implemented `Default` for `Stats`, `ChunkPrevIter`, `ChunkNextIter`, `WithoutShrink` and `WithoutDealloc`
- **docs:** small improvements; more examples

## 0.15.1 (2025-01-04)
- **added:** implemented `TakeOwnedSlice` for `vec::IntoIter` and `vec::Drain`
- **performance:** improved speed and time complexity of `split_off`
- **docs:** improvements to crate docs, owned slice and `split_off`

## 0.15.0 (2025-01-02)
- **breaking:** renamed `unchecked_push` to `push_unchecked`
- **breaking:** renamed `unchecked_push_with` to `push_with_unchecked`
- **breaking:** redesigned `OwnedSlice` trait (used as `append` parameter); `append` now accepts more types; `append` now accepts `alloc::vec::Vec` instead of `allocator_api2::vec::Vec`
- **breaking:** redesigned `split_off`; it no longer allocates, but splits in place; it now takes a range parameter instead of a position; now available for `Fixed*` and `BumpBox<str>`
- **breaking:** added safety condition of splittable memory blocks to `BumpAllocator`
- **deprecated:** `extend_from_array` in favor of `append`
- **added:** `BumpBox<[T; N]>::into_unsized` returning `BumpBox<[T]>`

## 0.14.0 (2024-12-12)
- **breaking:** fix `scoped_aligned`'s closure to take a `BumpScope` with `NEW_MIN_ALIGN` instead of `MIN_ALIGN`
- **breaking:** `aligned`'s closure now takes a `BumpScope` with lifetime `'a` instead of `'_`
- **docs:** various improvements

## 0.13.1 (2024-11-30)
- **docs:** improved documentation of `Bump` and `BumpScope` and in some other places

## 0.13.0 (2024-11-29)
- **breaking:** removed `"nightly-const-refs-to-static"` feature; `Bump::unallocated` is now automatically const for any rust version since 1.83
- **breaking:** removed `Bump(Scope)`'s `without_dealloc` and `without_shrink`; use `WithoutDealloc(&bump)` and `WithoutShrink(&bump)` instead
- **breaking:** renamed `stats` method on strings and vectors to `allocator_stats`
- **breaking:** feature gate panicking functions of `(Mut)BumpAllocator` with `"panic-on-alloc"` (those functions are doc hidden for now)
- **fixed:** potential UB in `write_vectored` in pathologic case of `usize` overflow when `bufs` contains a large amount of duplicate `IoSlice`s
- **added:** implemented `core::error::Error` for error types when rust version allows

## 0.12.3 (2024-11-23)
- **fixed:** interior nuls being ignored for `alloc_cstr_fmt_mut` and `MutBumpString::into_cstr` (unsound)

## 0.12.2 (2024-11-22)
- **fixed:** `CStr` now stops at the first nul; before, interior nuls were ignored (unsound)

## 0.12.1 (2024-11-21)
- **added:** implemented `NoDrop` for `CStr`, `OsStr` and `Path`
- **added:** `alloc_cstr`, `alloc_cstr_from_str`, `alloc_cstr_fmt` and `alloc_cstr_fmt_mut`
- **added:** `(Mut)BumpString`'s `(try_)into_cstr`

## 0.12.0 (2024-11-17)
- **breaking:** redesigned `OwnedSlice` trait

## 0.11.1 (2024-11-17)
- **fixed:** double dropping the elements of `MutBumpVec` when growing

## 0.11.0 (2024-11-16)
- **breaking:** vectors and strings now take a single `A` generic parameter instead of the `'b, 'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool` of before
- **breaking:** `vec`-like macros now take the bump allocator as is instead of by `$bump.as_scope()`;
  you will need to change `bump_vec![in bump` to `bump_vec![in &bump` and `bump_vec![in bump` to `bump_vec![in &mut bump` unless those `bump` are already references.
- **breaking:** `bump` methods on vectors and strings has been renamed to `allocator`, the old `allocator` method which returned the base allocator is gone
- **breaking:** `WithLifetime` has been removed
- **breaking:** `Stats<'a, UP>` is now `Stats<'a, GUARANTEED_ALLOCATED>`
- **breaking:** `GuaranteedAllocatedStats` has been removed in favor of `Stats<'a, true>`
- **breaking:** removed deprecated functions `BumpBox<[u8]>::into_boxed_str(_unchecked)`; use `BumpBox<str>::from_utf8(_unchecked)` instead
- **breaking:** renamed `into_guaranteed_allocated` to `guaranteed_allocated`; `as_guaranteed_allocated` to `guaranteed_allocated_ref`; `as_guaranteed_allocated_mut` to `guaranteed_allocated_mut`
- **breaking:** added `"panic-on-alloc"` feature which enables the panicking alloc functions (on by default); if you had `default-features = false` before you might need to enable this feature now
- **breaking:** renamed `MutBumpVecRev`'s `as_nonnull_ptr` to `as_non_null_ptr` and `as_nonnull_slice` to `as_non_null_slice`
- **breaking:** `append` methods now take `impl OwnedSlice<T>` instead
- **breaking:** `Chunk`, `ChunkNextIter` and `ChunkPrevIter` have moved to the `stats` module
- **fixed:** `BumpVec::split_off` now retains the capacity of `self` like the docs say
- **added:** more general `PartialEq` for vectors and strings
- **added:** `not_guaranteed_allocated(_ref)` methods on `Bump(Scope)` to turn `GUARANTEED_ALLOCATED` false
- **added:** `Bump::(try_)guaranteed_allocated_ref`

## 0.10.11 (2024-11-11)
- **added:** `map_in_place` to `(Mut)BumpVec`
- **added:** allow `map_in_place` to ZSTs regardless of alignment
- **fixed:** divide by zero panic when calling `map_in_place` with a ZST
- **fixed:** cases where constructing a `FixedBumpVec` of ZSTs resulted in a non-`usize::MAX` capacity
- **fixed:** potential UB when using `*fill*` or `map_in_place` with a ZST of alignment > 1

## 0.10.10 (2024-11-10)
- **added:** default generic parameters to `BumpPoolGuard`
- **added:** `(try_)map` to `BumpVec`
- **added:** `map_in_place` to `FixedBumpVec` and `BumpBox<[T]>`
- **fixed:** don't leak ZSTs when `(try_)alloc_slice_fill_with` panics
- **fixed:** don't allocate when the iterator of `(try_)alloc_iter_exact` panics

## 0.10.9 (2024-11-05)
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
