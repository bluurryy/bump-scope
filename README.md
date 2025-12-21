# bump-scope

[![Crates.io](https://img.shields.io/crates/v/bump-scope.svg)](https://crates.io/crates/bump-scope)
[![Documentation](https://img.shields.io/docsrs/bump-scope)](https://docs.rs/bump-scope)
[![Changelog](https://img.shields.io/badge/Changelog-gray)](CHANGELOG.md)
[![Rust](https://img.shields.io/crates/msrv/bump-scope)](#)
[![License](https://img.shields.io/crates/l/bump_scope)](#license)
[![Build Status](https://github.com/bluurryy/bump-scope/workflows/CI/badge.svg)](https://github.com/bluurryy/bump-scope/actions/workflows/ci.yml)

<!-- crate documentation intro start -->
A fast bump allocator that supports allocation scopes / checkpoints. Aka an arena for values of arbitrary types.
<!-- crate documentation intro end -->

## Table of Contents
- [What is bump allocation?](#what-is-bump-allocation)
- [Comparison to `bumpalo`](#comparison-to-bumpalo)
- [Allocator Methods](#allocator-methods)
- [Scopes and Checkpoints](#scopes-and-checkpoints)
- [Collections](#collections)
- [Parallel Allocation](#parallel-allocation)
- [Allocator API](#allocator-api)
- [Feature Flags](#feature-flags)
- [Bumping upwards or downwards?](#bumping-upwards-or-downwards)
- [What is minimum alignment?](#what-is-minimum-alignment)
- [What does *guaranteed allocated* mean?](#what-does-guaranteed-allocated-mean)
- [Motivation and History](#motivation-and-history)

<!-- crate documentation rest start -->
## What is bump allocation?
A bump allocator owns a big chunk of memory. It has a pointer that starts at one end of that chunk.
When an allocation is made that pointer gets aligned and bumped towards the other end of the chunk.
When its chunk is full, this allocator allocates another chunk with twice the size.

This makes allocations very fast. The drawback is that you can't reclaim memory like you do with a more general allocator.
Memory for the most recent allocation *can* be reclaimed. You can also use [scopes, checkpoints](#scopes-and-checkpoints) and [`reset`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.reset) to reclaim memory.

A bump allocator is great for *phase-oriented allocations* where you allocate objects in a loop and free them at the end of every iteration.
```rust
use bump_scope::Bump;
let mut bump: Bump = Bump::new();

loop {
    // use bump ...
    bump.reset();
}
```
The fact that the bump allocator allocates ever larger chunks and [`reset`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.reset) only keeps around the largest one means that after a few iterations, every bump allocation
will be done on the same chunk and no more chunks need to be allocated.

The introduction of scopes makes this bump allocator also great for temporary allocations and stack-like usage.

## Comparison to [`bumpalo`](https://docs.rs/bumpalo)

Bumpalo is a popular crate for bump allocation.
This crate was inspired by bumpalo and [Always Bump Downwards](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html)
(but ignores the title).

Unlike `bumpalo`, this crate...
- Supports [scopes and checkpoints](#scopes-and-checkpoints).
- Drop is always called for allocated values unless explicitly [leaked](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpBox.html#method.leak) or [forgotten](https://doc.rust-lang.org/core/mem/fn.forget.html).
  - `alloc*` methods return a [`BumpBox<T>`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpBox.html) which owns and drops `T`. Types that don't need dropping can be turned into references with [`into_ref`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpBox.html#method.into_ref) and [`into_mut`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpBox.html#method.into_mut).
- You can allocate a slice from any `Iterator` with [`alloc_iter`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.alloc_iter).
- `Bump`'s base allocator is generic.
- Won't try to allocate a smaller chunk if allocation failed.
- No built-in allocation limit. You can provide an allocator that enforces an allocation limit (see `tests/limit_memory_usage.rs`).
- Allocations are a tiny bit more optimized. See [./crates/callgrind-benches][benches].
- [You can choose the bump direction.](#bumping-upwards-or-downwards) Bumps upwards by default.

## Allocator Methods

The bump allocator provides many methods to conveniently allocate values, strings, and slices.
Have a look at the documentation of [`Bump`] for a method overview.

## Scopes and Checkpoints

You can create scopes to make allocations that live only for a part of its parent scope.
Entering and exiting scopes is virtually free. Allocating within a scope has no overhead.

You can create a new scope either with a [`scoped`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.scoped) closure or with a [`scope_guard`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.scope_guard):
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
You can also use the unsafe [`checkpoint`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.checkpoint) api
to reset the bump pointer to a previous position.
```rust
let bump: Bump = Bump::new();
let checkpoint = bump.checkpoint();

{
    let hello = bump.alloc_str("hello");
    assert_eq!(bump.stats().allocated(), 5);
}

unsafe { bump.reset_to(checkpoint); }
assert_eq!(bump.stats().allocated(), 0);
```

## Collections
`bump-scope` provides bump allocated versions of `Vec` and `String` called [`BumpVec`] and [`BumpString`].
They are also available in the following variants:
- [`Fixed*`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.FixedBumpVec.html) for fixed capacity collections
- [`Mut*`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.MutBumpVec.html) for collections optimized for a mutable bump allocator

##### API changes
The collections are designed to have the same api as their std counterparts with these exceptions:
- [`split_off`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpVec.html#method.split_off) —  splits the collection in place without allocation; the parameter is a range instead of a single index
- [`retain`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpVec.html#method.retain) —  takes a closure with a `&mut T` parameter like `Vec::retain_mut`

##### New features
- [`append`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpVec.html#method.append) —  allows appending all kinds of owned slice types like `[T; N]`, `Box<[T]>`, `Vec<T>`, `vec::Drain<T>` etc.
- [`map`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpVec.html#method.map) —  maps the elements, potentially reusing the existing allocation
- [`map_in_place`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpVec.html#method.map_in_place) —  maps the elements without allocation, failing to compile if not possible
- conversions between the regular collections, their `Fixed*` variants and `BumpBox<[T]>` / `BumpBox<str>`

## Parallel Allocation
[`Bump`] is `!Sync` which means it can't be shared between threads.

To bump allocate in parallel you can use a [`BumpPool`].

## Allocator API
`Bump` and `BumpScope` implement `bump-scope`'s own [`Allocator`] trait and with the
respective [feature flags](#feature-flags) also implement `allocator_api2@0.2`, `allocator_api2@0.3` and nightly's `Allocator` trait.
All of these traits mirror the nightly `Allocator` trait at the time of writing.

This allows you to [bump allocate collections](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#collections).

A bump allocator can grow, shrink and deallocate the most recent allocation.
When bumping upwards it can even do so in place.
Growing allocations other than the most recent one will require a new allocation and the old memory block becomes wasted space.
Shrinking or deallocating allocations other than the most recent one does nothing, which means wasted space.

A bump allocator does not require `deallocate` or `shrink` to free memory.
After all, memory will be reclaimed when exiting a scope, calling `reset` or dropping the `Bump`.
You can set the `DEALLOCATES` const parameter to `false` to make shrinking and deallocating a no-op.
There are also the [`WithoutDealloc`] and [`WithoutShrink`] wrappers.
```rust
use bump_scope::{Bump, WithoutDealloc};
use allocator_api2_04::boxed::Box;

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
<!-- feature documentation start -->
- **`std`** *(enabled by default)* — Adds `BumpPool` and implementations of `std::io` traits.
- **`alloc`** *(enabled by default)* — Adds `Global` as the default base allocator and some interactions with `alloc` collections.
- **`panic-on-alloc`** *(enabled by default)* — Adds functions and traits that will panic when the allocation fails.
  Without this feature, allocation failures cannot cause panics, and only
  `try_`-prefixed allocation methods will be available.
- **`serde`** — Adds `Serialize` implementations for `BumpBox`, strings and vectors, and `DeserializeSeed` for strings and vectors.
- **`bytemuck`** — Adds `bytemuck::*` extension traits for
  <code>[alloc_zeroed](https://docs.rs/bump-scope/1.5.1/bump_scope/bytemuck/trait.BumpExt.html#tymethod.alloc_zeroed)([_slice](https://docs.rs/bump-scope/1.5.1/bump_scope/bytemuck/trait.BumpExt.html#tymethod.alloc_zeroed_slice))</code>,
  [`init_zeroed`](https://docs.rs/bump-scope/1.5.1/bump_scope/bytemuck/trait.InitZeroed.html#tymethod.init_zeroed),
  [`extend_zeroed`](https://docs.rs/bump-scope/1.5.1/bump_scope/bytemuck/trait.VecExt.html#tymethod.extend_zeroed) and
  [`resize_zeroed`](https://docs.rs/bump-scope/1.5.1/bump_scope/bytemuck/trait.VecExt.html#tymethod.resize_zeroed).
- **`zerocopy-08`** — Adds `zerocopy_08::*` extension traits for
  <code>[alloc_zeroed](https://docs.rs/bump-scope/1.5.1/bump_scope/zerocopy_08/trait.BumpExt.html#tymethod.alloc_zeroed)([_slice](https://docs.rs/bump-scope/1.5.1/bump_scope/zerocopy_08/trait.BumpExt.html#tymethod.alloc_zeroed_slice))</code>,
  [`init_zeroed`](https://docs.rs/bump-scope/1.5.1/bump_scope/zerocopy_08/trait.InitZeroed.html#tymethod.init_zeroed),
  [`extend_zeroed`](https://docs.rs/bump-scope/1.5.1/bump_scope/zerocopy_08/trait.VecExt.html#tymethod.extend_zeroed) and
  [`resize_zeroed`](https://docs.rs/bump-scope/1.5.1/bump_scope/zerocopy_08/trait.VecExt.html#tymethod.resize_zeroed).
- **`allocator-api2-02`** — Makes `Bump(Scope)` implement `allocator_api2` version `0.2`'s `Allocator` and
  makes it possible to use an `allocator_api2::alloc::Allocator` as a base allocator via
  [`AllocatorApi2V02Compat`](https://docs.rs/bump-scope/1.5.1/bump_scope/alloc/compat/struct.AllocatorApi2V02Compat.html).
- **`allocator-api2-03`** — Makes `Bump(Scope)` implement `allocator_api2` version `0.3`'s `Allocator` and
  makes it possible to use an `allocator_api2::alloc::Allocator` as a base allocator via
  [`AllocatorApi2V03Compat`](https://docs.rs/bump-scope/1.5.1/bump_scope/alloc/compat/struct.AllocatorApi2V03Compat.html).
- **`allocator-api2-04`** — Makes `Bump(Scope)` implement `allocator_api2` version `0.4`'s `Allocator` and
  makes it possible to use an `allocator_api2::alloc::Allocator` as a base allocator via
  [`AllocatorApi2V04Compat`](https://docs.rs/bump-scope/1.5.1/bump_scope/alloc/compat/struct.AllocatorApi2V04Compat.html).

#### Nightly features
These nightly features are not subject to the same semver guarantees as the rest of the library.
Breaking changes to these features might be introduced in minor releases to keep up with changes in the nightly channel.

- **`nightly`** — Enables all other nightly feature flags.
- **`nightly-allocator-api`** — Makes `Bump(Scope)` implement `alloc`'s `Allocator` and
  allows using an `alloc::alloc::Allocator` as a base allocator via
  [`AllocatorNightlyCompat`](https://docs.rs/bump-scope/1.5.1/bump_scope/alloc/compat/struct.AllocatorNightlyCompat.html).

  This will also enable `allocator-api2` version `0.2`'s `nightly` feature.
- **`nightly-coerce-unsized`** — Makes `BumpBox<T>` implement [`CoerceUnsized`](https://doc.rust-lang.org/core/ops/unsize/trait.CoerceUnsized.html).
  With this `BumpBox<[i32;3]>` coerces to `BumpBox<[i32]>`, `BumpBox<dyn Debug>` and so on.
  You can unsize a `BumpBox` in stable without this feature using [`unsize_bump_box`].
- **`nightly-exact-size-is-empty`** — Implements `is_empty` manually for some iterators.
- **`nightly-trusted-len`** — Implements `TrustedLen` for some iterators.
- **`nightly-fn-traits`** — Implements `Fn*` traits for `BumpBox<T>`. Makes `BumpBox<T: FnOnce + ?Sized>` callable. Requires alloc crate.
- **`nightly-tests`** — Enables some tests that require a nightly compiler.
- **`nightly-dropck-eyepatch`** — Adds `#[may_dangle]` attribute to box and vector types' drop implementation.
  This makes it so references don't have to strictly outlive the container.
  (Just like with std's `Box` and `Vec`.)
- **`nightly-clone-to-uninit`** — Adds [`alloc_clone`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.alloc_clone) method to `Bump(Scope)`.
<!-- feature documentation end -->

## Bumping upwards or downwards?
Bump direction is controlled by the generic parameter `const UP: bool`. By default, `UP` is `true`, so the allocator bumps upwards.

Bumping upwards has the advantage that the most recent allocation can be grown and shrunk in place.
This benefits collections as well as <code>[alloc_iter](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.alloc_iter)([_mut](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.alloc_iter_mut))</code> and <code>[alloc_fmt](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.alloc_fmt)([_mut](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.alloc_fmt_mut))</code>
with the exception of [`MutBumpVecRev`] and [`alloc_iter_mut_rev`](https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.alloc_iter_mut_rev) which
can be grown and shrunk in place if and only if bumping downwards.

Bumping downwards can be done in less instructions.

For the performance impact see [./crates/callgrind-benches][benches].

## What is minimum alignment?
Minimum alignment is the alignment the bump pointer maintains when doing allocations.

When allocating a type in a bump allocator with a sufficient minimum alignment,
the bump pointer will not have to be aligned for the allocation but the allocation size
will need to be rounded up to the next multiple of the minimum alignment.

The minimum alignment is controlled by the generic parameter `const MIN_ALIGN: usize`. By default, `MIN_ALIGN` is `1`.

For the performance impact see [./crates/callgrind-benches][benches].

## What does *guaranteed allocated* mean?

A *guaranteed allocated* bump allocator will own at least one chunk that it has allocated
from its base allocator.

The constructors [`new`], [`with_size`], [`with_capacity`] and their variants always allocate
one chunk from the base allocator.

The exception is the [`unallocated`] constructor which creates a `Bump` without allocating any
chunks. Such a `Bump` will have the `GUARANTEED_ALLOCATED` generic parameter of `false`
which will make the [`scoped`], [`scoped_aligned`], [`aligned`] and [`scope_guard`] methods unavailable.

You can turn any non-`GUARANTEED_ALLOCATED` bump allocator into a guaranteed allocated one using
[`as_guaranteed_allocated`], [`as_mut_guaranteed_allocated`] or [`into_guaranteed_allocated`].

The point of this is so `Bump`s can be `const` constructed and constructed without allocating.
At the same time `Bump`s that have already allocated a chunk don't suffer additional runtime checks.


[`unsize_bump_box`]: https://docs.rs/bump-scope/1.5.1/bump_scope/macro.unsize_bump_box.html
[`WithoutShrink`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.WithoutShrink.html
[`WithoutDealloc`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.WithoutDealloc.html
[`MutBumpVecRev`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.MutBumpVecRev.html
[`Bump`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html
[`BumpVec`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpVec.html
[`BumpString`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpString.html
[`BumpPool`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.BumpPool.html
[`Allocator`]: https://docs.rs/bump-scope/1.5.1/bump_scope/alloc/trait.Allocator.html
[CHANGELOG]: https://docs.rs/bump-scope/1.5.1/bump_scope/CHANGELOG/index.html
[`into_guaranteed_allocated`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.into_guaranteed_allocated
[`as_mut_guaranteed_allocated`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.as_mut_guaranteed_allocated
[`as_guaranteed_allocated`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.as_guaranteed_allocated
[`scope_guard`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.scope_guard
[`aligned`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.aligned
[`scoped_aligned`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.scoped_aligned
[`scoped`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.scoped
[`unallocated`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.unallocated
[`with_capacity`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.with_capacity
[`with_size`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.with_size
[`new`]: https://docs.rs/bump-scope/1.5.1/bump_scope/struct.Bump.html#method.new
[benches]: https://github.com/bluurryy/bump-scope/tree/main/crates/callgrind-benches
<!-- crate documentation rest end -->

## Motivation and History

I was using bumpalo when I wanted to do some temporary allocation with the same bump allocator that I was also using to allocate longer lived objects. I did not find any crate that was basically just "bumpalo but with checkpoints" so I've decided to give it a shot myself.

It surprised me to learn that bumpalo bumps downwards. I wasn't sure about bumping downwards because it loses the realloc fast path ([Always Bump Downwards](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html)). I was also curious about the impact of minimum alignment so I made `UP` and `MIN_ALIGN` generic parameters.

To be able to allocate slices from arbitrary iterators or `fmt::Arguments` I had to implement my own `Vec` and `String`. (The `Vec` from `allocator_api2` would have worked, but the generated code wasn't great.) Comparing `alloc_iter` for an upwards and downwards bumping allocator is not really fair because of the different realloc behavior. So I implemented `alloc_iter_rev` and a `VecRev` that would push elements to the start instead of the end. A `VecRev` can be grown and shrunk in place when downwards bumping just as a `Vec` can be grown and shrunk in place when upwards allocating.

The conclusion I've come to by writing this library is that bumping downwards and having a minimum alignment makes very little difference. If you make any use of `alloc_iter`, `alloc_fmt` or growable collections then the more favorable realloc behavior of upwards bumping will likely save you more than the few instructions from bumping downwards.

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
