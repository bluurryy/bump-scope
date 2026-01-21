# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add `claim` api that allows you to enter scopes with a shared reference
- `alloc*` methods are now available for the `(Mut)BumpAllocatorTypedScope` traits
- `reserve_bytes` method is now available for the `BumpAllocatorTyped` trait
- Add new `BumpAllocator(Scope)` trait that allows you to be generic over `Bump` and `BumpScope`
- Add `SHRINKS` generic parameter, to toggle shrinking for the allocation api, `DEALLOCATES` no longer affects shrinking
- Improve documentation

### Fixed

- **Breaking:** Fix `no_std` builds with "serde" feature by depending on serde without default features

### Changed

- **Breaking:** The generic const parameters have been consolidated into a single `Settings` parameter.
- **Breaking:** Replaced allocator settings configuration methods with new `(borrow(_mut))_with_settings` methods
- **Breaking:** Bump allocator traits have been moved and renamed:
  - `(Mut)BumpAllocator` -> `traits::(Mut)BumpAllocatorCore`
  - `(Mut)BumpAllocatorScope` -> `traits::(Mut)BumpAllocatorCoreScope`
  - `(Mut)BumpAllocatorTypedScopeExt` -> `traits::(Mut)BumpAllocatorTyped`
  - `(Mut)BumpAllocatorScopeExt` -> `traits::(Mut)BumpAllocatorTypedScope`
- **Breaking:** Implement `bytemuck` and `zerocopy` allocator extension traits for all `T: BumpAllocatorTypedScope` and name them `BumpAllocatorTypedScopeExt`
- **Breaking:** `aligned` now takes a closure with a `&mut BumpScope` instead of a `BumpScope`
- For non-guaranteed-allocated `Stats`, `current_chunk` has been renamed to `get_current_chunk`
- For non-guaranteed-allocated `BumpScope`, `allocator` has been renamed to `get_allocator`
- Remove branch when allocating on a non-guaranteed-allocated `Bump(Scope)`, checking for a layout of `0`
- Depend on `serde_core` instead of `serde`

### Removed

- **Breaking:** Remove `alloc_layout` (`allocate_layout` provides the same functionality)
- **Breaking:** Remove deprecated api

## [1.5.1] - 2025-12-21

### Added

- Improve documentation

### Fixed

- Fix infinite loop when allocating a new chunk if the current chunk is not the largest by @Grimeh in [#127]

## [1.5.0] - 2025-12-12

### Added

- Add support for `allocator-api2` version `0.4` with a new "allocator-api2-04" feature

## [1.4.2] - 2025-12-08

### Fixed

- Fix compilation failure with `"nightly-allocator-api"` feature due to new `&mut A` where `A: Allocator` blanket impl

## [1.4.1] - 2025-11-22

### Added

- Improve documentation
- Refactor some internals

## [1.4.0] - 2025-10-05

### Added

- Add `init_move` to `BumpBox<[MaybeUninit<T>]>` to initialize slices with an `OwnedSlice`
- Add `split_off_first` and `split_off_last` to `BumpBox<[T]>`
- Improve documentation

## [1.3.1] - 2025-09-14

### Added

- Improve documentation

## [1.3.0] - 2025-09-08

### Added

- Add `DEALLOCATES` const parameter to toggle deallocation and shrinking
- Better compile error message for macros when "panic-on-alloc" is disabled

### Changed

- Rename `as_aligned_mut` to `as_mut_aligned`, deprecating the old naming

### Fixed

- Implement `NoDrop` for `BumpScope` regardless of `GUARANTEED_ALLOCATED`

## [1.2.1] - 2025-08-20

### Added

- Improve `BumpAllocatorTypedScopeExt` methods documentation

### Fixed

- Don't shrink `BumpVec` / `BumpString` with a `WithoutShrink` allocator (fixed `BumpAllocator::shrink_slice` to do nothing)

## [1.2.0] - 2025-08-14

### Added

- Add `pool()` method to `BumpPoolGuard` to get its `BumpPool`

### Fixed

- Fix rust-analyzer not showing hints for `BumpPool`

### Deprecated

- Deprecate `pool` field of `BumpPoolGuard`. Changing it can lead to undefined behavior! Use the new `pool()` method instead.


## [1.1.0] - 2025-08-14

### Added

- Document invariant that `shrink`ing a `BumpAllocator` will never error unless the alignment increases
- Add changelog to the crate documentation
- Add `Bump(Scope)::alloc_clone` to clone dynamically sized values into the bump allocator, requires the new "nightly-clone-to-uninit" feature

## [1.0.0] - 2025-08-13

### Added

- **Breaking:** `BumpAllocator` trait family has been reworked:
  - what was `BumpAllocator` is now `BumpAllocatorTypedScopeExt`, same with other traits that have an `*Ext` variant 
  - trait methods become public api
  - `BumpAllocator` becomes a sealed trait
  - `&dyn BumpAllocator` can be used for collections
- Add `Chunk::allocator()` which returns the base allocator
- Make `BumpScope(Guard)(Root)::allocator()` return an `&'a A` allocator instead of `&A`
- Make `BumpScopeGuardRoot::stats()` return `Stats<'a, ...>` instead of `Stats<'_, ...>`
- Relax base allocator trait bounds on `Bump(Scope)` struct and methods
- `alloc_try_with(_mut)` now works for non-GUARANTEED_ALLOCATED `Bump(Scope)`s
- **Breaking:** `guaranteed_allocated` conversion methods now take a closure parameter so you can initialize unallocated `Bump`s with a custom size or capacity
- Improve `Fixed*` collection documentation.
- Improve crate documentation.

### Changed

- **Breaking:** Raise minimum supported rust version to 1.85.1
- **Breaking:** Remove `without_shrink` and `without_dealloc` methods
- **Breaking:** Rename `guaranteed_allocated` to `into_guaranteed_allocated`
- **Breaking:** Rename `guaranteed_allocated_ref` to `as_guaranteed_allocated`
- **Breaking:** Rename `guaranteed_allocated_mut` to `as_mut_guaranteed_allocated`
- **Breaking:** Rename `not_guaranteed_allocated` to `into_not_guaranteed_allocated`
- **Breaking:** Rename `not_guaranteed_allocated_ref` to `as_not_guaranteed_allocated`
- Switch to rust edition 2024

### Fixed

- Fix small amount of wasted space when allocating `Mut*` collections on a downwards bumping allocator
- Fix potential UB when calling `allocator()` on an unallocated `Bump(Scope)`, now `allocator()` returns `Option<&A>` for `!GUARANTEED_ALLOCATED` `Bump(Scope)`s
- Fix allocating a layout of size zero on an unallocated `Bump` resulting in an assertion / potential UB
- Fix resetting an unallocated `Bump` resulting in an assertion / potential UB
- Fix deallocation of an outside object inside an `aligned` / `scoped` / `scoped_aligned` resulting in a misaligned bump position

## [0.17.4] - 2025-07-12

### Added

- Remove `#[cfg(feature = "alloc")]` bound on `BumpBox::into_box`.

### Fixed

- Make `allocator-api2-*` features compile without `alloc` feature.

## [0.17.3] - 2025-07-01

### Added

- Implement `Index(Mut)` for string types ([#78]).
- Implement `Index(Mut)` for `BumpBox<T>` where `T: Index(Mut)` ([#79]).
- Improve documentation.

### Fixed

- Fix bad chunk size calculation with base allocators of a high alignment or on platforms with a pointer size of 16, potentially leading to UB.

## [0.17.2] - 2025-06-16

### Added

- Improve documentation regarding allocator api, hashbrown, benchmarks and `BumpAllocator(Scope)`.

## [0.17.1] - 2025-06-07

### Added

- Add "nightly-dropck-eyepatch" feature to allow box and vectors to store types that don't outlive it. ([#70])
- Improve performance of `extend_from_slice_clone`. ([#69])
- Improve documentation.

## [0.17.0] - 2025-05-25

_This release clears out all the blockers that were standing in the way of a 1.0 release. Unless something unexpected comes up 1.0 will be released in the next month. If you've got thoughts, concerns, or suggestions let me know. Feedback is always welcome._

_If you are upgrading: please see [`UPGRADING.md#0.17.0`](UPGRADING.md#0.17.0)._

### Added

- Add `FixedBumpVec::{new, from_capacity_in}`.
- Add `{BumpVec, MutBumpVec, MutBumpVecRev}::from_owned_slice_in`.
- Add `{MutBumpVec, MutBumpVecRev}::from_iter_exact_in`.
- Add `as_non_null` to boxed str and string types.
- Add `AnyStats`, `AnyChunk` and associated types as type erased versions of their non-`Any*` variants.
- Add `Bump(Scope)::alloc_slice_move` to allocate `impl OwnedSlice`s (arrays, `Vec<T>`, `Box<[T]>` and so on).
- Add `BumpBox::as_raw` which returns a pointer to its contents.
- Add `Bump(Scope)::dealloc` to deallocate `BumpBox`es.
- Add "bytemuck" feature that adds extension traits just like "zerocopy-08" does.
- Add "nightly" feature that enables all other "nightly-*" features

### Changed

- **Breaking:** `bump-scope` no longer uses the types and traits from `allocator-api2` but defines its own `Allocator`, `AllocError` and `Global`. (#48)
  - The "allocator-api2-02" feature will make bump allocators implement the `Allocator` trait from `allocator-api2` version `0.2`.
  - The "allocator-api2-03" feature will make bump allocators implement the `Allocator` trait from `allocator-api2` version `0.3`.
  - The "nightly-allocator-api" feature will make bump allocators implement the nightly `Allocator` trait from `core`.
  - Each allocator api feature comes with a compatibility wrapper type in `bump_scope::alloc::compat` to make their `Allocator` implementor implement this crate's `Allocator` and vice versa
- **Breaking:** The `zerocopy` feature has been renamed to `zerocopy-08`. All methods that this feature added are no longer inherent methods but are provided via extension traits from `bump_scope::zerocopy_08`.
- **Breaking:** Change `bump_format!` implementation to not call `$bump.as_scope()` but use `$bump` as is. This is what `mut_bump_format!` and the `bump_vec` macros are already doing.
- **Breaking:** The `stats` method of `BumpAllocator`, strings and vectors return `AnyStats` now instead of `Stats`.
- **Breaking:** Add `A` and `UP` generic parameters to `Stats`, `Chunk` and associated types.
- **Breaking:** `Stats` is no longer re-exported at the crate level, you can import it from `bump_scope::stats::Stats`.

### Deprecated

- Deprecate `FixedBumpVec::EMPTY`, use `FixedBumpVec::new()` instead.
- Deprecate `FixedBumpString::EMPTY`, use `FixedBumpString::new()` instead.
- Deprecate `alloc_fixed_vec`, use `FixedBumpVec::with_capacity_in` instead.
- Deprecate `alloc_fixed_string`, use `FixedBumpString::with_capacity_in` instead.
- Deprecate `alloc_fixed_string`, use `FixedBumpString::with_capacity_in` instead.
- Deprecate `from_array_in`, use `from_owned_slice_in` instead.
- Deprecate renamed `as_non_null_ptr` to `as_non_null`.
- Deprecate `as_non_null_slice`, too niche of an api.
- Deprecate `BumpBox::into_unsized`, use `unsize_bump_box!` instead.
  
### Removed

- **Breaking:** Remove deprecated methods `(try_)extend_from_array`
- **Breaking:** Remove `BumpBox::deallocate_in`, use `Bump(Scope)::dealloc` instead.

### Fixed

- Fix build failing with `serde` feature and without `alloc` feature.
- Fix `Stats` and `Chunk` not reporting accurate sizes and pointers if the base allocator is not zero sized.

## [0.16.5] - 2025-05-09

### Added

- Add `pop_if` method to vector types.
- Implement `Default` for vector and string types.
- Implement `FromIterator` for vector types.

## [0.16.4] - 2025-04-10

### Added

- With the new `"nightly-fn-traits"` feature, `BumpBox<T>` implements the `Fn*` traits if `T` does (just like `Box`). This makes `BumpBox<T: FnOnce + ?Sized>` callable.

## [0.16.3] - 2025-02-21

### Added

- Add `capacity` field to the `Debug` representation of `Stats` and `Bump(Scope(Guard(Root)))`.

## [0.16.2] - 2025-02-21

### Added

- Improve documentation.

### Changed

- Simpler and shorter `Debug` output for `Bump(Scope(Guard(Root)))` and `Chunk(NextIter, PrevIter)`.
- Remove `rust-patterns` from `package.categories`, added `no-std::no-alloc`.

## [0.16.1] - 2025-01-22

### Added

- Add `(try_)replace_range` and `as_(mut_)ptr` to strings.
- Improve docs somewhat.

### Fixed

- Fix double-dropping elements when calling `into_flattened`

## [0.16.0] - 2025-01-10

### Added

- Add `unsize_bump_box` to unsize `BumpBox`es on stable.
- Improve docs for `split_off`.

### Changed

- **Breaking:** Replace `OwnedSlice`'s `owned_slice_ptr` method with `owned_slice_ref` which returns a reference instead of a pointer.

### Fixed

- Fix `Write` impl for `FixedBumpVec<u8>` not actually writing.

## [0.15.2] - 2025-01-06

### Added

- Implement `Default` for `Stats`, `ChunkPrevIter`, `ChunkNextIter`, `WithoutShrink` and `WithoutDealloc`.
- Improve documentation with more examples.

### Fixed

- Fix `append` implementation of `MutBumpVecRev` to now account for `take_owned_slice` panicking.

## [0.15.1] - 2025-01-04

### Added

- Implement `TakeOwnedSlice` for `vec::IntoIter` and `vec::Drain`.
- Improve speed and time complexity of `split_off`.
- Improve crate docs, docs for owned slice and `split_off`.

## [0.15.0] - 2025-01-02

### Added

- Add new `OwnedSlice` implementations (used for `append`).
- Add `BumpBox<[T; N]>::into_unsized` returning `BumpBox<[T]>`.

### Changed

- **Breaking:** Rename `unchecked_push` to `push_unchecked`.
- **Breaking:** Rename `unchecked_push_with` to `push_with_unchecked`.
- **Breaking:** Redesign `OwnedSlice` trait (used by `append`). 
- **Breaking:** `append` now accepts `alloc::vec::Vec` instead of `allocator_api2::vec::Vec`.
- **Breaking:** Redesign `split_off`. It no longer allocates, but splits in place. It now takes a range parameter instead of a position. Now available for `Fixed*` and `BumpBox<str>`.
- **Breaking:** Add safety condition of splittable memory blocks to `BumpAllocator`.

### Deprecated

- Deprecate `extend_from_array` in favor of `append`.

## [0.14.0] - 2024-12-12

### Added

- **Breaking:** Fix `scoped_aligned`'s closure to take a `BumpScope` with `NEW_MIN_ALIGN` instead of `MIN_ALIGN`.
- **Breaking:** `aligned`'s closure now takes a `BumpScope` with lifetime `'a` instead of `'_`.
- Improve documentation.

## [0.13.1] - 2024-11-30

### Added

- Improve documentation of `Bump` and `BumpScope` and in some other places.

## [0.13.0] - 2024-11-29

### Added

- Implement `core::error::Error` for error types when rust version allows.

### Changed

- **Breaking:** Renamed `stats` method on strings and vectors to `allocator_stats`.

### Removed

- **Breaking:** Remove `"nightly-const-refs-to-static"` feature. `Bump::unallocated` is now automatically const for any rust version since 1.83.
- **Breaking:** Remove `Bump(Scope)`'s `without_dealloc` and `without_shrink`. Use `WithoutDealloc(&bump)` and `WithoutShrink(&bump)` instead.
- **Breaking:** feature gate panicking functions of `(Mut)BumpAllocator` with `"panic-on-alloc"` (those functions are doc hidden for now)

### Fixed

- Fix potential UB in `write_vectored` in pathologic case of `usize` overflow when `bufs` contains a large amount of duplicate `IoSlice`s.

## [0.12.3] - 2024-11-23

### Fixed

- Interior nuls being ignored for `alloc_cstr_fmt_mut` and `MutBumpString::into_cstr` (unsound).

## [0.12.2] - 2024-11-22

### Fixed

- `CStr` now stops at the first nul. Before, interior nuls were ignored (unsound).

## [0.12.1] - 2024-11-21

### Added

- Implement `NoDrop` for `CStr`, `OsStr` and `Path`.
- Add `alloc_cstr`, `alloc_cstr_from_str`, `alloc_cstr_fmt` and `alloc_cstr_fmt_mut`.
- Add `(try_)into_cstr` to `(Mut)BumpString`.

## [0.12.0] - 2024-11-17

### Changed

- Redesigned `OwnedSlice` trait.

## [0.11.1] - 2024-11-17

### Fixed

- Fix double dropping the elements of `MutBumpVec` when growing.

## [0.11.0] - 2024-11-16

### Added

- **Breaking:** Add `"panic-on-alloc"` feature which enables the panicking alloc functions (on by default); if you had `default-features = false` before you might need to enable this feature now.
- **Breaking:** `append` methods now take `impl OwnedSlice<T>` instead
- Add more general `PartialEq` for vectors and strings.
- Add `not_guaranteed_allocated(_ref)` methods on `Bump(Scope)` to turn `GUARANTEED_ALLOCATED` false.
- Add `Bump::(try_)guaranteed_allocated_ref`.

### Changed

- **Breaking:** Vectors and strings now take a single `A` generic parameter instead of the `'b, 'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool` of before.
- **Breaking:** `vec`-like macros now take the bump allocator as is instead of by `$bump.as_scope()`. You will need to change `bump_vec![in bump` to `bump_vec![in &bump` and `bump_vec![in bump` to `bump_vec![in &mut bump` unless those `bump` are already references.
- **Breaking:** `bump` methods on vectors and strings have been renamed to `allocator`, the old `allocator` method which returned the base allocator is gone.
- **Breaking:** `Stats<'a, UP>` is now `Stats<'a, GUARANTEED_ALLOCATED>`
- **Breaking:** Rename `into_guaranteed_allocated` to `guaranteed_allocated`, `as_guaranteed_allocated` to `guaranteed_allocated_ref` and `as_guaranteed_allocated_mut` to `guaranteed_allocated_mut`.
- **Breaking:** Rename `MutBumpVecRev`'s `as_nonnull_ptr` to `as_non_null_ptr` and `as_nonnull_slice` to `as_non_null_slice`.
- **Breaking:** `Chunk`, `ChunkNextIter` and `ChunkPrevIter` have moved to the `stats` module.

### Removed

- **Breaking:** Remove `WithLifetime`.
- **Breaking:** Remove `GuaranteedAllocatedStats` in favor of `Stats<'a, true>`
- **Breaking:** Remove deprecated functions `BumpBox<[u8]>::into_boxed_str(_unchecked)`. Use `BumpBox<str>::from_utf8(_unchecked)` instead.

### Fixed

- `BumpVec::split_off` now retains the capacity of `self` like the docs say.

## [0.10.11] - 2024-11-11

### Added

- Add `map_in_place` to `(Mut)BumpVec`.
- Allow `map_in_place` to ZSTs regardless of alignment.

### Fixed

- Fix divide by zero panic when calling `map_in_place` with a ZST.
- Fix cases where constructing a `FixedBumpVec` of ZSTs resulted in a non-`usize::MAX` capacity.
- Fix potential UB when using `*fill*` or `map_in_place` with a ZST of alignment > 1.

## [0.10.10] - 2024-11-10

### Added

- Add default generic parameters to `BumpPoolGuard`
- Add `(try_)map` to `BumpVec`.
- Add `map_in_place` to `FixedBumpVec` and `BumpBox<[T]>`.

### Fixed

- Fix leaking ZSTs when `(try_)alloc_slice_fill_with` panics.
- Don't allocate when the iterator of `(try_)alloc_iter_exact` panics.

## [0.10.9] - 2024-11-05

### Added

- Add `(try_)get_with_size` and `(try_)get_with_capacity` to `BumpPool`.
- `BumpPoolGuard`'s `pool` field is now `pub`.

## [0.10.8] - 2024-10-30

### Added

- `FromUtf8Error` now implements `Clone`, `PartialEq`, `Eq`, `Display` and `Error`.

## [0.10.7] - 2024-10-30

### Added

- Add minimum capacity when growing vectors and strings (just like `Vec`).
- Add `(try_)from_utf8_lossy_in` for strings.
- Add `(try_)from_utf16(_lossy)_in` for strings.

## [0.10.6] - 2024-10-26

### Added

- Add default generic allocator parameter for `bump_vec::Splice`.

### Fixed

- Fix potential UB when using `BumpVec::splice`.

## [0.10.5] - 2024-10-19

### Added

- Add `splice` to `BumpVec`.
- Add `reserve_exact` to vectors and strings.
- Add `spare_capacity_mut` and `from_iter(_exact)_in` to vectors.

## [0.10.4] - 2024-10-18

### Added

- Increase performance of `(Mut)BumpString::from_str_in`.

## [0.10.3] - 2024-10-15

### Added

- Make `owned_str` module `pub`.

## [0.10.2] - 2024-10-15

### Added

- Add missing string methods to `BumpBox<str>`: `len`, `is_empty`, `set_len`, `retain`, `clear`, `as(_mut)_ptr`, `remove`, `as_mut_bytes`, `from_utf8(_unchecked)`.
- Add string methods `pop`, `truncate`, `retain`, `drain`.
- Make more `len` and `is_empty` methods `const`.
- Add `split_off` to suitable vector and string types.
- Add `reserve` methods to `FixedBumpString`.
- Implement `Default` for `FixedBumpString`.
- Implement `Add<&str>` for `BumpString`.
- Implement `Extend<char>` and `Extend<&char>` for suitable string types.
- Implement `Clone` for `BumpVec` and `BumpString`

### Deprecated

- Deprecate `BumpBox`'s `into_boxed_str(_unchecked)` in favor of `from_utf8(_unchecked)`.

## [0.10.1] - 2024-10-14

### Added

- `MutBumpVecRev::{ append, into_flattened, unchecked_push(_with), as_non_null_{ptr, slice} }`.

### Fixed

- Fix `MutBumpVecRev::extend_from_within_clone` doing nothing for ZSTs.
- Fix potential UB in `MutBumpVecRev::extend_from_slice_clone` when clone panics.

## [0.10.0] - 2024-10-07

### Changed

- **Breaking:** Upgrade `zerocopy` dependency to version `0.8.2`.
- **Breaking:** Remove deprecated methods `FixedBumpVec::{ layout, as_(mut_)boxed_slice }`.

## [0.9.2] - 2024-09-28

### Added

- Remove a duplicate call to `shrink_to_fit` in `alloc_iter` and `alloc_fmt`.

### Deprecated

- Deprecate `FixedBumpVec`'s `layout` and `as(_mut)_boxed_slice`.

## [0.9.1] - 2024-09-23

### Added

- Remove a branch when doing fallible sized allocations like `try_alloc_*` ([#34]).

## [0.9.0] - 2024-09-18

### Added

- Add `from_parts` and `into_parts` to `BumpVec` and `BumpString`.
- Add `FixedBumpString::into_string` which returns a `BumpString`.

### Changed

- **Breaking:** `BumpVec` and `BumpString` now deallocate on drop, and shrink when calling `into_(boxed_)slice`.
- **Breaking:** `BumpVec::into_iter` now returns `bump_vec::IntoIter` which deallocates on drop.
- **Breaking:** `IntoIter`, `Drain` and `ExtractIf` have been moved to the `owned_slice` module.

## [0.8.2] - 2024-09-15

### Added

- `NoDrop` blanket impl for `[T]` is more general, replacing `Copy` bound with `NoDrop`

## [0.8.1] - 2024-09-15

### Added

- Implement `NoDrop` for `BumpBox`, `FixedBumpVec`, `FixedBumpString` and `BumpScope`.

## [0.8.0] - 2024-09-03

### Added

- Add `(try_)alloc_try_with_mut` as an optimized version of `(try_)alloc_try_with`.
- `reset_to` now takes a `&self` instead of `&mut self`.

### Changed

- **Breaking:** `(try_)alloc_try_with` now requires a guaranteed allocated `Bump(Scope)`.

### Fixed

- Fix panic message on formatting failure to mention formatting failure, instead of a "capacity overflow".
- Fix `(try_)alloc_try_with` UB when allocating inside the provided closure, memory leak when using an unallocated bump allocator.
- Fix potential UB when using `alloc_iter_mut*` or `MutBump*` collections.
  

## [0.7.0] - 2024-08-31

### Added

- **Breaking:** Allow returning values from closure arguments of `scoped`, `aligned` and `scoped_aligned`.

## [0.6.0] - 2024-08-25

### Changed

- **Breaking:** `(Fixed)BumpString::as_mut_vec` now returns `&mut` as it should instead of `&`.
- **Breaking:** Remove `GUARANTEED_ALLOCATED` parameter from `BumpPool(Guard)`.

### Removed

- **Breaking:** Remove deprecated `(try_)alloc_slice_zeroed` in favor of `(try_)alloc_zeroed_slice`.

## [0.5.9] - 2024-08-25

### Added

- Removed a branch when bumping downwards (#25).

### Fixed

- Fix UB when using a base allocator with an alignment greater than `2 * size_of::<usize>()` ([#32]).

## [0.5.8] - 2024-08-22

### Fixed

- Fix `Mut*` collections' `into(_boxed)_slice` as well as `alloc_iter_mut(_rev)` and `alloc_fmt_mut` to not take up more than the necessary space.

## [0.5.7] - 2024-08-18

### Added

- Add `from_utf8_unchecked` for all string types.
- Add `BumpString::into_fixed_string`.
- Add `BumpBox<[T]>::partition`.

## [0.5.6] - 2024-08-17

### Fixed

- Fix `alloc_iter` and `alloc_fmt` to not take up more than the necessary space.

## [0.5.5] - 2024-07-26

### Fixed

- Fix Rust Analyzer breaking for other structs with a default `Global` allocator parameter.

## [0.5.4] - 2024-07-26

### Fixed

- Fix Rust Analyzer breaking for `Bump`.

## [0.5.3] - 2024-06-28

### Added

- Add `extend_zeroed` for vectors and strings.
- Add `resize_zeroed` for vectors.

## [0.5.2] - 2024-06-09

### Deprecated

- Deprecate `alloc_slice_zeroed` in favor of `alloc_zeroed_slice`.

## [0.5.1] - 2024-06-08

### Added

- `zerocopy` feature that adds `alloc_zeroed`, `alloc_slice_zeroed` and `BumpBox::init_zeroed`.

## [0.5.0] - 2024-05-21

### Added

- Any allocator that implements `Default` can now be used as a base allocator (before it was just `Global`).
- Add `bump` method to `BumpVec` and `BumpString` which returns the bump allocator.
- Add `checkpoint` and `reset_to` to non-`GUARANTEED_ALLOCATED` bump allocators, adds a safety condition to `reset_to`

### Changed

- **Breaking:** `BumpPool::new` is now no longer const, you can the same const constructor with `BumpPool::new_in(Global)`.
- **Breaking:** You can no longer be generic over `GUARANTEED_ALLOCATED` in some ways due to the `BaseAllocator` bound.

## [0.4.0] - 2024-05-19

### Added

- `impl From<GuaranteedAllocatedStats> for Stats`
- `BumpBox::<[MaybeUninit<T>]>::init_fill_iter`
- `BumpBox::deallocate_in`

### Changed

- **Breaking:** renamed `Stats::to_stats` to `to_guaranteed_stats`

### Removed

- **Breaking:** removed deprecated `BumpBox::into_fixed_vec` and `into_fixed_string`.

## [0.3.1] - 2024-05-01

### Added

- Optimization to not align the bump pointer when the size happens to be a multiple of `MIN_ALIGN` ([#12])

### Fixed

- Fix crash in debug mode when using `alloc_iter_mut(_rev)` or calling `into_(boxed_)slice` on a `MutBumpVec(Rev)` ([#16]).

## [0.3.0] - 2024-04-22

### Added

- Add `guaranteed_allocated_stats` which returns `GuaranteedAllocatedStats`.
- Make `BumpPool::new` `const`.

### Changed

- **Breaking:** renamed `Stats` to `GuaranteedAllocatedStats`
- **Breaking:** renamed `MaybeUnallocatedStats` to `Stats`
- **Breaking:** `stats` now always returns `Stats` and is always available
- **Breaking:** renamed `into_init` and `as_init(_mut)` to `into_guaranteed_allocated` and `as_guaranteed_allocated(_mut)`

## [0.2.1] - 2024-04-21

### Fixed

- Fixed docs and changelog.

## [0.2.0] - 2024-04-21

### Added

- `Bump::uninit` to create a `Bump` without allocation (and `const` with feature `nightly-const-refs-to-static`) ([#7](https://github.com/bluurryy/bump-scope/issues/7))

### Changed

- **Breaking:** adds the `INIT` const param to signify whether the bump has an allocated chunk
- **Breaking:** `BumpVec::into_iter` returns `IntoIter<'a, T>` instead of `IntoIter<'b, T>` ([#8](https://github.com/bluurryy/bump-scope/issues/8))

## [0.1.8] - 2024-04-11

### Added

- Add `serde::Serialize` implementations for `BumpBox`, strings and vectors.
- Add `serde::DeserializeSeed` implementations for strings and vectors.

## [0.1.7] - 2024-04-07

### Added

- Add `BumpPool` along with `BumpPoolGuard`.
- Implement `Send` and `Sync` for `BumpBox`, `FixedBumpVec` and `FixedBumpString`.

## [0.1.6] - 2024-04-07

### Fixed

- Fixed ZST allocation with respect to `drop`, `clone` and `default` calls.
- Fixed `alloc_with` and `alloc_slice_fill_with` not calling `f` for ZSTs.

## [0.1.5] - 2024-04-05

### Added

- Add `BumpVec::into_fixed_vec`.
- Add `FixedBumpVec::into_vec`.
- Add fallible `FixedBumpVec` api.
- Add `FixedBumpString`.
- Add `from_init` and `from_uninit` methods for `FixedBumpVec` and `FixedBumpString`.

### Deprecated

- Deprecate `BumpBox::into_fixed_vec` and `BumpBox::into_fixed_string`.

## [0.1.4] - 2024-04-02

### Added

- Add `BumpString::shrink_to_fit`.

## [0.1.3] - 2024-04-02

### Fixed

- Fix `aligned` and `scoped_aligned` not aligning.

## [0.1.2] - 2024-03-29

### Added

- Add `BumpVec::shrink_to_fit`.

### Fixed

- Fix unsoundness when allocating large slices.

## [0.1.1] - 2024-03-28

### Fixed

- Fix `BumpVec` and `BumpString` growing.

## [0.1.0] - 2024-03-28

### Changed

- **Breaking:** `BumpVec` and `BumpString` now take an `&Bump(Scope)`, `MutBumpVec` and `MutBumpString` take a `&mut Bump(Scope)`. ([#3])

## [0.0.1] - 2024-03-27

### Added

- `alloc_iter` and `alloc_fmt` don't require the `alloc` feature anymore.

### Fixed

- Allocating on a downwards `Bump` with a layout of `[u8; 0]` no longer panics.

## [0.0.0] - 2024-03-26

[#127]: https://github.com/bluurryy/bump-scope/pull/127
[#79]: https://github.com/bluurryy/bump-scope/pull/79
[#78]: https://github.com/bluurryy/bump-scope/pull/78
[#70]: https://github.com/bluurryy/bump-scope/pull/70
[#69]: https://github.com/bluurryy/bump-scope/pull/69
[#48]: https://github.com/bluurryy/bump-scope/pull/48
[#34]: https://github.com/bluurryy/bump-scope/pull/34
[#32]: https://github.com/bluurryy/bump-scope/issues/32
[#25]: https://github.com/bluurryy/bump-scope/issues/25
[#16]: https://github.com/bluurryy/bump-scope/issues/16
[#12]: https://github.com/bluurryy/bump-scope/issues/12
[#3]: https://github.com/bluurryy/bump-scope/issues/3

<!-- next-url -->
[Unreleased]: https://github.com/bluurryy/bump-scope/compare/v1.5.1...HEAD
[1.5.1]: https://github.com/bluurryy/bump-scope/releases/tag/v1.5.1
[1.5.0]: https://github.com/bluurryy/bump-scope/releases/tag/v1.5.0
[1.4.2]: https://github.com/bluurryy/bump-scope/releases/tag/v1.4.2
[1.4.1]: https://github.com/bluurryy/bump-scope/releases/tag/v1.4.1
[1.4.0]: https://github.com/bluurryy/bump-scope/releases/tag/v1.4.0
[1.3.1]: https://github.com/bluurryy/bump-scope/releases/tag/v1.3.1
[1.3.0]: https://github.com/bluurryy/bump-scope/releases/tag/v1.3.0
[1.2.1]: https://github.com/bluurryy/bump-scope/releases/tag/v1.2.1
[1.2.0]: https://github.com/bluurryy/bump-scope/releases/tag/v1.2.0
[1.1.0]: https://github.com/bluurryy/bump-scope/releases/tag/v1.1.0
[1.0.0]: https://github.com/bluurryy/bump-scope/releases/tag/v1.0.0
[0.17.4]: https://github.com/bluurryy/bump-scope/releases/tag/v0.17.4
[0.17.3]: https://github.com/bluurryy/bump-scope/releases/tag/v0.17.3
[0.17.2]: https://github.com/bluurryy/bump-scope/releases/tag/v0.17.2
[0.17.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.17.1
[0.17.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.17.0
[0.16.5]: https://github.com/bluurryy/bump-scope/releases/tag/v0.16.5
[0.16.4]: https://github.com/bluurryy/bump-scope/releases/tag/v0.16.4
[0.16.3]: https://github.com/bluurryy/bump-scope/releases/tag/v0.16.3
[0.16.2]: https://github.com/bluurryy/bump-scope/releases/tag/v0.16.2
[0.16.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.16.1
[0.16.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.16.0
[0.15.2]: https://github.com/bluurryy/bump-scope/releases/tag/v0.15.2
[0.15.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.15.1
[0.15.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.15.0
[0.14.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.14.0
[0.13.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.13.1
[0.13.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.13.0
[0.12.3]: https://github.com/bluurryy/bump-scope/releases/tag/v0.12.3
[0.12.2]: https://github.com/bluurryy/bump-scope/releases/tag/v0.12.2
[0.12.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.12.1
[0.12.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.12.0
[0.11.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.11.1
[0.11.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.11.0
[0.10.11]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.11
[0.10.10]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.10
[0.10.9]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.9
[0.10.8]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.8
[0.10.7]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.7
[0.10.6]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.6
[0.10.5]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.5
[0.10.4]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.4
[0.10.3]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.3
[0.10.2]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.2
[0.10.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.1
[0.10.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.10.0
[0.9.2]: https://github.com/bluurryy/bump-scope/releases/tag/v0.9.2
[0.9.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.9.1
[0.9.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.9.0
[0.8.2]: https://github.com/bluurryy/bump-scope/releases/tag/v0.8.2
[0.8.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.8.1
[0.8.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.8.0
[0.7.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.7.0
[0.6.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.6.0
[0.5.9]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.9
[0.5.8]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.8
[0.5.7]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.7
[0.5.6]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.6
[0.5.5]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.5
[0.5.4]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.4
[0.5.3]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.3
[0.5.2]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.2
[0.5.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.1
[0.5.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.5.0
[0.4.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.4.0
[0.3.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.3.1
[0.3.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.3.0
[0.2.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.2.1
[0.2.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.2.0
[0.1.8]: https://github.com/bluurryy/bump-scope/releases/tag/v0.1.8
[0.1.7]: https://github.com/bluurryy/bump-scope/releases/tag/v0.1.7
[0.1.6]: https://github.com/bluurryy/bump-scope/releases/tag/v0.1.6
[0.1.5]: https://github.com/bluurryy/bump-scope/releases/tag/v0.1.5
[0.1.4]: https://github.com/bluurryy/bump-scope/releases/tag/v0.1.4
[0.1.3]: https://github.com/bluurryy/bump-scope/releases/tag/v0.1.3
[0.1.2]: https://github.com/bluurryy/bump-scope/releases/tag/v0.1.2
[0.1.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.1.1
[0.1.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.1.0
[0.0.1]: https://github.com/bluurryy/bump-scope/releases/tag/v0.0.1
[0.0.0]: https://github.com/bluurryy/bump-scope/releases/tag/v0.0.0