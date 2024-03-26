# bump-scope

[![Crates.io](https://img.shields.io/crates/v/hashbrown.svg)](https://crates.io/crates/bump-scope)
[![Documentation](https://docs.rs/hashbrown/badge.svg)](https://docs.rs/bump-scope)
![Rust](https://img.shields.io/badge/rust-1.64.0%2B-blue.svg)
![](https://img.shields.io/crates/l/rodeo)

[//]: # (START_OF_CRATE_DOCS)

A fast bump allocator that supports allocation scopes. Aka an arena for values of arbitrary types.

## What is bump allocation?
A bump allocator owns a big chunk of memory. It has a pointer that starts at one end of that chunk.
When an allocation is made that pointer gets aligned and bumped towards the other end of the chunk by the allocation's size.
When its chunk is full, it allocates another chunk with twice the size.

This makes allocations very fast. The drawback is that you can't reclaim memory like you do with a more general allocator.
Memory for the most recent allocation *can* be reclaimed. You can also use scopes and `reset` to reclaim memory.

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
will be done on the same chunk.

The introduction of scopes makes this bump allocator also great for temporary allocations and stack-like usage.

## Comparison to [`bumpalo`](https://docs.rs/bumpalo)

Bumpalo is a popular crate for bump allocation. This crate was inspired by bumpalo and [Always Bump Downwards](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html).

Unlike `bumpalo`, this crate...
- Supports [scopes and checkpoints](#scopes).
- Drop is always called for allocated values unless explicitly leaked or forgotten.
  - `alloc*` methods return a `BumpBox<T>` which owns and drops `T`. Types that don't need dropping can be turned into references with `into_ref` and `into_mut`.
- You can efficiently allocate items from *any* `Iterator` with `alloc_iter_mut`(`_rev`).
- Every method that panics on allocation failure has a fallible `try_*` counterpart.
- `Bump`'s base allocator is generic.
- `Bump` needs to allocate on construction.
- `Bump` and `BumpScope` have the same repr as `NonNull<u8>`. (vs 3x pointer sized)
- Won't try to allocate a smaller chunk if allocation failed.
- No built-in allocation limit. You can provide an allocator that enforces an allocation limit (see `tests/limit_memory_usage.rs`).
- Allocations are a bit more optimized. (see `crates/inspect-asm/out/x86-64`)
- [You can choose the bump direction.](#bumping-upwards-or-downwards) Bumps upwards by default.
- [You can choose the minimum alignment.](#minimum-alignment)

## Scopes
You can create scopes to make allocations that live only for a part of its parent scope.
Creating and exiting scopes is virtually free. Allocating within a scope has no overhead.

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
In this example we are intentionally using the name `bump` for every scope.
The parent `bump` is not usable anyway because it is mutably borrowed. By shadowing the `bump` name you can't accidentally access it
or move it into the closure, which would cause a compile error.

You can also use the unsafe `checkpoint` api to reset the bump pointer to a previous location.
```rust
let checkpoint = bump.checkpoint();

{
    let hello = bump.alloc_str("hello");
    assert_eq!(bump.stats().allocated(), 5);
}

unsafe { bump.reset_to(checkpoint); }
assert_eq!(bump.stats().allocated(), 0);
```

## Allocator API
`Bump` and `BumpScope` implement `allocator_api2::alloc::Allocator`.
With this you can bump allocate `allocator_api2::boxed::Box`, `allocator_api2::vec::Vec` and collections
from other crates that support it like [`hashbrown::HashMap`](https://docs.rs/hashbrown/latest/hashbrown/struct.HashMap.html).

A bump allocator can grow, shrink and deallocate the most recent allocation.
When bumping upwards it can even do so in place.
Growing other allocations will require a new allocation and the old memory block becomes wasted space.
Shrinking or deallocating other allocations does nothing which means wasted space.

A bump allocator does not *require* `deallocate` or `shrink` to free memory.
After all, memory will be reclaimed when exiting a scope or calling `reset`.
You can wrap a bump allocator in a type that makes `deallocate` and `shrink` a no-op using `without_dealloc` and `without_shrink`.
```rust
use bump_scope::Bump;
use allocator_api2::boxed::Box;
let bump: Bump = Bump::new();

let boxed = Box::new_in(5, &bump);
assert_eq!(bump.stats().allocated(), 4);
drop(boxed);
assert_eq!(bump.stats().allocated(), 0);

let boxed = Box::new_in(5, bump.without_dealloc());
assert_eq!(bump.stats().allocated(), 4);
drop(boxed);
assert_eq!(bump.stats().allocated(), 4);
```

## Feature Flags
This crate supports `no_std`, unless the `std` feature is enabled.

- `std` *(default)*:

  Adds implementations of `std::io` traits for `BumpBox` and `(Fixed)BumpVec`. Activates `alloc` feature.

<p></p>

- `alloc` *(default)*:

  Adds implementations interacting with `String` and `Cow<str>`.

<p></p>

- `nightly-allocator-api` *(requires nightly)*:

  Enables `allocator-api2`'s `nightly` feature which makes it reexport the nightly allocator api instead of its own implementation.
  With this you can bump allocate collections from the standard library.

<p></p>

- `nightly-coerce-unsized` *(requires nightly)*:
  
  Makes `BumpBox<T>` implement `CoerceUnsized`.
  With this `BumpBox<[i32;3]>` coerces to `BumpBox<[i32]>`, `BumpBox<dyn Debug>` and so on.

## Bumping upwards or downwards?
Bump direction is controlled by the generic parameter `const UP: bool`. By default, `UP` is `true`, so the allocator bumps upwards.

- Bumping upwards...
  - has the advantage that the most recent allocation can be grown and shrunk in place.
  - makes `alloc_iter(_mut)` and `alloc_fmt(_mut)` faster.
- Bumping downwards...
  - uses slightly fewer instructions per allocation.
  - makes `alloc_iter_mut_rev` faster.

## Minimum alignment?
The minimum alignment is controlled by the generic parameter `const MIN_ALIGN: usize`. By default, `MIN_ALIGN` is `1`.

Changing the minimum alignment to e.g. `4` makes it so allocations with the alignment of `4` don't need to align the bump pointer anymore.
This will penalize allocations of a smaller alignment as their size now needs to be rounded up the next multiple of `4`.

This amounts to about 1 or 2 instructions per allocation.

[//]: # (END_OF_CRATE_DOCS)

## License

Licensed under either of:

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

at your option.

### Your contributions

Unless you explicitly state otherwise,
any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, 
shall be dual licensed as above,
without any additional terms or conditions.