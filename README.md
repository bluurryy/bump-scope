# bump-scope

[![Crates.io](https://img.shields.io/crates/v/bump-scope.svg)](https://crates.io/crates/bump-scope)
[![Documentation](https://img.shields.io/docsrs/bump-scope)](https://docs.rs/bump-scope)
[![Rust](https://img.shields.io/crates/msrv/bump-scope)](#)
[![License](https://img.shields.io/crates/l/bump_scope)](#license)
[![Build Status](https://github.com/bluurryy/bump-scope/workflows/Rust/badge.svg)](https://github.com/bluurryy/bump-scope/actions/workflows/rust.yml)

<!-- cargo-rdme start -->

A fast bump allocator that supports allocation scopes / checkpoints. Aka an arena for values of arbitrary types.

## What is bump allocation?
A bump allocator owns a big chunk of memory. It has a pointer that starts at one end of that chunk.
When an allocation is made that pointer gets aligned and bumped towards the other end of the chunk by the allocation's size.
When its chunk is full, this allocator allocates another chunk with twice the size.

This makes allocations very fast. The drawback is that you can't reclaim memory like you do with a more general allocator.
Memory for the most recent allocation *can* be reclaimed. You can also use [scopes, checkpoints](#scopes-and-checkpoints) and [`reset`](Bump::reset) to reclaim memory.

A bump allocator is great for *phase-oriented allocations* where you allocate objects in a loop and free them at the end of every iteration.
```rust
use bump_scope::Bump;
let mut bump: Bump = Bump::new();

loop {
    // use bump ...
    bump.reset();
}
```
The fact that the bump allocator allocates ever larger chunks and [`reset`](Bump::reset) only keeps around the largest one means that after a few iterations, every bump allocation
will be done on the same chunk and no more chunks need to be allocated.

The introduction of scopes makes this bump allocator also great for temporary allocations and stack-like usage.

## Comparison to [`bumpalo`](https://docs.rs/bumpalo)

Bumpalo is a popular crate for bump allocation.
This crate was inspired by bumpalo and [Always Bump Downwards](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html)
(but ignores the title).

Unlike `bumpalo`, this crate...
- Supports [scopes and checkpoints](#scopes-and-checkpoints).
- Drop is always called for allocated values unless explicitly leaked or forgotten.
  - `alloc*` methods return a `BumpBox<T>` which owns and drops `T`. Types that don't need dropping can be turned into references with `into_ref` and `into_mut`.
- You can allocate a slice from *any* `Iterator` with `alloc_iter`.
- Every method that panics on allocation failure has a fallible `try_*` counterpart.
- `Bump`'s base allocator is generic.
- Won't try to allocate a smaller chunk if allocation failed.
- No built-in allocation limit. You can provide an allocator that enforces an allocation limit (see `tests/limit_memory_usage.rs`).
- Allocations are a bit more optimized. (see [`bump-scope-inspect-asm/out/x86-64`](https://github.com/bluurryy/bump-scope-inspect-asm/tree/main/out/x86-64) and [benchmarks](https://bluurryy.github.io/bump-scope/criterion/report/))
- [You can choose the bump direction.](#bumping-upwards-or-downwards) Bumps upwards by default.
- [You can choose the minimum alignment.](#minimum-alignment) `1` by default.

## Allocator Methods

The bump allocator provides many methods to conveniently allocate values, strings and slices.
Have a look at the documentation of `Bump` for a method overview.

## Scopes and Checkpoints

You can create scopes to make allocations that live only for a part of its parent scope.
Entering and exiting scopes is virtually free. Allocating within a scope has no overhead.

You can create a new scope either with a `scoped` closure or with a `scope_guard`:
```rust
use bump_scope::Bump;

let mut bump: Bump = Bump::new();

// you can use a closure
bump.scoped(|mut bump| {
    let hello = bump.alloc_str("hello");
    assert_eq!(bump.stats().allocated(), 5);

    bump.scoped(|bump| {
        let world = bump.alloc_str("world");

        println!("{hello} and {world} are both live");
        assert_eq!(bump.stats().allocated(), 10);
    });

    println!("{hello} is still live");
    assert_eq!(bump.stats().allocated(), 5);
});

assert_eq!(bump.stats().allocated(), 0);

// or you can use scope guards
{
    let mut guard = bump.scope_guard();
    let mut bump = guard.scope();

    let hello = bump.alloc_str("hello");
    assert_eq!(bump.stats().allocated(), 5);

    {
        let mut guard = bump.scope_guard();
        let bump = guard.scope();

        let world = bump.alloc_str("world");

        println!("{hello} and {world} are both live");
        assert_eq!(bump.stats().allocated(), 10);
    }

    println!("{hello} is still live");
    assert_eq!(bump.stats().allocated(), 5);
}

assert_eq!(bump.stats().allocated(), 0);
```
You can also use the unsafe `checkpoint` api to reset the bump pointer to a previous location.
```rust
let mut bump: Bump = Bump::new();
let checkpoint = bump.checkpoint();

{
    let hello = bump.alloc_str("hello");
    assert_eq!(bump.stats().allocated(), 5);
}

unsafe { bump.reset_to(checkpoint); }
assert_eq!(bump.stats().allocated(), 0);
```

## Collections
`bump-scope` provides bump allocated variants of `Vec` and `String` called `BumpVec` and `BumpString`.
They are also available in the following variants:
- `Fixed*` for fixed capacity collections
- `Mut*` for collections optimized for a mutable bump allocator

##### Api changes
The collections are designed to have a similar api to their std counterparts but they do make some breaking changes:
- [`split_off`](BumpVec::split_off) —  splits the collection in place without allocation; the parameter is a range instead of a single index
- [`retain`](BumpVec::retain) —  takes a closure with a `&mut T` parameter like [`Vec::retain_mut`](alloc::vec::Vec::retain_mut)

##### New features
- [`append`](BumpVec::append) —  allows appending all kinds of owned slice types like `[T; N]`, `Box<[T]>`, `Vec<T>`, `Drain<T>` etc
- [`map`](BumpVec::map) —  maps the elements, potentially reusing the existing allocation
- [`map_in_place`](BumpVec::map_in_place) —  maps the elements without allocation
- conversions between the regular collections, their `Fixed*` variants and `BumpBox<[T]>` / `BumpBox<str>`

## Parallel Allocation
`Bump` is `!Sync` which means it can't be shared between threads.

To bump allocate in parallel you can use a `BumpPool`.

## Allocator API
`Bump` and `BumpScope` implement either the `Allocator` trait from
[`allocator_api2`](https://docs.rs/allocator-api2/0.3.0/allocator_api2/alloc/trait.Allocator.html)
or from [`alloc`](https://doc.rust-lang.org/nightly/alloc/alloc/trait.Allocator.html)
with the "nightly-allocator-api" feature.
They can be used to allocate collections.

A bump allocator can grow, shrink and deallocate the most recent allocation.
When bumping upwards it can even do so in place.
Growing allocations other than the most recent one will require a new allocation and the old memory block becomes wasted space.
Shrinking or deallocating allocations other than the most recent one does nothing, which means wasted space.

A bump allocator does not require `deallocate` or `shrink` to free memory.
After all, memory will be reclaimed when exiting a scope, calling `reset` or dropping the `Bump`.
You can wrap a bump allocator in a type that makes `deallocate` and `shrink` a no-op using `WithoutDealloc` and `WithoutShrink`.
```rust
use bump_scope::{ Bump, WithoutDealloc };
use allocator_api2_03::boxed::Box;

let bump: Bump = Bump::new();

let boxed = Box::new_in(5, &bump);
assert_eq!(bump.stats().allocated(), 4);
drop(boxed);
assert_eq!(bump.stats().allocated(), 0);

let boxed = Box::new_in(5, WithoutDealloc(&bump));
assert_eq!(bump.stats().allocated(), 4);
drop(boxed);
assert_eq!(bump.stats().allocated(), 4);
```

## Feature Flags
* **`std`** *(enabled by default)* —  Adds `BumpPool` and implementations of `std::io` traits.
* **`alloc`** *(enabled by default)* —  Adds `Global` as the default base allocator and some interactions with `alloc` collections.
* **`panic-on-alloc`** *(enabled by default)* —  Adds functions and traits that will panic when the allocation fails.
  Without this feature, allocation failures cannot cause panics, and only
  `try_`-prefixed allocation methods will be available.
* **`serde`** —  Adds `Serialize` implementations for `BumpBox`, strings and vectors, and `DeserializeSeed` for strings and vectors.
* **`zerocopy`** —  Adds `alloc_zeroed(_slice)`, `init_zeroed`, `resize_zeroed` and `extend_zeroed`.
* **`allocator-api2-02`** —  Makes `Bump(Scope)` implement `allocator_api2::Allocator` and
  allows using `allocator_api2::Allocator`s as base allocators via
  `AllocatorApiV02Compat`.
* **`allocator-api2-03`** —  Makes `Bump(Scope)` implement `allocator_api2::Allocator` and
  allows using `allocator_api2::Allocator`s as base allocators via
  `AllocatorApiV03Compat`.

 #### Nightly features
* **`nightly-allocator-api`** —  Makes `Bump(Scope)` implement `alloc::Allocator` and
  allows using `alloc::Allocator`s as base allocators via
  `AllocatorNightlyCompat`.
 
  This will also enable "allocator-api2-02/nightly".
* **`nightly-coerce-unsized`** —  Makes `BumpBox<T>` implement [`CoerceUnsized`](core::ops::CoerceUnsized).
  With this `BumpBox<[i32;3]>` coerces to `BumpBox<[i32]>`, `BumpBox<dyn Debug>` and so on.
  You can unsize a `BumpBox` in stable without this feature using [`unsize_bump_box`].
* **`nightly-exact-size-is-empty`** —  Implements `is_empty` manually for some iterators.
* **`nightly-trusted-len`** —  Implements `TrustedLen` for some iterators.
* **`nightly-fn-traits`** —  Implements `Fn*` traits for `BumpBox<T>`. Makes `BumpBox<T: FnOnce + ?Sized>` callable. Requires alloc crate.

## Bumping upwards or downwards?
Bump direction is controlled by the generic parameter `const UP: bool`. By default, `UP` is `true`, so the allocator bumps upwards.

Bumping upwards has the advantage that the most recent allocation can be grown and shrunk in place.
This benefits collections as well as <code>alloc_iter(_mut)</code> and <code>alloc_fmt(_mut)</code>
with the exception of `MutBumpVecRev` and `alloc_iter_mut_rev`.
`MutBumpVecRev` can be grown and shrunk in place iff bumping downwards.

Bumping downwards shaves off a few non-branch instructions per allocation.

## Minimum alignment?
The minimum alignment is controlled by the generic parameter `const MIN_ALIGN: usize`. By default, `MIN_ALIGN` is `1`.

For example changing the minimum alignment to `4` makes it so allocations with the alignment of `4` don't need to align the bump pointer anymore.
This will penalize allocations whose sizes are not a multiple of `4` as their size now needs to be rounded up the next multiple of `4`.

The overhead of aligning and rounding up is 1 (`UP = false`) or 2 (`UP = true`) non-branch instructions on x86-64.

## `GUARANTEED_ALLOCATED` parameter?
If `GUARANTEED_ALLOCATED` is `true` then the bump allocator is guaranteed to have at least one allocated chunk.
This is usually the case unless it was created with `Bump::unallocated`.

You need a guaranteed allocated `Bump(Scope)` to create scopes via `scoped` and `scope_guard`.
You can make a `Bump(Scope)` guaranteed allocated using
<code>[guaranteed_allocated](Bump::guaranteed_allocated)([_ref](Bump::guaranteed_allocated_ref)/[_mut](Bump::guaranteed_allocated_mut))</code>.

The point of this is so `Bump`s can be created without allocating memory and even `const` constructed since rust version 1.83.
At the same time `Bump`s that have already allocated a chunk don't suffer runtime checks for entering scopes and creating checkpoints.

<!-- cargo-rdme end -->

## Testing

Running `cargo test` requires a nightly compiler. 

## License

Licensed under either of:

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

at your option.

---

This project includes code adapted from the Rust standard library 
(https://github.com/rust-lang/rust),  
Copyright © The Rust Project Developers.
Such code is also licensed under MIT OR Apache-2.0.

### Your contributions

Unless you explicitly state otherwise,
any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, 
shall be dual licensed as above,
without any additional terms or conditions.
