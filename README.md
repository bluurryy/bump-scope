# bump-scope

[![Crates.io](https://img.shields.io/crates/v/bump-scope.svg)](https://crates.io/crates/bump-scope)
[![Documentation](https://img.shields.io/docsrs/bump-scope)](https://docs.rs/bump-scope)
[![Changelog](https://img.shields.io/badge/Changelog-.md-8c3e8c)](CHANGELOG.md)
[![MSRV](https://img.shields.io/crates/msrv/bump-scope)](#)
[![License](https://img.shields.io/crates/l/bump_scope)](#license)
[![handmade](https://img.shields.io/badge/handmade-4b0)](#)

> [!WARNING]  
> This is the **in-development version 3.0.0**, which contains breaking changes and is not yet released. 
> For the latest release see [the `v2` branch](https://github.com/bluurryy/bump-scope/tree/v2).

<!-- crate documentation intro start -->
A fast bump allocator that supports allocation scopes / checkpoints. Aka an arena for values of arbitrary types.
<!-- crate documentation intro end -->

## Table of Contents
- [What is bump allocation?](#what-is-bump-allocation)
- [Overview](#overview)
- [Scopes and Checkpoints](#scopes-and-checkpoints)
- [Feature Flags](#feature-flags)
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

## Overview

- Implements `core`'s `Allocator` and takes an `Allocator`.
- Supports [scopes and checkpoints](#scopes-and-checkpoints).
- Has helper methods in `BumpAllocator` for safely allocating values, slices, strings, collecting iterators, formatting, etc.
- Very fast, see [./benches][benches].
- Provides configuration options, to let you optimize the bump allocator for your application.
- Supports parallel bump allocation with a `BumpPool`.

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
    assert_eq!(bump.allocated(), 5);

    bump.scoped(|bump| {
        let world = bump.alloc_str("world").into_ref();

        println!("{hello} and {world} are both live");
        assert_eq!(bump.allocated(), 10);
    });

    println!("{hello} is still live");
    assert_eq!(bump.allocated(), 5);
});

assert_eq!(bump.allocated(), 0);

// or you can use scope guards
{
    let mut guard = bump.scope_guard();
    let mut bump = guard.scope();

    let hello = bump.alloc_str("hello").into_ref();
    assert_eq!(bump.allocated(), 5);

    {
        let mut guard = bump.scope_guard();
        let bump = guard.scope();

        let world = bump.alloc_str("world").into_ref();

        println!("{hello} and {world} are both live");
        assert_eq!(bump.allocated(), 10);
    }

    println!("{hello} is still live");
    assert_eq!(bump.allocated(), 5);
}

assert_eq!(bump.allocated(), 0);
```
You can also use the unsafe `checkpoint` api
to reset the bump pointer to a previous position.
```rust
use bump_scope::{Bump, BumpAllocator};

let bump: Bump = Bump::new();
let checkpoint = bump.checkpoint();

{
    let hello = bump.alloc_str("hello");
    assert_eq!(bump.allocated(), 5);
}

unsafe { bump.reset_to(checkpoint); }
assert_eq!(bump.allocated(), 0);
```

## Collections and Scopes

When using a `Bump(Scope)` as an allocator for a collection you will find that you can no longer
call `scoped` or `scope_guard` since those functions require `&mut self` which does not allow
any outstanding references to the allocator.

```rust
let vec: Vec<u8, &Bump> = Vec::new_in(&bump);

bump.scoped(|bump_scope| {
    ...
});
```

As a workaround you can use the [`claim`] method on `&Bump(Scope)` to return a `BumpClaimGuard` which
mutably dereferences to a `BumpScope`, enabling you to call `.scoped()` and `.scope_guard()`.

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

## Feature Flags
<!-- feature documentation start -->
- **`std`** *(enabled by default)* — Adds `BumpPool` and implementations of `std::io` traits.
- **`panic-on-alloc`** *(enabled by default)* — Adds functions and traits that will panic when allocations fail.
  Without this feature, allocation failures cannot cause panics, and only
  `try_`-prefixed allocation methods will be available.
- **`bytemuck`** — Adds `bytemuck` module with
  <code>BumpAllocatorExt::alloc_zeroed(_slice)</code>
  and <code>box_zero_init(_slice)</code>.
- **`zerocopy-08`** — Adds `zerocopy_08` module with
  <code>BumpAllocatorExt::alloc_zeroed(_slice)</code>
  and <code>box_zero_init(_slice)</code>.
- **`nightly`** — Enables nightly features. ***This feature is exempt from semver.***
  - Adds `Take(OwnedSlice)` implementations for `vec::IntoIter` and `vec::Drain` of any allocator.
  - Uses std's `vec::from_elem_in` instead of our polyfill for the `vec!` macro
  - Implements `AllocatorNightly` for our bump allocators
  - Adds `BumpAllocator::alloc_clone`
  - Promises symbolic alignment to miri for bump allocations
<!-- feature documentation end -->

## MSRV policy
Increasing the minimum supported rust version is not considered a breaking change.
It can happen in any release.
The MSRV is currently **<!--msrv start-->1.100.0<!--msrv end-->**.


[benches]: https://github.com/bluurryy/bump-scope/tree/main/benches
[`Allocator`]: https://doc.rust-lang.org/core/alloc/trait.Allocator.html
<!-- crate documentation rest end -->

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
