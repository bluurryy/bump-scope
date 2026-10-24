// NB: We avoid using closures to map `Result` and `Option`s in various places because they result in less readable assembly output.
// When using closures, functions like `capacity_overflow` can get the name of some closure that invokes it instead, like `bump_scope::mut_bump_vec::MutBumpVec<T,_,_,A>::generic_grow_amortized::{{closure}}`.

// This crate uses modified code from the rust standard library. <https://github.com/rust-lang/rust/tree/master/library>.
// Especially `BoxU` methods, vectors, strings, `polyfill` and `tests/from_std` are based on code from the standard library.

#![no_std]
#![cfg_attr(feature = "nightly", feature(clone_to_uninit, ptr_metadata, allocator_ext))]
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
    clippy::partialeq_ne_impl,
    clippy::items_after_statements,
    clippy::missing_transmute_annotations
)]
#![allow(
    clippy::wildcard_imports, // `expect` is broken for this lint
    clippy::collapsible_else_if, // this is not `expect` because nightly as of 2025-12-28 doesn't warn about this for some reason
)]
#![doc(test(
    attr(deny(dead_code, unused_imports, deprecated)),
    attr(cfg_attr(feature = "nightly-tests", feature(allocator_ext))),
))]
#![cfg_attr(feature = "nightly", allow(clippy::incompatible_msrv))]
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
//! # Comparison to [`bumpalo`](https://docs.rs/bumpalo)
//!
//! Bumpalo is a popular crate for bump allocation.
//! This crate was inspired by bumpalo and [Always Bump Downwards](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html)
//! (but ignores the title).
//!
//! Unlike `bumpalo`, this crate...
//! - Supports [scopes and checkpoints](#scopes-and-checkpoints).
//! - Drop is always called for allocated values unless explicitly [leaked](BoxU::leak) or [forgotten](core::mem::forget).
//!   - `alloc*` methods return a [`BumpAllocation`] which owns and drops `T`. Types that don't need dropping can be turned into references with [`into_ref`](BumpAllocation::into_ref) and [`into_mut`](BumpAllocation::into_mut).
//! - You can allocate a slice from any `Iterator` with [`alloc_iter`](Bump::alloc_iter).
//! - `Bump`'s base allocator is generic.
//! - Won't try to allocate a smaller chunk if allocation failed.
//! - No built-in allocation limit. You can provide an allocator that enforces an allocation limit (see `examples/limit_memory_usage.rs`).
//! - Allocations are a tiny bit more optimized. See [./crates/callgrind-benches][benches].
//! - [You can choose the bump direction.](crate::config) Bumps upwards by default.
//!
//! # Allocator Methods
//!
//! The bump allocator provides many methods to conveniently allocate values, strings, and slices.
//! Have a look at the documentation of [`Bump`] for a method overview.
//!
//! # Scopes and Checkpoints
//!
//! You can create scopes to make allocations that live only for a part of its parent scope.
//! Entering and exiting scopes is virtually free. Allocating within a scope has no overhead.
//!
//! You can create a new scope either with a [`scoped`](Bump::scoped) closure or with a [`scope_guard`](Bump::scope_guard):
//! ```
//! use bump_scope::Bump;
//!
//! let mut bump: Bump = Bump::new();
//!
//! // you can use a closure
//! bump.scoped(|mut bump| {
//!     let hello = bump.alloc_str("hello");
//!     assert_eq!(bump.stats().allocated(), 5);
//!
//!     bump.scoped(|bump| {
//!         let world = bump.alloc_str("world");
//!
//!         println!("{hello} and {world} are both live");
//!         assert_eq!(bump.stats().allocated(), 10);
//!     });
//!
//!     println!("{hello} is still live");
//!     assert_eq!(bump.stats().allocated(), 5);
//! });
//!
//! assert_eq!(bump.stats().allocated(), 0);
//!
//! // or you can use scope guards
//! {
//!     let mut guard = bump.scope_guard();
//!     let mut bump = guard.scope();
//!
//!     let hello = bump.alloc_str("hello");
//!     assert_eq!(bump.stats().allocated(), 5);
//!
//!     {
//!         let mut guard = bump.scope_guard();
//!         let bump = guard.scope();
//!
//!         let world = bump.alloc_str("world");
//!
//!         println!("{hello} and {world} are both live");
//!         assert_eq!(bump.stats().allocated(), 10);
//!     }
//!
//!     println!("{hello} is still live");
//!     assert_eq!(bump.stats().allocated(), 5);
//! }
//!
//! assert_eq!(bump.stats().allocated(), 0);
//! ```
//! You can also use the unsafe [`checkpoint`](Bump::checkpoint) api
//! to reset the bump pointer to a previous position.
//! ```
//! # use bump_scope::Bump;
//! let bump: Bump = Bump::new();
//! let checkpoint = bump.checkpoint();
//!
//! {
//!     let hello = bump.alloc_str("hello");
//!     assert_eq!(bump.stats().allocated(), 5);
//!     # _ = hello;
//! }
//!
//! unsafe { bump.reset_to(checkpoint); }
//! assert_eq!(bump.stats().allocated(), 0);
//! ```
//! When using a `Bump(Scope)` as an allocator for collections you will find that you can no longer
//! call `scoped` or `scope_guard` because those functions require `&mut self` which does not allow
//! any outstanding references to the allocator.
//!
//! As a workaround you can use the [`claim`] method on a `&Bump(Scope)` to return a `BumpClaimGuard` which
//! mutably dereferences to a `BumpScope`, allowing you to call `.scoped()` and `.scope_guard()`.
//!
//! A `bump.claim()` call replaces the allocator referred to by `bump` with a dummy allocator while the returned `BumpClaimGuard`
//! is live. This dummy allocator errors when trying to allocate or grow memory and reports an empty bump allocator from the `stats` api.
//!
//! This makes it possible to enter scopes while a there are still outstanding
//! references to that bump allocator:
//! ```
//! # use bump_scope::{ Bump, BumpScope, BumpVec as Vec };
//! let bump: Bump = Bump::new();
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
//! # Parallel Allocation
//! [`Bump`] is `!Sync` which means it can't be shared between threads.
//!
//! To bump allocate in parallel you can use a [`BumpPool`].
//!
//! # Allocator API
//! `Bump` and `BumpScope` implement `bump-scope`'s own [`Allocator`] trait and with the
//! respective [feature flags](#feature-flags) also implement `allocator_api2` version `0.2`,
//! `0.3`, `0.4` and nightly's `Allocator` trait.
//!
//! This allows you to [bump allocate collections](crate::Bump#collections).
//!
//! A bump allocator can grow, shrink and deallocate the most recent allocation.
//! When bumping upwards it can even do so in place.
//! Growing allocations other than the most recent one will require a new allocation and the old memory block becomes wasted space.
//! Shrinking or deallocating allocations other than the most recent one does nothing, which means wasted space.
//!
//! A bump allocator does not require `deallocate` or `shrink` to free memory.
//! After all, memory will be reclaimed when exiting a scope, calling `reset` or dropping the `Bump`.
//! You can set [the `deallocates` and `shrinks` config parameters](crate::config) to false to make deallocating and shrinking a no-op.
//!
//! # Feature Flags
//! <!-- feature documentation start -->
//! <!-- feature documentation end -->
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
mod boxed_slice_initializer;
mod bump;
mod bump_align_guard;
mod bump_allocation;
mod bump_claim_guard;
#[cfg(feature = "std")]
mod bump_pool;
mod bump_scope;
mod bump_scope_guard;
mod bumping;
pub(crate) mod chunk_size;
pub(crate) mod chunk_size_config;
pub(crate) mod chunk_types;
pub mod config;
mod destructure;
mod error_behavior;
/// TODO
pub mod extra;
mod layout;
mod macros;
mod no_drop;
/// TODO
pub mod owned_slice;
mod polyfill;
mod raw_bump;
pub mod stats;
mod string;
#[cfg(test)]
mod tests;
/// TODO:
pub mod traits;
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
#[cfg(test)]
use core::{
    alloc::{AllocError, Allocator, Layout},
    ptr::NonNull,
};
use core::{mem, num::NonZeroUsize};
pub use no_drop::NoDrop;
#[cfg(feature = "panic-on-alloc")]
use private::capacity_overflow;
pub use traits::{BumpAllocator, BumpAllocatorScope, MutBumpAllocator, MutBumpAllocatorScope};
pub use unmanaged::Unmanaged;

/// TODO
pub type BoxU<'a, T> = Box<T, Unmanaged<'a>>;

/// TODO
pub type VecU<'a, T> = Vec<T, Unmanaged<'a>>;

/// The changelog.
///
#[doc = include_str!("../CHANGELOG.md")]
#[expect(non_snake_case)]
pub mod CHANGELOG {}

/// Does not check for overflow.
#[inline(always)]
fn up_align_usize_unchecked(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    (addr + mask) & !mask
}

#[inline(always)]
fn down_align_usize(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}

#[inline(always)]
fn bump_down(addr: NonZeroUsize, size: usize, align: usize) -> usize {
    let subtracted = addr.get().saturating_sub(size);
    down_align_usize(subtracted, align)
}

/// This is not part of the public api!
///
/// Any changes to this module are semver-exempt!
#[doc(hidden)]
pub mod private {
    pub extern crate alloc;
    pub extern crate core;

    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    pub const fn capacity_overflow() -> ! {
        panic!("capacity overflow");
    }

    #[cold]
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    pub const fn format_trait_error() -> ! {
        panic!("formatting trait implementation returned an error");
    }
}

#[cfg(feature = "panic-on-alloc")]
use alloc::alloc::handle_alloc_error;

// This is just `Result::into_ok` but with a name to match our use case.
#[inline(always)]
#[cfg(feature = "panic-on-alloc")]
fn panic_on_error<T>(result: Result<T, !>) -> T {
    result.into_ok()
}

trait SizedTypeProperties: Sized {
    const SIZE: usize = mem::size_of::<Self>();
    const ALIGN: usize = mem::align_of::<Self>();

    const IS_ZST: bool = mem::size_of::<Self>() == 0;
}

impl<T> SizedTypeProperties for T {}

/// Aligns a bump address to the `min_align`.
///
/// This is a noop when `min_align` is `0`.
#[must_use]
#[inline(always)]
fn align_pos(up: bool, min_align: usize, pos: usize) -> usize {
    if up {
        // Aligning an address that is `<= range.end` with an alignment
        // that is `<= MIN_CHUNK_ALIGN` cannot exceed `range.end` and
        // cannot overflow as `range.end` is always aligned to `MIN_CHUNK_ALIGN`.
        up_align_usize_unchecked(pos, min_align)
    } else {
        // The chunk start is non-null and is aligned to `MIN_CHUNK_ALIGN`
        // `MIN_ALIGN <= MIN_CHUNK_ALIGN` will never pass the chunk start
        // and stay non-zero.
        down_align_usize(pos, min_align)
    }
}

// Used for static assertions.
#[cfg(test)]
#[derive(Default, Clone)]
pub(crate) struct NoopAllocator;

#[cfg(test)]
unsafe impl Allocator for NoopAllocator {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
}

#[cold]
#[inline(never)]
fn cant_change_config_due_to_const_constructible() -> ! {
    panic!("can't change config: `const_constructible` becomes false and `self` has no chunks");
}

#[cold]
#[inline(never)]
fn cant_change_config_due_to_claimable() -> ! {
    panic!("can't change config: `claimable` becomes false and `self.is_claimed()`");
}
