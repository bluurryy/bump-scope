// NB: We avoid using closures to map `Result` and `Option`s in various places because they result in less readable assembly output.
// When using closures, functions like `capacity_overflow` can get the name of some closure that invokes it instead, like `bump_scope::mut_bump_vec::MutBumpVec<T,_,_,A>::generic_grow_amortized::{{closure}}`.

// This crate uses modified code from the rust standard library. <https://github.com/rust-lang/rust/tree/master/library>.
// Especially `BoxU` methods, vectors, strings, `polyfill` and `tests/from_std` are based on code from the standard library.

#![no_std]
#![cfg_attr(
    feature = "nightly",
    feature(clone_to_uninit, ptr_metadata, allocator_ext, doc_notable_trait)
)]
#![cfg_attr(all(feature = "nightly", test, doc), feature(offset_of_enum))]
#![cfg_attr(docsrs,
    feature(doc_cfg),
    doc(auto_cfg(hide(feature, values("panic-on-alloc")))) // too noisy
)]
#![warn(
    clippy::pedantic,
    clippy::cargo,
    clippy::correctness,
    clippy::perf,
    clippy::style,
    clippy::suspicious,
    missing_docs,
    rustdoc::missing_crate_level_docs
)]
#![expect(
    clippy::inline_always,
    clippy::module_name_repetitions,
    clippy::copy_iterator,
    clippy::items_after_statements,
    clippy::missing_transmute_annotations
)]
#![allow(
    clippy::wildcard_imports, // `expect` is broken for this lint
    clippy::collapsible_else_if, // this is not `expect` because nightly as of 2025-12-28 doesn't warn about this for some reason
)]
#![doc(test(
    attr(deny(dead_code, unused_imports, deprecated)),
    attr(allow(unused_features)),
    attr(cfg_attr(feature = "nightly", feature(allocator_ext, btreemap_alloc))),
))]
#![cfg_attr(feature = "nightly", allow(
    clippy::incompatible_msrv,
    invalid_doc_attributes, // false positive for doc_notable_trait
))]
//! <!-- crate documentation intro start -->
//! A fast bump allocator that supports allocation scopes / checkpoints. Aka an arena for values of arbitrary types.
//! <!-- crate documentation intro end -->
//!
//! **[Changelog][CHANGELOG] -**
//! **[Crates.io](https://crates.io/crates/bump-scope) -**
//! **[Repository](https://github.com/bluurryy/bump-scope)**
//!
//! <!-- crate documentation rest start -->
//! # What is bump allocation?
//! A bump allocator owns a big chunk of memory. It has a pointer that starts at one end of that chunk.
//! When an allocation is made that pointer gets aligned and bumped towards the other end of the chunk.
//! When its chunk is full, this allocator allocates another chunk with twice the size.
//!
//! This makes allocations very fast. The drawback is that you can't reclaim memory like you do with a more general allocator.
//! Memory for the most recent allocation *can* be reclaimed. You can also use [scopes, checkpoints](#scopes-and-checkpoints) and [`reset`](Bump::reset) to reclaim memory.
//!
//! A bump allocator is great for *phase-oriented allocations* where you allocate objects in a loop and free them at the end of every iteration.
//! ```
//! use bump_scope::Bump;
//! let mut bump: Bump = Bump::new();
//! # let mut first = true;
//!
//! loop {
//!     # if !first { break }; first = false;
//!     // use bump ...
//!     bump.reset();
//! }
//! ```
//! The fact that the bump allocator allocates ever larger chunks and [`reset`](Bump::reset) only keeps around the largest one means that after a few iterations, every bump allocation
//! will be done on the same chunk and no more chunks need to be allocated.
//!
//! The introduction of scopes makes this bump allocator also great for temporary allocations and stack-like usage.
//!
//! # Overview
//!
//! - Implements `core`'s `Allocator` and takes an `Allocator`.
//! - Supports [scopes and checkpoints](#scopes-and-checkpoints).
//! - Has helper methods in [`BumpAllocator`] for safely allocating values, slices, strings, collecting iterators, formatting, etc.
//! - Very fast, see [./crates/callgrind-benches][benches].
//! - Provides configuration options, to let you optimize the bump allocator for your application.
//! - Supports parallel bump allocation with a [`BumpPool`].
//!
//! # Scopes and Checkpoints
//!
//! You can create scopes to make allocations that live only for a part of its parent scope.
//! Entering and exiting scopes is virtually free. Allocating within a scope has no overhead.
//!
//! You can create a new scope either with a [`scoped`](Bump::scoped) closure or with a [`scope_guard`](Bump::scope_guard):
//! ```
//! use bump_scope::{Bump, BumpAllocator};

//! let mut bump: Bump = Bump::new();
//!
//! // you can use a closure
//! bump.scoped(|mut bump| {
//!     let hello = bump.alloc_str("hello").into_ref();
//!     assert_eq!(bump.allocated(), 5);
//!
//!     bump.scoped(|bump| {
//!         let world = bump.alloc_str("world").into_ref();
//!
//!         println!("{hello} and {world} are both live");
//!         assert_eq!(bump.allocated(), 10);
//!     });
//!
//!     println!("{hello} is still live");
//!     assert_eq!(bump.allocated(), 5);
//! });
//!
//! assert_eq!(bump.allocated(), 0);
//!
//! // or you can use scope guards
//! {
//!     let mut guard = bump.scope_guard();
//!     let mut bump = guard.scope();
//!
//!     let hello = bump.alloc_str("hello").into_ref();
//!     assert_eq!(bump.allocated(), 5);
//!
//!     {
//!         let mut guard = bump.scope_guard();
//!         let bump = guard.scope();
//!
//!         let world = bump.alloc_str("world").into_ref();
//!
//!         println!("{hello} and {world} are both live");
//!         assert_eq!(bump.allocated(), 10);
//!     }
//!
//!     println!("{hello} is still live");
//!     assert_eq!(bump.allocated(), 5);
//! }
//!
//! assert_eq!(bump.allocated(), 0);
//! ```
//! You can also use the unsafe [`checkpoint`](Bump::checkpoint) api
//! to reset the bump pointer to a previous position.
//! ```
//! use bump_scope::{Bump, BumpAllocator};
//!
//! let bump: Bump = Bump::new();
//! let checkpoint = bump.checkpoint();
//!
//! {
//!     let hello = bump.alloc_str("hello");
//!     assert_eq!(bump.allocated(), 5);
//!     # _ = hello;
//! }
//!
//! unsafe { bump.reset_to(checkpoint); }
//! assert_eq!(bump.allocated(), 0);
//! ```
//! When using a `Bump(Scope)` as an allocator for collections you will find that you can no longer
//! call `scoped` or `scope_guard` because those functions require `&mut self` which does not allow
//! any outstanding references to the allocator.
//!
//! As a workaround you can use the [`claim`] method on a `&Bump(Scope)` to return a `BumpClaimGuard` which
//! mutably dereferences to a `BumpScope`, enabling you to call `.scoped()` and `.scope_guard()`.
//!
//! A `bump.claim()` call replaces the allocator referred to by `bump` with a dummy allocator while the returned `BumpClaimGuard`
//! is live. This dummy allocator errors when trying to allocate or grow memory and reports an empty bump allocator from the `stats` api.
//!
//! This makes it possible to enter scopes while a there are still outstanding
//! references to that bump allocator:
//! ```
//! use bump_scope::{Bump, BumpScope};
//! let bump: Bump = Bump::new();
//!
//! let mut vec: Vec<u8, &Bump> = Vec::new_in(&bump);
//!
//! bump.claim().scoped(|bump_scope| {
//!     // you can allocate in the scope as usual
//!     let mut vec2: Vec<u8, &BumpScope> = Vec::new_in(bump_scope);
//!     vec2.reserve(456);
//!
//!     // allocating on the `bump` outside the scope will fail
//!     assert!(vec.try_reserve(123).is_err());
//! });
//!
//! // now allocating on `bump` is possible again
//! vec.reserve(123);
//! ```
//!
//! # Feature Flags
//! <!-- feature documentation start -->
//! - **`std`** *(enabled by default)* — Adds `BumpPool` and implementations of `std::io` traits.
//! - **`panic-on-alloc`** *(enabled by default)* — Adds functions and traits that will panic when allocations fail.
//!   Without this feature, allocation failures cannot cause panics, and only
//!   `try_`-prefixed allocation methods will be available.
//! - **`bytemuck`** — Adds `bytemuck` module with
//!   <code>[BumpAllocatorExt](bytemuck::BumpAllocatorExt)::[alloc_zeroed](bytemuck::BumpAllocatorExt::alloc_zeroed)([_slice](bytemuck::BumpAllocatorExt::alloc_zeroed_slice))</code>
//!   and <code>[box_zero_init](bytemuck::box_zero_init)([_slice](bytemuck::box_zero_init_slice))</code>.
//! - **`zerocopy-08`** — Adds `zerocopy_08` module with
//!   <code>[BumpAllocatorExt](zerocopy_08::BumpAllocatorExt)::[alloc_zeroed](zerocopy_08::BumpAllocatorExt::alloc_zeroed)([_slice](zerocopy_08::BumpAllocatorExt::alloc_zeroed_slice))</code>
//!   and <code>[box_zero_init](zerocopy_08::box_zero_init)([_slice](zerocopy_08::box_zero_init_slice))</code>.
//! - **`nightly`** — Enables nightly features. ***This feature is exempt from semver.***
//!   - Adds `Take(OwnedSlice)` implementations for `vec::IntoIter` and `vec::Drain` of any allocator.
//!   - Uses std's `vec::from_elem_in` instead of our polyfill for the `vec!` macro
//!   - Implements `AllocatorNightly` for our bump allocators
//!   - Adds `BumpAllocator::alloc_clone`
//!   - Promises symbolic alignment to miri for bump allocations
//! <!-- feature documentation end -->
//!
//! # MSRV policy
//! The minimum supported rust version may increase with a minor release.
//! It's currently **<!--msrv start-->1.100.0<!--msrv end-->**.
//!
//! [benches]: https://github.com/bluurryy/bump-scope/tree/main/crates/callgrind-benches
//! [`new`]: Bump::new
//! [`Allocator`]: core::alloc::Allocator
//! [`with_size`]: Bump::with_size
//! [`with_capacity`]: Bump::with_capacity
//! [`scoped`]: crate::BumpScope::scoped
//! [`scope_guard`]: crate::BumpScope::scope_guard
//! [`claim`]: crate::BumpScope::claim
//! <!-- crate documentation rest end -->

#[cfg(any(feature = "std", test))]
extern crate std;

extern crate alloc;

mod allocator_impl;
mod bump;
mod bump_align_guard;
mod bump_allocation;
mod bump_claim_guard;
mod bump_imp;
#[cfg(feature = "std")]
mod bump_pool;
mod bump_scope;
mod bump_scope_guard;
mod bumping;
mod bytemuck_and_zerocopy;
pub(crate) mod chunk_size;
pub(crate) mod chunk_size_config;
pub(crate) mod chunk_types;
pub mod config;
mod containers;
mod destructure;
mod error_mode;
pub mod extra;
mod global_or_system;
mod layout;
mod macros;
mod no_drop;
mod owned_slice;
mod polyfill;
/// This is not part of the public api. This module is semver-exempt!
#[doc(hidden)]
pub mod private;
pub mod stats;
#[cfg(test)]
mod tests;
mod traits;
mod unmanaged;
mod util;

use alloc::{boxed::Box, vec::Vec};
pub use bump::Bump;
pub use bump_allocation::BumpAllocation;
pub use bump_claim_guard::BumpClaimGuard;
#[cfg(feature = "std")]
pub use bump_pool::{BumpPool, BumpPoolGuard};
pub use bump_scope::BumpScope;
pub use bump_scope_guard::{BumpScopeGuard, Checkpoint};
pub use no_drop::NoDrop;
#[cfg(feature = "panic-on-alloc")]
use private::capacity_overflow;
pub use traits::{AsBumpScope, AsMutBumpScope, BumpAllocator, BumpAllocatorScope, MutBumpAllocator, MutBumpAllocatorScope};
pub use unmanaged::Unmanaged;

#[cfg(feature = "bytemuck")]
pub use bytemuck_and_zerocopy::bytemuck;

#[cfg(feature = "zerocopy-08")]
pub use bytemuck_and_zerocopy::zerocopy_08;

/// A `Box` without an allocator, alias for <code>Box<T, [Unmanaged]<'a>></code>.
pub type BoxU<'a, T> = Box<T, Unmanaged<'a>>;

/// A `Vec` without an allocator, alias for <code>Vec<T, [Unmanaged]<'a>></code>.
pub type VecU<'a, T> = Vec<T, Unmanaged<'a>>;

/// The changelog.
///
#[doc = include_str!("../CHANGELOG.md")]
#[expect(non_snake_case)]
pub mod CHANGELOG {}
