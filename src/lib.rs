// NB: We avoid using closures to map `Result` and `Option`s in various places because they result in less readable assembly output.
// When using closures, functions like `capacity_overflow` can get the name of some closure that invokes it instead, like `bump_scope::mut_bump_vec::MutBumpVec<T,_,_,A>::generic_grow_amortized::{{closure}}`.

// This crate uses modified code from the rust standard library. <https://github.com/rust-lang/rust/tree/master/library>.
// Especially `BumpBox` methods, vectors, strings, `polyfill` and `tests/from_std` are based on code from the standard library.

#![no_std]
#![cfg_attr(
    any(feature = "nightly-allocator-api", feature = "nightly-fn-traits"),
    feature(allocator_api)
)]
#![cfg_attr(feature = "nightly-coerce-unsized", feature(coerce_unsized, unsize))]
#![cfg_attr(feature = "nightly-exact-size-is-empty", feature(exact_size_is_empty))]
#![cfg_attr(feature = "nightly-trusted-len", feature(trusted_len))]
#![cfg_attr(feature = "nightly-fn-traits", feature(fn_traits, tuple_trait, unboxed_closures))]
#![cfg_attr(feature = "nightly-tests", feature(offset_of_enum))]
#![cfg_attr(feature = "nightly-dropck-eyepatch", feature(dropck_eyepatch))]
#![cfg_attr(feature = "nightly-clone-to-uninit", feature(clone_to_uninit, ptr_metadata))]
#![cfg_attr(docsrs,
    feature(doc_cfg),
    doc(auto_cfg(hide(feature = "panic-on-alloc"))) // too noisy
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
    clippy::missing_transmute_annotations,
    clippy::multiple_crate_versions, // we have allocator-api2 version 0.2 and 0.3
)]
#![allow(
    clippy::wildcard_imports, // `expect` is broken for this lint
    clippy::collapsible_else_if, // this is not `expect` because nightly as of 2025-12-28 doesn't warn about this for some reason
)]
#![doc(test(
    attr(deny(dead_code, unused_imports, deprecated)),
    attr(cfg_attr(feature = "nightly-allocator-api", feature(allocator_api, btreemap_alloc))),
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
//! # Comparison to [`bumpalo`](https://docs.rs/bumpalo)
//!
//! Bumpalo is a popular crate for bump allocation.
//! This crate was inspired by bumpalo and [Always Bump Downwards](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html)
//! (but ignores the title).
//!
//! Unlike `bumpalo`, this crate...
//! - Supports [scopes and checkpoints](#scopes-and-checkpoints).
//! - Drop is always called for allocated values unless explicitly [leaked](BumpBox::leak) or [forgotten](core::mem::forget).
//!   - `alloc*` methods return a [`BumpBox<T>`](BumpBox) which owns and drops `T`. Types that don't need dropping can be turned into references with [`into_ref`](BumpBox::into_ref) and [`into_mut`](BumpBox::into_mut).
//! - You can allocate a slice from any `Iterator` with [`alloc_iter`](Bump::alloc_iter).
//! - `Bump`'s base allocator is generic.
//! - Won't try to allocate a smaller chunk if allocation failed.
//! - No built-in allocation limit. You can provide an allocator that enforces an allocation limit (see `examples/limit_memory_usage.rs`).
//! - Allocations are a tiny bit more optimized. See [./crates/callgrind-benches][benches].
//! - [You can choose the bump direction.](crate::settings#bumping-upwards-or-downwards) Bumps upwards by default.
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
//! call `scoped` or `scope_guard` because those functions require `&mut self` which means no outstanding
//! references to the allocator can exist.
//!
//! As a workaround you can use [`claim`] to essentially turn a `&Bump(Scope)` into a `&mut BumpScope`.
//! The `claim` method works by temporarily replacing the allocator of the original `&Bump(Scope)` with
//! a dummy allocator that will fail allocation requests, panics on `scoped` and will report an empty
//! bump allocator from the `stats` api. The returned [`BumpClaimGuard`] will then have exclusive access to
//! bump allocation and can mutably deref to `&mut BumpScope`:
//! ```
//! # use bump_scope::{ Bump, BumpVec as Vec };
//! let bump: Bump = Bump::new();
//! let mut vec: Vec<u8, &Bump> = Vec::new_in(&bump);
//!
//! bump.claim().scoped(|bump| {
//!     // allocating on the original bump will
//!     // fail while the bump allocator is claimed
//!     assert!(vec.try_reserve(123).is_err());
//! });
//!
//! // now allocation on the original bump succeeds again
//! vec.reserve(123);
//! ```
//!
//! # Collections
//! `bump-scope` provides bump allocated versions of `Vec` and `String` called [`BumpVec`] and [`BumpString`].
//! They are also available in the following variants:
//! - [`Fixed*`](FixedBumpVec) for fixed capacity collections
//! - [`Mut*`](MutBumpVec) for collections optimized for a mutable bump allocator
//!
//! #### API changes
//! The collections are designed to have the same api as their std counterparts with these exceptions:
//! - [`split_off`](BumpVec::split_off) —  splits the collection in place without allocation; the parameter is a range instead of a single index
//! - [`retain`](BumpVec::retain) —  takes a closure with a `&mut T` parameter like [`Vec::retain_mut`](alloc_crate::vec::Vec::retain_mut)
//!
//! #### New features
//! - [`append`](BumpVec::append) —  allows appending all kinds of owned slice types like `[T; N]`, `Box<[T]>`, `Vec<T>`, `vec::Drain<T>` etc.
//! - [`map`](BumpVec::map) —  maps the elements, potentially reusing the existing allocation
//! - [`map_in_place`](BumpVec::map_in_place) —  maps the elements without allocation, failing to compile if not possible
//! - conversions between the regular collections, their `Fixed*` variants and `BumpBox<[T]>` / `BumpBox<str>`
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
//! You can set the `DEALLOCATES` and `SHRINKS` parameters to false or use the [`WithoutDealloc`] and [`WithoutShrink`] wrappers
//! to make deallocating and shrinking a no-op.
//!
//! # Feature Flags
//! <!-- feature documentation start -->
//! - **`std`** *(enabled by default)* — Adds `BumpPool` and implementations of `std::io` traits.
//! - **`alloc`** *(enabled by default)* — Adds `Global` as the default base allocator and some interactions with `alloc` collections.
//! - **`panic-on-alloc`** *(enabled by default)* — Adds functions and traits that will panic when allocations fail.
//!   Without this feature, allocation failures cannot cause panics, and only
//!   `try_`-prefixed allocation methods will be available.
//! - **`serde`** — Adds `Serialize` implementations for `BumpBox`, strings and vectors, and `DeserializeSeed` for strings and vectors.
//! - **`bytemuck`** — Adds `bytemuck::*` extension traits for
//!   <code>[alloc_zeroed](bytemuck::BumpAllocatorTypedScopeExt::alloc_zeroed)([_slice](bytemuck::BumpAllocatorTypedScopeExt::alloc_zeroed_slice))</code>,
//!   [`init_zeroed`](bytemuck::InitZeroed::init_zeroed),
//!   [`extend_zeroed`](bytemuck::VecExt::extend_zeroed) and
//!   [`resize_zeroed`](bytemuck::VecExt::resize_zeroed).
//! - **`zerocopy-08`** — Adds `zerocopy_08::*` extension traits for
//!   <code>[alloc_zeroed](zerocopy_08::BumpAllocatorTypedScopeExt::alloc_zeroed)([_slice](zerocopy_08::BumpAllocatorTypedScopeExt::alloc_zeroed_slice))</code>,
//!   [`init_zeroed`](zerocopy_08::InitZeroed::init_zeroed),
//!   [`extend_zeroed`](zerocopy_08::VecExt::extend_zeroed) and
//!   [`resize_zeroed`](zerocopy_08::VecExt::resize_zeroed).
//! - **`allocator-api2-02`** — Makes `Bump(Scope)` implement `allocator_api2` version `0.2`'s `Allocator` and
//!   makes it possible to use an `allocator_api2::alloc::Allocator` as a base allocator via
//!   [`AllocatorApi2V02Compat`](crate::alloc::compat::AllocatorApi2V02Compat).
//! - **`allocator-api2-03`** — Makes `Bump(Scope)` implement `allocator_api2` version `0.3`'s `Allocator` and
//!   makes it possible to use an `allocator_api2::alloc::Allocator` as a base allocator via
//!   [`AllocatorApi2V03Compat`](crate::alloc::compat::AllocatorApi2V03Compat).
//! - **`allocator-api2-04`** — Makes `Bump(Scope)` implement `allocator_api2` version `0.4`'s `Allocator` and
//!   makes it possible to use an `allocator_api2::alloc::Allocator` as a base allocator via
//!   [`AllocatorApi2V04Compat`](crate::alloc::compat::AllocatorApi2V04Compat).
//!
//! ### Nightly features
//! These nightly features are not subject to the same semver guarantees as the rest of the library.
//! Breaking changes to these features might be introduced in minor releases to keep up with changes in the nightly channel.
//!
//! - **`nightly`** — Enables all other nightly feature flags.
//! - **`nightly-allocator-api`** — Makes `Bump(Scope)` implement `alloc`'s `Allocator` and
//!   allows using an `core::alloc::Allocator` as a base allocator via
//!   [`AllocatorNightlyCompat`](crate::alloc::compat::AllocatorNightlyCompat).
//!
//!   This will also enable `allocator-api2` version `0.2`'s `nightly` feature.
//! - **`nightly-coerce-unsized`** — Makes `BumpBox<T>` implement [`CoerceUnsized`](core::ops::CoerceUnsized).
//!   With this `BumpBox<[i32;3]>` coerces to `BumpBox<[i32]>`, `BumpBox<dyn Debug>` and so on.
//!   You can unsize a `BumpBox` in stable without this feature using [`unsize_bump_box`].
//! - **`nightly-exact-size-is-empty`** — Implements `is_empty` manually for some iterators.
//! - **`nightly-trusted-len`** — Implements `TrustedLen` for some iterators.
//! - **`nightly-fn-traits`** — Implements `Fn*` traits for `BumpBox<T>`. Makes `BumpBox<T: FnOnce + ?Sized>` callable. Requires alloc crate.
//! - **`nightly-tests`** — Enables some tests that require a nightly compiler.
//! - **`nightly-dropck-eyepatch`** — Adds `#[may_dangle]` attribute to box and vector types' drop implementation.
//!   This makes it so references don't have to strictly outlive the container.
//!   (Just like with std's `Box` and `Vec`.)
//! - **`nightly-clone-to-uninit`** — Adds [`alloc_clone`](crate::traits::BumpAllocatorTypedScope::alloc_clone) method.
//! <!-- feature documentation end -->
//!
//! [benches]: https://github.com/bluurryy/bump-scope/tree/main/crates/callgrind-benches
//! [`new`]: Bump::new
//! [`with_size`]: Bump::with_size
//! [`with_capacity`]: Bump::with_capacity
//! [`unallocated`]: Bump::unallocated
//! [`scoped`]: crate::traits::BumpAllocator::scoped
//! [`scoped_aligned`]: crate::traits::BumpAllocator::scoped_aligned
//! [`aligned`]: crate::traits::BumpAllocatorScope::aligned
//! [`scope_guard`]: crate::traits::BumpAllocator::scope_guard
//! [`as_guaranteed_allocated`]: Bump::as_guaranteed_allocated
//! [`as_mut_guaranteed_allocated`]: Bump::as_mut_guaranteed_allocated
//! [`into_guaranteed_allocated`]: Bump::into_guaranteed_allocated
//! [`claim`]: crate::traits::BumpAllocatorScope::claim
//! <!-- crate documentation rest end -->

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(any(feature = "alloc", feature = "nightly-fn-traits"))]
extern crate alloc as alloc_crate;

pub mod alloc;
mod allocator_impl;
mod bump;
mod bump_align_guard;
/// Contains [`BumpBox`] and associated types.
mod bump_box;
mod bump_claim_guard;
#[cfg(feature = "std")]
mod bump_pool;
mod bump_scope;
mod bump_scope_guard;
/// Contains [`BumpString`] and associated types.
mod bump_string;
/// Contains [`BumpVec`] and associated types.
pub mod bump_vec;
mod bumping;
mod chunk;
mod destructure;
mod error_behavior;
mod features;
mod fixed_bump_string;
mod fixed_bump_vec;
mod from_utf16_error;
mod from_utf8_error;
mod layout;
mod mut_bump_string;
/// Contains [`MutBumpVec`] and associated types.
pub mod mut_bump_vec;
/// Contains [`MutBumpVecRev`] and associated types.
mod mut_bump_vec_rev;
mod no_drop;
/// Contains types associated with owned slices.
pub mod owned_slice;
/// Contains types associated with owned strings.
pub mod owned_str;
mod partial_eq;
mod polyfill;
mod set_len_on_drop;
mod set_len_on_drop_by_ptr;
pub mod settings;
pub mod stats;
/// Traits that provide ways to be generic over `Bump(Scope)`s.
pub mod traits;
mod without_dealloc;

use alloc::Allocator;
pub use bump::Bump;
pub use bump_box::BumpBox;
pub use bump_claim_guard::BumpClaimGuard;
#[cfg(feature = "std")]
pub use bump_pool::{BumpPool, BumpPoolGuard};
pub use bump_scope::BumpScope;
pub use bump_scope_guard::{BumpScopeGuard, BumpScopeGuardRoot, Checkpoint};
pub use bump_string::BumpString;
#[doc(inline)]
pub use bump_vec::BumpVec;
#[cfg(feature = "panic-on-alloc")]
use core::convert::Infallible;
use core::{mem, num::NonZeroUsize, ptr::NonNull};
use error_behavior::ErrorBehavior;
pub use fixed_bump_string::FixedBumpString;
pub use fixed_bump_vec::FixedBumpVec;
pub use from_utf8_error::FromUtf8Error;
pub use from_utf16_error::FromUtf16Error;
use layout::ArrayLayout;
pub use mut_bump_string::MutBumpString;
#[doc(inline)]
pub use mut_bump_vec::MutBumpVec;
pub use mut_bump_vec_rev::MutBumpVecRev;
pub use no_drop::NoDrop;
#[cfg(feature = "panic-on-alloc")]
use private::{PanicsOnAlloc, capacity_overflow, format_trait_error};
use set_len_on_drop::SetLenOnDrop;
pub use without_dealloc::{WithoutDealloc, WithoutShrink};

#[doc = include_str!("../CHANGELOG.md")]
#[expect(non_snake_case)]
pub mod CHANGELOG {}

#[cfg(feature = "bytemuck")]
features::bytemuck_or_zerocopy! {
    mod bytemuck
    trait Zeroable
}

#[cfg(feature = "zerocopy-08")]
features::bytemuck_or_zerocopy! {
    mod zerocopy_08
    trait FromZeros
}

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

#[cfg(test)]
mod tests;

/// This is not part of the public api!
///
/// Any changes to this module are semver-exempt!
#[doc(hidden)]
pub mod private {
    pub use core;
    use core::ptr::NonNull;

    use crate::BumpBox;

    #[cfg(feature = "panic-on-alloc")]
    /// Wrapper type, used for ad hoc overwriting of trait implementations, like for `Write` in `alloc_fmt`.
    #[repr(transparent)]
    pub struct PanicsOnAlloc<T: ?Sized>(pub T);

    #[cfg(feature = "panic-on-alloc")]
    impl<T: ?Sized> PanicsOnAlloc<T> {
        pub(crate) fn from_mut(value: &mut T) -> &mut PanicsOnAlloc<T> {
            unsafe { &mut *(core::ptr::from_mut::<T>(value) as *mut Self) }
        }
    }

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

    #[must_use]
    #[expect(clippy::elidable_lifetime_names)]
    pub fn bump_box_into_raw_with_lifetime<'a, T: ?Sized>(boxed: BumpBox<'a, T>) -> (NonNull<T>, &'a ()) {
        (boxed.into_raw(), &())
    }

    #[must_use]
    #[expect(clippy::elidable_lifetime_names)]
    pub unsafe fn bump_box_from_raw_with_lifetime<'a, T: ?Sized>(ptr: NonNull<T>, _lifetime: &'a ()) -> BumpBox<'a, T> {
        unsafe { BumpBox::from_raw(ptr) }
    }
}

#[cfg(all(feature = "alloc", feature = "panic-on-alloc"))]
use alloc_crate::alloc::handle_alloc_error;

#[cold]
#[inline(never)]
#[cfg(all(not(feature = "alloc"), feature = "panic-on-alloc"))]
fn handle_alloc_error(_layout: Layout) -> ! {
    panic!("allocation failed")
}

// This is just `Result::into_ok` but with a name to match our use case.
#[inline(always)]
#[cfg(feature = "panic-on-alloc")]
fn panic_on_error<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
    }
}

trait SizedTypeProperties: Sized {
    const SIZE: usize = mem::size_of::<Self>();
    const ALIGN: usize = mem::align_of::<Self>();

    const IS_ZST: bool = mem::size_of::<Self>() == 0;
}

impl<T> SizedTypeProperties for T {}

mod supported_base_allocator {
    use crate::settings::{Boolean, False, True};

    pub trait Sealed<GuaranteedAllocated: Boolean> {
        #[doc(hidden)]
        fn default_or_panic() -> Self;
    }

    impl<A> Sealed<False> for A
    where
        A: Default,
    {
        fn default_or_panic() -> Self {
            A::default()
        }
    }

    impl<A> Sealed<True> for A {
        fn default_or_panic() -> Self {
            unreachable!("default should not be required for `GUARANTEED_ALLOCATED` bump allocators");
        }
    }
}

/// Trait that the base allocator of a `Bump` is required to implement to make allocations.
///
/// Every [`Allocator`] that implements [`Clone`] automatically implements `BaseAllocator` when `GuaranteedAllocated`.
/// When not guaranteed allocated, allocators are additionally required to implement [`Default`].
///
/// This trait is *sealed*: the list of implementors below is total.
pub trait BaseAllocator<GuaranteedAllocated: Boolean>:
    Allocator + Clone + supported_base_allocator::Sealed<GuaranteedAllocated>
{
}

impl<A> BaseAllocator<False> for A where A: Allocator + Clone + Default {}

impl<A> BaseAllocator<True> for A where A: Allocator + Clone {}

/// Call this with a macro that accepts tokens of either `A` or `A = $crate::alloc::Global`.
///
/// We do it this way instead of having a parameter like
/// ```ignore
/// #[cfg(feature = "alloc")] A = $crate::alloc::Global,
/// #[cfg(not(feature = "alloc"))] A,
/// ```
/// because Rust Analyzer thinks those are two parameters and gets confused.
macro_rules! maybe_default_allocator {
    ($macro:ident) => {
        #[cfg(feature = "alloc")]
        $macro!(A = $crate::alloc::Global);

        #[cfg(not(feature = "alloc"))]
        $macro!(A);
    };
}

pub(crate) use maybe_default_allocator;

use crate::settings::{Boolean, False, True};

// (copied from rust standard library)
//
// Tiny Vecs are dumb. Skip to:
// - 8 if the element size is 1, because any heap allocators is likely
//   to round up a request of less than 8 bytes to at least 8 bytes.
// - 4 if elements are moderate-sized (<= 1 KiB).
// - 1 otherwise, to avoid wasting too much space for very short Vecs.
const fn min_non_zero_cap(size: usize) -> usize {
    if size == 1 {
        8
    } else if size <= 1024 {
        4
    } else {
        1
    }
}

/// Aligns a bump address to the `min_align`.
///
/// This is a noop when `min_align` is `0`.
#[must_use]
#[inline(always)]
fn align_pos(up: bool, min_align: usize, pos: NonZeroUsize) -> usize {
    if up {
        // Aligning an address that is `<= range.end` with an alignment
        // that is `<= MIN_CHUNK_ALIGN` cannot exceed `range.end` and
        // cannot overflow as `range.end` is always aligned to `MIN_CHUNK_ALIGN`.
        up_align_usize_unchecked(pos.get(), min_align)
    } else {
        // The chunk start is non-null and is aligned to `MIN_CHUNK_ALIGN`
        // `MIN_ALIGN <= MIN_CHUNK_ALIGN` will never pass the chunk start
        // and stay non-zero.
        down_align_usize(pos.get(), min_align)
    }
}
