#![no_std]
#![cfg_attr(
    feature = "nightly",
    allow(incomplete_features),
    feature(allocator_ext, trivial_clone, const_trait_impl, specialization)
)]
#![warn(
    clippy::pedantic,
    clippy::cargo,
    clippy::correctness,
    clippy::perf,
    clippy::style,
    clippy::suspicious,
    clippy::as_pointer_underscore,
    clippy::as_ptr_cast_mut,
    clippy::assigning_clones,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::else_if_without_else,
    clippy::empty_drop,
    clippy::infinite_loop,
    missing_docs,
    rustdoc::missing_crate_level_docs
)]
#![deny(unconditional_recursion)]
#![expect(
    clippy::inline_always,
    clippy::module_name_repetitions,
    clippy::copy_iterator,
    clippy::items_after_statements
)]
//! A fast bump allocator that supports allocation scopes / checkpoints. Aka an arena for values of arbitrary types.
//!
//! This crate provides just the core bump allocation facilities. Its goal is to provide a stable and simple if inconvenient api.
//! For a rich api with helper methods for safe allocations, support for parallel allocation and `Box` and `Vec` interactions have a look at [bump-scope].
//!
//! # Overview
//!
//! - Implements `core`'s `Allocator` and takes an `Allocator`.
//! - Supports [scopes and checkpoints](#scopes-and-checkpoints).
//! - Very fast, see [benches].
//! - Provides configuration options to let you optimize the bump allocator for your application.
//!
//! # Scopes and Checkpoints
//!
//! You can create scopes to make allocations that live only for a part of its parent scope.
//! Entering and exiting scopes is virtually free. Allocating within a scope has no overhead.
//!
//! You can create a new scope with a [`scope_guard`]:
//! ```
//! # extern crate alloc;
//! use alloc::alloc::{Global, Layout};
//! use bump_scope_core::config::DEFAULT_BUMP_CONFIG;
//!
//! type Bump = bump_scope_core::Bump<DEFAULT_BUMP_CONFIG, Global>;
//! type BumpScope = bump_scope_core::BumpScope<DEFAULT_BUMP_CONFIG, Global>;
//!
//! let mut bump = Bump::new_in(Global)?;
//!
//! {
//!     let mut bump = bump.scope_guard();
//!     let two = bump.alloc_sized::<!, [u8; 2]>()?;
//!     # _ = two;
//!     assert_eq!(bump.allocated(), 2);
//!
//!     {
//!         let mut bump = bump.scope_guard();
//!         let three = bump.alloc_sized::<!, [u8; 3]>();
//!         # _ = three;
//!         assert_eq!(bump.allocated(), 5);
//!     }
//!
//!     assert_eq!(bump.allocated(), 2);
//! }
//!
//! assert_eq!(bump.stats().allocated(), 0);
//! # Ok::<(), !>(())
//! ```
//! You can also use the unsafe [`checkpoint`] api
//! to reset the bump pointer to a previous position.
//! ```
//! # extern crate alloc;
//! # use alloc::alloc::{Global, Layout, AllocError};
//! # use bump_scope_core::config::DEFAULT_BUMP_CONFIG;
//! # type Bump = bump_scope_core::Bump<DEFAULT_BUMP_CONFIG, Global>;
//! # type BumpScope = bump_scope_core::BumpScope<DEFAULT_BUMP_CONFIG, Global>;
//! # let mut bump = Bump::new_in(Global)?;
//! #
//! let bump = Bump::new_in(Global)?;
//! let checkpoint = bump.checkpoint();
//!
//! let three = bump.alloc_sized::<!, [u8; 3]>()?;
//! assert_eq!(bump.allocated(), 3);
//! # _ = three;
//!
//! unsafe { bump.reset_to(checkpoint); }
//! assert_eq!(bump.allocated(), 0);
//! # Ok::<(), !>(())
//! ```
//!
//! # Collections and Scopes
//!
//! When using a `Bump(Scope)` as an allocator for a collection you will find that you can no longer
//! call `scope_guard` since it requires `&mut self` which does not allow any outstanding references
//! to the allocator.
//!
//! ```compile_fail,E0502
//! # extern crate alloc;
//! # use alloc::alloc::{Global, Layout, AllocError};
//! # use bump_scope_core::config::DEFAULT_BUMP_CONFIG;
//! # type Bump = bump_scope_core::Bump<DEFAULT_BUMP_CONFIG, Global>;
//! # type BumpScope = bump_scope_core::BumpScope<DEFAULT_BUMP_CONFIG, Global>;
//! # let mut bump = Bump::new_in(Global)?;
//! #
//! let vec: Vec<u8, &Bump> = Vec::new_in(&bump);
//!
//! {
//!     let bump = bump.scope_guard();
//!     # _ = bump;
//!     // ...
//! }
//! # _ = vec;
//! # Ok::<(), !>(())
//! ```
//!
//! As a workaround you can use the [`claim`] method on `&Bump(Scope)` to return a `BumpClaimGuard` which
//! mutably dereferences to a `BumpScope`, enabling you to call `.scoped()` and `.scope_guard()`.
//!
//! A `bump.claim()` call replaces the allocator referred to by `bump` with a dummy allocator while the returned `BumpClaimGuard`
//! is live. This dummy allocator errors when trying to allocate or grow memory and reports an empty bump allocator from the `stats` api.
//!
//! This makes it possible to enter scopes while a there are still outstanding
//! references to that bump allocator:
//! ```
//! # extern crate alloc;
//! # use bump_scope_core::config::DEFAULT_BUMP_CONFIG;
//! # use alloc::alloc::{Global, Layout, AllocError};
//! # type Bump = bump_scope_core::Bump<DEFAULT_BUMP_CONFIG, Global>;
//! # type BumpScope = bump_scope_core::BumpScope<DEFAULT_BUMP_CONFIG, Global>;
//! # let mut bump = Bump::new_in(Global)?;
//! #
//! let mut vec: Vec<u8, &Bump> = Vec::new_in(&bump);
//!
//! {
//!     let mut guard = bump.claim();
//!     let mut bump_scope = guard.scope_guard();
//!
//!     // you can allocate in the scope as usual
//!     let mut vec2: Vec<u8, &BumpScope> = Vec::new_in(&bump_scope);
//!     vec2.reserve(456);
//!
//!     // allocating on the `bump` outside the scope will fail
//!     assert!(vec.try_reserve(123).is_err());
//! }
//!
//! // now allocating on `bump` is possible again
//! vec.reserve(123);
//! # Ok::<(), !>(())
//! ```
//!
//! # Feature Flags
//! <!-- feature documentation start -->
//! - **`alloc`** *(enabled by default)* — Makes use of [`alloc::alloc::handle_alloc_error`] when appropriate.
//! - **`nightly`** — Enables nightly features. ***This feature is exempt from semver.***
//!   - Promises symbolic alignment to miri for bump allocations
//!   - Reduces the size of `Bump(Scope)` when the base allocator is `AllocatorClone + TriviallyClone`
//! <!-- feature documentation end -->
//!
//! [benches]: https://github.com/bluurryy/bump-scope/tree/main/benches
//! [`claim`]: BumpScope::claim
//! [`checkpoint`]: BumpScope::checkpoint
//! [`scope_guard`]: BumpScope::scope_guard
//! [bump-scope]: https://docs.rs/bump-scope

#[cfg(feature = "alloc")]
extern crate alloc;

mod allocator_impl;
mod base_allocator_storage;
mod bump;
mod bump_scope;
mod bumping;
mod checkpoint;
mod chunk_size;
mod chunk_size_config;
mod chunk_types;
mod destructure;
mod error_mode;
mod layout;
mod polyfill;
mod util;

mod bump_align_guard;
mod bump_claim_guard;
mod bump_scope_guard;

pub mod config;
pub mod stats;

pub use bump::Bump;
pub use bump_claim_guard::BumpClaimGuard;
pub use bump_scope::BumpScope;
pub use bump_scope_guard::BumpScopeGuard;
pub use checkpoint::Checkpoint;
pub use error_mode::ErrorMode;
