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
#![cfg_attr(docsrs,
    feature(doc_auto_cfg, doc_cfg_hide),
    doc(cfg_hide(feature = "panic-on-alloc")) // too noisy
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
#![allow(
    clippy::inline_always,
    clippy::module_name_repetitions,
    clippy::copy_iterator,
    clippy::comparison_chain,
    clippy::partialeq_ne_impl,
    clippy::collapsible_else_if,
    clippy::items_after_statements,
    clippy::missing_transmute_annotations,
    clippy::manual_assert,
    clippy::range_plus_one,
    rustdoc::redundant_explicit_links, // for cargo-rdme
    unknown_lints, // for `private_bounds` in msrv
    unused_unsafe, // only triggered in old rust versions, like msrv
    clippy::multiple_crate_versions, // we have allocator-api2 version 0.2 and 0.3
    rustdoc::invalid_rust_codeblocks, // for our current workaround to conditionally enable doc tests in macro
)]
#![doc(test(
    attr(deny(dead_code, unused_imports, deprecated)),
    attr(cfg_attr(feature = "nightly-allocator-api", feature(allocator_api, btreemap_alloc))),
))]
//! A fast bump allocator that supports allocation scopes / checkpoints. Aka an arena for values of arbitrary types.
//!
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
//! - Drop is always called for allocated values unless explicitly [leaked](crate::BumpBox::leak) or [forgotten](::core::mem::forget).
//!   - `alloc*` methods return a [`BumpBox<T>`](crate::BumpBox) which owns and drops `T`. Types that don't need dropping can be turned into references with [`into_ref`](crate::BumpBox::into_ref) and [`into_mut`](crate::BumpBox::into_mut).
//! - You can allocate a slice from *any* `Iterator` with [`alloc_iter`](crate::Bump::alloc_iter).
//! - Every method that panics on allocation failure has a fallible `try_*` counterpart.
//! - `Bump`'s base allocator is generic.
//! - Won't try to allocate a smaller chunk if allocation failed.
//! - No built-in allocation limit. You can provide an allocator that enforces an allocation limit (see `tests/limit_memory_usage.rs`).
//! - Allocations are a tiny bit more optimized. See [./crates/callgrind-benches][benches].
//! - [You can choose the bump direction.](#bumping-upwards-or-downwards) Bumps upwards by default.
//!
//! # Allocator Methods
//!
//! The bump allocator provides many methods to conveniently allocate values, strings, and slices.
//! Have a look at the documentation of [`Bump`](crate::Bump) for a method overview.
//!
//! # Scopes and Checkpoints
//!
//! You can create scopes to make allocations that live only for a part of its parent scope.
//! Entering and exiting scopes is virtually free. Allocating within a scope has no overhead.
//!
//! You can create a new scope either with a [`scoped`](crate::Bump::scoped) closure or with a [`scope_guard`](crate::Bump::scope_guard):
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
//! You can also use the unsafe [`checkpoint`](crate::Bump::checkpoint) api
//! to reset the bump pointer to a previous position.
//! ```
//! # use bump_scope::Bump;
//! let mut bump: Bump = Bump::new();
//! let checkpoint = bump.checkpoint();
//!
//! {
//!     let hello = bump.alloc_str("hello");
//!     assert_eq!(bump.stats().allocated(), 5);
//! }
//!
//! unsafe { bump.reset_to(checkpoint); }
//! assert_eq!(bump.stats().allocated(), 0);
//! ```
//!
//! # Collections
//! `bump-scope` provides bump allocated variants of `Vec` and `String` called [`BumpVec`](crate::BumpVec) and [`BumpString`](crate::BumpString).
//! They are also available in the following variants:
//! - [`Fixed*`](crate::FixedBumpVec) for fixed capacity collections
//! - [`Mut*`](crate::MutBumpVec) for collections optimized for a mutable bump allocator
//!
//! #### API changes
//! The collections are designed to have the same api as their std counterparts with these exceptions:
//! - [`split_off`](BumpVec::split_off) —  splits the collection in place without allocation; the parameter is a range instead of a single index
//! - [`retain`](BumpVec::retain) —  takes a closure with a `&mut T` parameter like [`Vec::retain_mut`](alloc_crate::vec::Vec::retain_mut)
//!
//! #### New features
//! - [`append`](BumpVec::append) —  allows appending all kinds of owned slice types like `[T; N]`, `Box<[T]>`, `Vec<T>`, `vec::Drain<T>` etc.
//! - [`map`](BumpVec::map) —  maps the elements, potentially reusing the existing allocation
//! - [`map_in_place`](BumpVec::map_in_place) —  maps the elements without allocation
//! - conversions between the regular collections, their `Fixed*` variants and `BumpBox<[T]>` / `BumpBox<str>`
//!
//! # Parallel Allocation
//! [`Bump`](crate::Bump) is `!Sync` which means it can't be shared between threads.
//!
//! To bump allocate in parallel you can use a [`BumpPool`](crate::BumpPool).
//!
//! # Allocator API
//! `Bump` and `BumpScope` implement `bump-scope`'s own [`Allocator`](crate::alloc::Allocator) trait and with the
//! respective [feature flags](#feature-flags) also implement `allocator_api2@0.2`, `allocator_api2@0.3` and nightly's `Allocator` trait.
//! All of these traits mirror the nightly `Allocator` trait at the time of writing.
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
//! You can wrap a bump allocator in a type that makes `deallocate` and `shrink` a no-op using [`WithoutDealloc`](crate::WithoutDealloc) and [`WithoutShrink`](crate::WithoutShrink).
//! ```
//! # #[cfg(feature = "allocator-api2-03")]
//! # {
//! use bump_scope::{Bump, WithoutDealloc};
//! use allocator_api2_03::boxed::Box;
//!
//! let bump: Bump = Bump::new();
//!
//! let boxed = Box::new_in(5, &bump);
//! assert_eq!(bump.stats().allocated(), 4);
//! drop(boxed);
//! assert_eq!(bump.stats().allocated(), 0);
//!
//! let boxed = Box::new_in(5, WithoutDealloc(&bump));
//! assert_eq!(bump.stats().allocated(), 4);
//! drop(boxed);
//! assert_eq!(bump.stats().allocated(), 4);
//! # }
//! ```
//!
//! # Feature Flags
//! * **`std`** *(enabled by default)* —  Adds `BumpPool` and implementations of `std::io` traits.
//! * **`alloc`** *(enabled by default)* —  Adds `Global` as the default base allocator and some interactions with `alloc` collections.
//! * **`panic-on-alloc`** *(enabled by default)* —  Adds functions and traits that will panic when the allocation fails.
//!   Without this feature, allocation failures cannot cause panics, and only
//!   `try_`-prefixed allocation methods will be available.
//! * **`serde`** —  Adds `Serialize` implementations for `BumpBox`, strings and vectors, and `DeserializeSeed` for strings and vectors.
//! * **`bytemuck`** —  Adds `bytemuck::*` extension traits for `alloc_zeroed(_slice)`, `BumpBox::init_zeroed` and
//!   `resize_zeroed` and `extend_zeroed` for vector types.
//! * **`zerocopy-08`** —  Adds `zerocopy_08::*` extension traits for `alloc_zeroed(_slice)`, `BumpBox::init_zeroed` and
//!   `resize_zeroed` and `extend_zeroed` for vector types.
//! * **`allocator-api2-02`** —  Makes `Bump(Scope)` implement `allocator_api2` version `0.2`'s `Allocator` and
//!   makes it possible to use an `allocator_api2::alloc::Allocator` as a base allocator via
//!   [`AllocatorApiV02Compat`](crate::alloc::compat::AllocatorApi2V02Compat).
//! * **`allocator-api2-03`** —  Makes `Bump(Scope)` implement `allocator_api2` version `0.3`'s `Allocator` and
//!   makes it possible to use an `allocator_api2::alloc::Allocator` as a base allocator via
//!   [`AllocatorApiV03Compat`](crate::alloc::compat::AllocatorApi2V03Compat).
//!
//!  ### Nightly features
//!  These nightly features are not subject to the same semver guarantees as the rest of the library.
//!  Breaking changes to these features might be introduced in minor releases to keep up with changes in the nightly channel.
//! * **`nightly`** —  Enables all other nightly feature flags.
//! * **`nightly-allocator-api`** —  Makes `Bump(Scope)` implement `alloc`'s `Allocator` and
//!   allows using an `alloc::alloc::Allocator` as a base allocator via
//!   [`AllocatorNightlyCompat`](crate::alloc::compat::AllocatorNightlyCompat).
//!  
//!   This will also enable `allocator-api2` version `0.2`'s `nightly` feature.
//! * **`nightly-coerce-unsized`** —  Makes `BumpBox<T>` implement [`CoerceUnsized`](core::ops::CoerceUnsized).
//!   With this `BumpBox<[i32;3]>` coerces to `BumpBox<[i32]>`, `BumpBox<dyn Debug>` and so on.
//!   You can unsize a `BumpBox` in stable without this feature using [`unsize_bump_box`](crate::unsize_bump_box).
//! * **`nightly-exact-size-is-empty`** —  Implements `is_empty` manually for some iterators.
//! * **`nightly-trusted-len`** —  Implements `TrustedLen` for some iterators.
//! * **`nightly-fn-traits`** —  Implements `Fn*` traits for `BumpBox<T>`. Makes `BumpBox<T: FnOnce + ?Sized>` callable. Requires alloc crate.
//! * **`nightly-tests`** —  Enables some tests that require a nightly compiler.
//! * **`nightly-dropck-eyepatch`** —  Adds `#[may_dangle]` attribute to box and vector types' drop implementation.
//!   This makes it so references don't have to strictly outlive the container.
//!   (That's how std's `Box` and `Vec` work.)
//!
//! # Bumping upwards or downwards?
//! Bump direction is controlled by the generic parameter `const UP: bool`. By default, `UP` is `true`, so the allocator bumps upwards.
//!
//! Bumping upwards has the advantage that the most recent allocation can be grown and shrunk in place.
//! This benefits collections as well as <code>[alloc_iter](crate::Bump::alloc_iter)([_mut](crate::Bump::alloc_iter_mut))</code> and <code>[alloc_fmt](crate::Bump::alloc_fmt)([_mut](crate::Bump::alloc_fmt_mut))</code>
//! with the exception of [`MutBumpVecRev`](crate::MutBumpVecRev) and [`alloc_iter_mut_rev`](crate::Bump::alloc_iter_mut_rev).
//! [`MutBumpVecRev`](crate::MutBumpVecRev) can be grown and shrunk in place if and only if bumping downwards.
//!
//! For the performance impact see [./crates/callgrind-benches][benches].
//!
//! # Minimum alignment?
//! The minimum alignment is controlled by the generic parameter `const MIN_ALIGN: usize`. By default, `MIN_ALIGN` is `1`.
//!
//! For example changing the minimum alignment to `4` makes it so allocations with the alignment of `4` don't need to align the bump pointer anymore.
//! This will penalize allocations whose sizes are not a multiple of `4` as their size now needs to be rounded up the next multiple of `4`.
//!
//! For the performance impact see [./crates/callgrind-benches][benches].
//!
//! # `GUARANTEED_ALLOCATED` parameter?
//! If `GUARANTEED_ALLOCATED` is `true` then the bump allocator is guaranteed to have at least one allocated chunk.
//! This is usually the case unless it was created with [`Bump::unallocated`](crate::Bump::unallocated).
//!
//! You need a guaranteed allocated `Bump(Scope)` to create scopes via `scoped` and `scope_guard`.
//! You can make a `Bump(Scope)` guaranteed allocated using
//! <code>[guaranteed_allocated](Bump::guaranteed_allocated)([_ref](Bump::guaranteed_allocated_ref)/[_mut](Bump::guaranteed_allocated_mut))</code>.
//!
//! The point of this is so `Bump`s can be created without allocating memory and even `const` constructed since rust version 1.83.
//! At the same time `Bump`s that have already allocated a chunk don't suffer runtime checks for entering scopes and creating checkpoints.
//!
//! [benches]: https://github.com/bluurryy/bump-scope/tree/main/crates/callgrind-benches

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(any(feature = "alloc", feature = "nightly-fn-traits"))]
extern crate alloc as alloc_crate;

pub mod alloc;
mod allocator_impl;
mod bump;
mod bump_align_guard;
mod bump_allocator;
mod bump_allocator_scope;
/// Contains [`BumpBox`] and associated types.
mod bump_box;
#[cfg(feature = "std")]
mod bump_pool;
mod bump_scope;
mod bump_scope_guard;
/// Contains [`BumpString`] and associated types.
mod bump_string;
/// Contains [`BumpVec`] and associated types.
pub mod bump_vec;
mod bumping;
mod chunk_size;
mod destructure;
mod error_behavior;
mod features;
mod fixed_bump_string;
mod fixed_bump_vec;
mod from_utf16_error;
mod from_utf8_error;
mod layout;
mod mut_bump_allocator;
mod mut_bump_allocator_scope;
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
mod raw_bump_box;
mod raw_chunk;
mod raw_fixed_bump_string;
mod raw_fixed_bump_vec;
mod set_len_on_drop;
mod set_len_on_drop_by_ptr;
pub mod stats;
mod without_dealloc;

use alloc::Allocator;
pub use bump::Bump;
pub use bump_allocator::BumpAllocator;
pub use bump_allocator_scope::BumpAllocatorScope;
pub use bump_box::BumpBox;
#[cfg(feature = "std")]
pub use bump_pool::{BumpPool, BumpPoolGuard};
pub use bump_scope::BumpScope;
pub use bump_scope_guard::{BumpScopeGuard, BumpScopeGuardRoot, Checkpoint};
pub use bump_string::BumpString;
#[doc(inline)]
pub use bump_vec::BumpVec;
use chunk_header::{unallocated_chunk_header, ChunkHeader};
#[cfg(feature = "panic-on-alloc")]
use core::convert::Infallible;
use core::{mem, num::NonZeroUsize, ptr::NonNull};
use error_behavior::ErrorBehavior;
pub use fixed_bump_string::FixedBumpString;
pub use fixed_bump_vec::FixedBumpVec;
pub use from_utf16_error::FromUtf16Error;
pub use from_utf8_error::FromUtf8Error;
use layout::ArrayLayout;
pub use mut_bump_allocator::MutBumpAllocator;
pub use mut_bump_allocator_scope::MutBumpAllocatorScope;
pub use mut_bump_string::MutBumpString;
#[doc(inline)]
pub use mut_bump_vec::MutBumpVec;
pub use mut_bump_vec_rev::MutBumpVecRev;
pub use no_drop::NoDrop;
#[cfg(feature = "panic-on-alloc")]
use private::{capacity_overflow, format_trait_error, PanicsOnAlloc};
use raw_chunk::RawChunk;
use set_len_on_drop::SetLenOnDrop;
pub use without_dealloc::{WithoutDealloc, WithoutShrink};

#[cfg(feature = "bytemuck")]
/// Contains extension traits.
pub mod bytemuck {
    pub use crate::features::bytemuck::{BumpExt, BumpScopeExt, InitZeroed, VecExt};
}

#[cfg(feature = "zerocopy-08")]
/// Contains extension traits.
pub mod zerocopy_08 {
    pub use crate::features::zerocopy_08::{BumpExt, BumpScopeExt, InitZeroed, VecExt};
}

/// Specifies the current minimum alignment of a bump allocator.
#[derive(Clone, Copy)]
pub struct MinimumAlignment<const ALIGNMENT: usize>;

mod supported_minimum_alignment {
    use crate::ArrayLayout;

    pub trait Sealed {
        /// We'd be fine with just an [`core::ptr::Alignment`], but that's not stable.
        #[doc(hidden)]
        const LAYOUT: ArrayLayout;

        #[doc(hidden)]
        const MIN_ALIGN: usize;
    }
}

/// Statically guarantees that a minimum alignment is marked as supported.
///
/// This trait is *sealed*: the list of implementors below is total. Users do not have the ability to mark additional
/// `MinimumAlignment<N>` values as supported. Only bump allocators with the supported minimum alignments are constructable.
#[allow(private_bounds)]
pub trait SupportedMinimumAlignment: supported_minimum_alignment::Sealed + Copy {}

macro_rules! supported_alignments {
    ($($i:literal)*) => {
        $(
            impl supported_minimum_alignment::Sealed for MinimumAlignment<$i> {
                const LAYOUT: ArrayLayout = match ArrayLayout::from_size_align(0, $i) {
                    Ok(layout) => layout,
                    Err(_) => unreachable!(),
                };

                const MIN_ALIGN: usize = $i;
            }
            impl SupportedMinimumAlignment for MinimumAlignment<$i> {}
        )*
    };
}

supported_alignments!(1 2 4 8 16);

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

mod chunk_header;
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
            unsafe { &mut *(value as *mut T as *mut Self) }
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
    #[allow(clippy::needless_lifetimes, clippy::elidable_lifetime_names)]
    pub fn bump_box_into_raw_with_lifetime<'a, T: ?Sized>(boxed: BumpBox<'a, T>) -> (NonNull<T>, &'a ()) {
        (boxed.into_raw(), &())
    }

    #[must_use]
    #[allow(clippy::needless_lifetimes, clippy::elidable_lifetime_names)]
    pub unsafe fn bump_box_from_raw_with_lifetime<'a, T: ?Sized>(ptr: NonNull<T>, _lifetime: &'a ()) -> BumpBox<'a, T> {
        BumpBox::from_raw(ptr)
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
#[allow(unreachable_patterns)] // msrv 1.64.0 does not allow omitting the `Err` arm
fn panic_on_error<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => unreachable!(),
    }
}

trait SizedTypeProperties: Sized {
    const SIZE: usize = mem::size_of::<Self>();
    const ALIGN: usize = mem::align_of::<Self>();

    const IS_ZST: bool = mem::size_of::<Self>() == 0;
}

impl<T> SizedTypeProperties for T {}

macro_rules! const_param_assert {
    (
        ($(const $param_ident:ident: $param_ty:ident),+) => $($assert_args:tt)*
    ) => {{
            struct ConstParamAssert<$(const $param_ident: $param_ty),+> {}
            impl<$(const $param_ident: $param_ty),+> ConstParamAssert<$($param_ident),+> {
                const CONST_PARAM_ASSERT: () = assert!($($assert_args)*);
            }
            #[allow(unused_variables)]
            let assertion = ConstParamAssert::<$($param_ident),+>::CONST_PARAM_ASSERT;
    }};
}

pub(crate) use const_param_assert;

macro_rules! condition {
    (if true { $($then:tt)* } else { $($else:tt)* }) => { $($then)* };
    (if false { $($then:tt)* } else { $($else:tt)* }) => { $($else)* };
}

pub(crate) use condition;

macro_rules! bump_common_methods {
    ($is_scope:ident) => {
        #[inline(always)]
        pub(crate) fn is_unallocated(&self) -> bool {
            !GUARANTEED_ALLOCATED && self.chunk.get().is_unallocated()
        }

        $crate::condition! {
            if $is_scope {
                /// Returns a type which provides statistics about the memory usage of the bump allocator.
                #[must_use]
                #[inline(always)]
                pub fn stats(&self) -> Stats<'a, A, UP, GUARANTEED_ALLOCATED> {
                    let header = self.chunk.get().header_ptr().cast();
                    unsafe { Stats::from_header_unchecked(header) }
                }
            } else {
                /// Returns a type which provides statistics about the memory usage of the bump allocator.
                #[must_use]
                #[inline(always)]
                pub fn stats(&self) -> Stats<'_, A, UP, GUARANTEED_ALLOCATED> {
                    let header = self.chunk.get().header_ptr().cast();
                    unsafe { Stats::from_header_unchecked(header) }
                }
            }
        }

        /// Returns a reference to the base allocator.
        #[must_use]
        #[inline(always)]
        pub fn allocator(&self) -> &A {
            unsafe { self.chunk.get().allocator().as_ref() }
        }
    };
}

pub(crate) use bump_common_methods;

mod supported_base_allocator {
    pub trait Sealed<const GUARANTEED_ALLOCATED: bool> {
        #[doc(hidden)]
        fn default_or_panic() -> Self;
    }

    impl<A> Sealed<false> for A
    where
        A: Default,
    {
        fn default_or_panic() -> Self {
            A::default()
        }
    }

    impl<A> Sealed<true> for A {
        fn default_or_panic() -> Self {
            unreachable!("default should not be required for `GUARANTEED_ALLOCATED` bump allocators");
        }
    }
}

/// Trait that any allocator used as a base allocator of a bump allocator needs to implement.
///
/// Every [`Allocator`] that implements [`Clone`] automatically implements `BaseAllocator` when `GUARANTEED_ALLOCATED`.
/// When not guaranteed allocated, allocators are additionally required to implement [`Default`].
///
/// This trait is *sealed*: the list of implementors below is total.
pub trait BaseAllocator<const GUARANTEED_ALLOCATED: bool = true>:
    Allocator + Clone + supported_base_allocator::Sealed<GUARANTEED_ALLOCATED>
{
}

impl<A> BaseAllocator<false> for A where A: Allocator + Clone + Default {}

impl<A> BaseAllocator<true> for A where A: Allocator + Clone {}

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

macro_rules! collection_method_allocator_stats {
    () => {
        /// Returns a type which provides statistics about the memory usage of the bump allocator.
        ///
        /// This is equivalent to calling `.allocator().stats()`.
        /// This merely exists for api parity with `Mut*` collections which can't have a `allocator` method.
        #[must_use]
        #[inline(always)]
        pub fn allocator_stats(&self) -> $crate::stats::AnyStats<'_> {
            self.allocator.stats()
        }
    };
}

pub(crate) use collection_method_allocator_stats;

macro_rules! mut_collection_method_allocator_stats {
    () => {
        /// Returns a type which provides statistics about the memory usage of the bump allocator.
        ///
        /// This collection does not update the bump pointer, so it also doesn't contribute to the `remaining` and `allocated` stats.
        #[must_use]
        #[inline(always)]
        pub fn allocator_stats(&self) -> $crate::stats::AnyStats<'_> {
            self.allocator.stats()
        }
    };
}

pub(crate) use mut_collection_method_allocator_stats;

/// We don't use `document-features` the usual way because then we can't have our features
/// be copied into the `README.md` via [`cargo-rdme`](https://github.com/orium/cargo-rdme).
#[test]
#[ignore = "this is not a real test, it's just to insert documentation"]
#[cfg(feature = "alloc")]
fn insert_feature_docs() {
    use alloc_crate::{format, vec::Vec};

    let lib_rs = std::fs::read_to_string("src/lib.rs").unwrap();

    let start_marker = "//! # Feature Flags";
    let end_marker = "//! # ";

    let start_index = lib_rs.find(start_marker).unwrap() + start_marker.len();
    let end_index = lib_rs[start_index..].find(end_marker).unwrap() + start_index;

    let before = &lib_rs[..start_index];
    let after = &lib_rs[end_index..];

    let features = document_features::document_features!()
        .lines()
        .map(|line| format!("//! {line}"))
        .collect::<Vec<_>>()
        .join("\n");

    let new_lib_rs = format!("{before}\n{features}\n//!\n{after}");
    std::fs::write("src/lib.rs", new_lib_rs).unwrap();
}
