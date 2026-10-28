# bump-scope

[![Crates.io](https://img.shields.io/crates/v/bump-scope.svg)](https://crates.io/crates/bump-scope)
[![Documentation](https://img.shields.io/docsrs/bump-scope)](https://docs.rs/bump-scope)
[![Changelog](https://img.shields.io/badge/Changelog-gray)](CHANGELOG.md)
[![Rust](https://img.shields.io/crates/msrv/bump-scope)](#)
[![License](https://img.shields.io/crates/l/bump_scope)](#license)
[![Build Status](https://github.com/bluurryy/bump-scope/workflows/CI/badge.svg)](https://github.com/bluurryy/bump-scope/actions/workflows/ci.yml)

> [!WARNING]  
> This is the **in-development version 3.0.0**, which contains breaking changes and is not yet released. 
> For the latest release see [the `v2` branch](https://github.com/bluurryy/bump-scope/tree/v2).

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
- [Motivation and History](#motivation-and-history)
- [Contributing](#contributing)

<!-- crate documentation rest start -->
## What is bump allocation?
A bump allocator owns a big chunk of memory. It has a pointer that starts at one end of that chunk.
When an allocation is made that pointer gets aligned and bumped towards the other end of the chunk.
When its chunk is full, this allocator allocates another chunk with twice the size.

This makes allocations very fast. The drawback is that you can't reclaim memory like you do with a more general allocator.
Memory for the most recent allocation *can* be reclaimed. You can also use [scopes, checkpoints](#scopes-and-checkpoints) and `reset` to reclaim memory.

A bump allocator is great for *phase-oriented allocations* where you allocate objects in a loop and free them at the end of every iteration.
```rust
use bump_scope::Bump;
let mut bump: Bump = Bump::new();

loop {
    // use bump ...
    bump.reset();
}
```
The fact that the bump allocator allocates ever larger chunks and `reset` only keeps around the largest one means that after a few iterations, every bump allocation
will be done on the same chunk and no more chunks need to be allocated.

The introduction of scopes makes this bump allocator also great for temporary allocations and stack-like usage.

## Comparison to [`bumpalo`](https://docs.rs/bumpalo)

Bumpalo is a popular crate for bump allocation.
This crate was inspired by bumpalo and [Always Bump Downwards](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html)
(but ignores the title).

Unlike `bumpalo`, this crate...
- Supports [scopes and checkpoints](#scopes-and-checkpoints).
- Drop is always called for allocated values unless explicitly [leaked](https://doc.rust-lang.org/alloc/boxed/struct.Box.html#method.leak) or [forgotten](https://doc.rust-lang.org/core/mem/fn.forget.html).
  - `alloc*` methods return a `BumpAllocation` which owns and drops `T`. Types that don't need dropping can be turned into references with `into_ref` and `into_mut`.
- You can allocate a slice from any `Iterator` with `alloc_iter`.
- `Bump`'s base allocator is generic.
- Won't try to allocate a smaller chunk if allocation failed.
- No built-in allocation limit. You can provide an allocator that enforces an allocation limit (see `examples/limit_memory_usage.rs`).
- Allocations are a tiny bit more optimized. See [./crates/callgrind-benches][benches].
- You can choose the bump direction. Bumps upwards by default.

## Allocator Methods

The bump allocator provides many methods to conveniently allocate values, strings, and slices.
Have a look at the documentation of `Bump` for a method overview.

## Scopes and Checkpoints

You can create scopes to make allocations that live only for a part of its parent scope.
Entering and exiting scopes is virtually free. Allocating within a scope has no overhead.

You can create a new scope either with a `scoped` closure or with a `scope_guard`:
```rust
use bump_scope::{Bump, BumpAllocator};
let mut bump: Bump = Bump::new();

// you can use a closure
bump.scoped(|mut bump| {
    let hello = bump.alloc_str("hello").into_ref();
    assert_eq!(bump.stats().allocated(), 5);

    bump.scoped(|bump| {
        let world = bump.alloc_str("world").into_ref();

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

    let hello = bump.alloc_str("hello").into_ref();
    assert_eq!(bump.stats().allocated(), 5);

    {
        let mut guard = bump.scope_guard();
        let bump = guard.scope();

        let world = bump.alloc_str("world").into_ref();

        println!("{hello} and {world} are both live");
        assert_eq!(bump.stats().allocated(), 10);
    }

    println!("{hello} is still live");
    assert_eq!(bump.stats().allocated(), 5);
}

assert_eq!(bump.stats().allocated(), 0);
```
You can also use the unsafe `checkpoint` api
to reset the bump pointer to a previous position.
```rust
use bump_scope::{Bump, BumpAllocator};

let bump: Bump = Bump::new();
let checkpoint = bump.checkpoint();

{
    let hello = bump.alloc_str("hello");
    assert_eq!(bump.stats().allocated(), 5);
}

unsafe { bump.reset_to(checkpoint); }
assert_eq!(bump.stats().allocated(), 0);
```
When using a `Bump(Scope)` as an allocator for collections you will find that you can no longer
call `scoped` or `scope_guard` because those functions require `&mut self` which does not allow
any outstanding references to the allocator.

As a workaround you can use the [`claim`] method on a `&Bump(Scope)` to return a `BumpClaimGuard` which
mutably dereferences to a `BumpScope`, allowing you to call `.scoped()` and `.scope_guard()`.

A `bump.claim()` call replaces the allocator referred to by `bump` with a dummy allocator while the returned `BumpClaimGuard`
is live. This dummy allocator errors when trying to allocate or grow memory and reports an empty bump allocator from the `stats` api.

This makes it possible to enter scopes while a there are still outstanding
references to that bump allocator:
```rust
use bump_scope::{Bump, BumpScope};
let bump: Bump = Bump::new();

let mut vec: Vec<u8, &Bump> = Vec::new_in(&bump);

bump.claim().scoped(|bump_scope| {
    // you can allocate in the scope as usual
    let mut vec2: Vec<u8, &BumpScope> = Vec::new_in(bump_scope);
    vec2.reserve(456);

    // allocating on the `bump` outside the scope will fail
    assert!(vec.try_reserve(123).is_err());
});

// now allocating on `bump` is possible again
vec.reserve(123);
```

## Parallel Allocation
`Bump` is `!Sync` which means it can't be shared between threads.

To bump allocate in parallel you can use a `BumpPool`.

## Allocator API
`Bump` and `BumpScope` implement `bump-scope`'s own [`Allocator`] trait and with the
respective [feature flags](#feature-flags) also implement `allocator_api2` version `0.2`,
`0.3`, `0.4` and nightly's `Allocator` trait.

This allows you to bump allocate collections.

A bump allocator can grow, shrink and deallocate the most recent allocation.
When bumping upwards it can even do so in place.
Growing allocations other than the most recent one will require a new allocation and the old memory block becomes wasted space.
Shrinking or deallocating allocations other than the most recent one does nothing, which means wasted space.

A bump allocator does not require `deallocate` or `shrink` to free memory.
After all, memory will be reclaimed when exiting a scope, calling `reset` or dropping the `Bump`.
You can set the `deallocates` and `shrinks` config parameters to false to make deallocating and shrinking a no-op.

## Feature Flags
<!-- feature documentation start -->
- **`std`** *(enabled by default)* — Adds `BumpPool` and implementations of `std::io` traits.
- **`panic-on-alloc`** *(enabled by default)* — Adds functions and traits that will panic when allocations fail.
  Without this feature, allocation failures cannot cause panics, and only
  `try_`-prefixed allocation methods will be available.
- **`bytemuck`** — Adds `bytemuck::*` extension traits for
  <code>alloc_zeroed(_slice)</code>,
  `box_zero_init` and
  `box_zero_init_slice`.
- **`zerocopy-08`** — Adds `zerocopy_08::*` extension traits for
  <code>alloc_zeroed(_slice)</code>,
  `box_zero_init` and
  `box_zero_init_slice`.
- **`nightly`** — Enables nightly features.
  TODO: enumerate...

  Breaking changes to these features might be introduced in minor releases to keep up with changes in the nightly channel.
<!-- feature documentation end -->


[benches]: https://github.com/bluurryy/bump-scope/tree/main/crates/callgrind-benches
[`Allocator`]: https://doc.rust-lang.org/core/alloc/trait.Allocator.html
<!-- crate documentation rest end -->

## Motivation and History

I did not find any crate that was basically just "bumpalo but with scopes /checkpoints" so I've decided to give it a shot myself.

I wasn't sure about bumping downwards because it loses the realloc fast path ([Always Bump Downwards](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html)). I was also curious about the impact of minimum alignment so I made `UP` and `MIN_ALIGN` generic parameters.

To be able to allocate slices from arbitrary iterators or `fmt::Arguments` I had to implement my own `Vec` and `String`. (The `Vec` from `allocator_api2` would have worked, but the generated code wasn't great.) Comparing `alloc_iter` for an upwards and downwards bumping allocator is not really fair because of the different realloc behavior. So I implemented `alloc_iter_rev` and a `VecRev` that would push elements to the start instead of the end. A `VecRev` can be grown and shrunk in place when downwards bumping just as a `Vec` can be grown and shrunk in place when upwards allocating.

I've found that bumping downwards and having a minimum alignment makes little difference in terms of performance. If you make use of `alloc_iter`, `alloc_fmt` or growable collections then the more favorable realloc behavior for upwards bumping will likely be more important than the few instructions you save from bumping downwards.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).

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
