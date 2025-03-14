// NB: We avoid using closures to map `Result` and `Option`s in various places because they result in less readable assembly output.
// When using closures, functions like `capacity_overflow` can get the name of some closure that invokes it instead, like `bump_scope::mut_bump_vec::MutBumpVec<T,_,_,A>::generic_grow_amortized::{{closure}}`.

// This crate uses modified code from the rust standard library. <https://github.com/rust-lang/rust/tree/master/library>.
// Especially `BumpBox` methods, vectors, strings, `polyfill` and `tests/from_std` are based on code from the standard library.

#![no_std]
#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api, vec_into_raw_parts))]
#![cfg_attr(feature = "nightly-coerce-unsized", feature(coerce_unsized, unsize))]
#![cfg_attr(feature = "nightly-exact-size-is-empty", feature(exact_size_is_empty))]
#![cfg_attr(feature = "nightly-trusted-len", feature(trusted_len))]
#![cfg_attr(
    test,
    feature(
        exclusive_wrapper,
        pointer_is_aligned_to,
        assert_matches,
        inplace_iteration,
        drain_keep_rest,
        iter_next_chunk,
        iter_advance_by,
        slice_partition_dedup,
        iter_partition_in_place,
        offset_of_enum,
        iter_array_chunks,
    )
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg_hide), doc(cfg_hide(feature = "panic-on-alloc")))]
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
    clippy::literal_string_with_formatting_args, // TODO: this gets triggered, but clippy fails to point out the source location, ...
    rustdoc::redundant_explicit_links, // for cargo-rdme
    unknown_lints, // for `private_bounds` in msrv
    unused_unsafe, // only triggered in old rust versions, like msrv
)]
#![doc(test(
    attr(warn(dead_code, unused_imports)),
    attr(cfg_attr(feature = "nightly-allocator-api", feature(allocator_api, btreemap_alloc)))
))]
//! A fast bump allocator that supports allocation scopes / checkpoints. Aka an arena for values of arbitrary types.
//!
//! # What is bump allocation?
//! A bump allocator owns a big chunk of memory. It has a pointer that starts at one end of that chunk.
//! When an allocation is made that pointer gets aligned and bumped towards the other end of the chunk by the allocation's size.
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
//! - Allocations are a bit more optimized. (see [`bump-scope-inspect-asm/out/x86-64`](https://github.com/bluurryy/bump-scope-inspect-asm/tree/main/out/x86-64) and [benchmarks](https://bluurryy.github.io/bump-scope/criterion/report/))
//! - [You can choose the bump direction.](#bumping-upwards-or-downwards) Bumps upwards by default.
//! - [You can choose the minimum alignment.](#minimum-alignment) `1` by default.
//!
//! # Allocator Methods
//!
//! The bump allocator provides many methods to conveniently allocate values, strings and slices.
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
//! You can also use the unsafe [`checkpoint`](crate::Bump::checkpoint) api to reset the bump pointer to a previous location.
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
//! #### Api changes
//! The collections are designed to have a similar api to their std counterparts but they do make some breaking changes:
//! - [`split_off`](BumpVec::split_off) —  splits the collection in place without allocation; the parameter is a range instead of a single index
//! - [`retain`](BumpVec::retain) —  takes a closure with a `&mut T` parameter like [`Vec::retain_mut`](alloc::vec::Vec::retain_mut)
//!
//! #### New features
//! - [`append`](BumpVec::append) —  allows appending all kinds of owned slice types like `[T; N]`, `Box<[T]>`, `Vec<T>`, `Drain<T>` etc
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
//! `Bump` and `BumpScope` implement `allocator_api2`'s [`Allocator`](https://docs.rs/allocator-api2/0.2.16/allocator_api2/alloc/trait.Allocator.html) trait.
//! They can be used to [allocate collections](crate::Bump#collections).
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
//! use bump_scope::{ Bump, WithoutDealloc };
//! use allocator_api2::boxed::Box;
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
//! ```
//!
//! # Feature Flags
//! * **`std`** *(enabled by default)* —  Adds `BumpPool` and implementations of `std::io` traits.
//! * **`alloc`** *(enabled by default)* —  Adds `Global` as the default base allocator and some interactions with `alloc` collections.
//! * **`panic-on-alloc`** *(enabled by default)* —  Adds functions and traits that will panic when the allocation fails.
//!   Without this feature, allocation failures cannot cause panics, and only
//!   `try_`-prefixed allocation methods will be available.
//! * **`serde`** —  Adds `Serialize` implementations for `BumpBox`, strings and vectors, and `DeserializeSeed` for strings and vectors.
//! * **`zerocopy`** —  Adds `alloc_zeroed(_slice)`, `init_zeroed`, `resize_zeroed` and `extend_zeroed`.
//!
//!  ### Nightly features
//! * **`nightly-allocator-api`** —  Enables `allocator-api2`'s `nightly` feature which makes it reexport the nightly allocator api instead of its own implementation.
//!   With this you can bump allocate collections from the standard library.
//! * **`nightly-coerce-unsized`** —  Makes `BumpBox<T>` implement [`CoerceUnsized`](core::ops::CoerceUnsized).
//!   With this `BumpBox<[i32;3]>` coerces to `BumpBox<[i32]>`, `BumpBox<dyn Debug>` and so on.
//!   You can unsize a `BumpBox` in stable without this feature using [`unsize_bump_box`].
//! * **`nightly-exact-size-is-empty`** —  Implements `is_empty` manually for some iterators.
//! * **`nightly-trusted-len`** —  Implements `TrustedLen` for some iterators.
//!
//! # Bumping upwards or downwards?
//! Bump direction is controlled by the generic parameter `const UP: bool`. By default, `UP` is `true`, so the allocator bumps upwards.
//!
//! Bumping upwards has the advantage that the most recent allocation can be grown and shrunk in place.
//! This benefits collections as well as <code>[alloc_iter](crate::Bump::alloc_iter)([_mut](crate::Bump::alloc_iter_mut))</code> and <code>[alloc_fmt](crate::Bump::alloc_fmt)([_mut](crate::Bump::alloc_fmt_mut))</code>
//! with the exception of [`MutBumpVecRev`](crate::MutBumpVecRev) and [`alloc_iter_mut_rev`](crate::Bump::alloc_iter_mut_rev).
//! [`MutBumpVecRev`](crate::MutBumpVecRev) can be grown and shrunk in place iff bumping downwards.
//!
//! Bumping downwards shaves off a few non-branch instructions per allocation.
//!
//! # Minimum alignment?
//! The minimum alignment is controlled by the generic parameter `const MIN_ALIGN: usize`. By default, `MIN_ALIGN` is `1`.
//!
//! For example changing the minimum alignment to `4` makes it so allocations with the alignment of `4` don't need to align the bump pointer anymore.
//! This will penalize allocations whose sizes are not a multiple of `4` as their size now needs to be rounded up the next multiple of `4`.
//!
//! The overhead of aligning and rounding up is 1 (`UP = false`) or 2 (`UP = true`) non-branch instructions on x86-64.
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

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod allocator;
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
mod doc;
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
/// Types that provide statistics about the memory usage of the bump allocator.
pub mod stats;
mod without_dealloc;

pub use allocator_api2;
#[cfg(all(feature = "alloc", feature = "panic-on-alloc"))]
use allocator_api2::alloc::handle_alloc_error;
use allocator_api2::alloc::{AllocError, Allocator};
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
use chunk_size::ChunkSize;
#[cfg(feature = "panic-on-alloc")]
use core::convert::Infallible;
use core::{
    alloc::Layout,
    ffi::CStr,
    fmt,
    mem::{self, MaybeUninit},
    num::NonZeroUsize,
    ptr::NonNull,
};
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
#[doc(inline)]
pub use stats::Stats;
pub use without_dealloc::{WithoutDealloc, WithoutShrink};

// This must be kept in sync with ChunkHeaders `repr(align(16))`.
const CHUNK_ALIGN_MIN: usize = 16;

const _: () = assert!(CHUNK_ALIGN_MIN == bumping::MIN_CHUNK_ALIGN);

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
const fn up_align_nonzero(addr: NonZeroUsize, align: usize) -> Option<NonZeroUsize> {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    let addr_plus_mask = match addr.checked_add(mask) {
        Some(addr_plus_mask) => addr_plus_mask,
        None => return None,
    };
    let aligned = addr_plus_mask.get() & !mask;
    NonZeroUsize::new(aligned)
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

#[inline(always)]
const unsafe fn assume_unchecked(condition: bool) {
    if !condition {
        core::hint::unreachable_unchecked();
    }
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

#[cold]
#[inline(never)]
#[cfg(all(not(feature = "alloc"), feature = "panic-on-alloc"))]
fn handle_alloc_error(_layout: Layout) -> ! {
    panic!("allocation failed")
}

// this is just `Result::into_ok` but with a name to match our use case
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

    #[cfg(test)]
    const NEEDS_DROP: bool = mem::needs_drop::<Self>();
}

impl<T> SizedTypeProperties for T {}

macro_rules! const_param_assert {
    (
        ($(const $param_ident:ident: $param_ty:ident),+) => $($assert_args:tt)*
    ) => {{
            struct ConstParamAssert<$(const $param_ident: $param_ty),+> {}
            impl<$(const $param_ident: $param_ty),+> ConstParamAssert<$($param_ident),+> {
                #[allow(dead_code)]
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
                pub fn stats(&self) -> Stats<'a, GUARANTEED_ALLOCATED> {
                    let header = self.chunk.get().header_ptr().cast();
                    unsafe { Stats::from_header_unchecked(header) }
                }
            } else {
                /// Returns a type which provides statistics about the memory usage of the bump allocator.
                #[must_use]
                #[inline(always)]
                pub fn stats(&self) -> Stats<GUARANTEED_ALLOCATED> {
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

macro_rules! wrap_result {
    ($ok:ty, $err:ty) => { Result<$ok, $err> };
    (, $err:ty) => { Result<(), $err> };
}

pub(crate) use wrap_result;

macro_rules! error_behavior_generic_methods_if {
    (
        if $fail_if:literal

        $(
            $(#[$attr:meta])*
            $(do panics $(#[doc = $panics:literal])*)?
            $(do errors $(#[doc = $errors:literal])*)?
            $(do examples $(#[doc = $examples:literal])*)?
            impl

            $(#[$attr_infallible:meta])*
            $(do panics $(#[doc = $infallible_panics:literal])*)?
            $(do errors $(#[doc = $infallible_errors:literal])*)?
            $(do examples $(#[doc = $infallible_examples:literal])*)?
            for fn $infallible:ident

            $(#[$attr_fallible:meta])*
            $(do panics $(#[doc = $fallible_panics:literal])*)?
            $(do errors $(#[doc = $fallible_errors:literal])*)?
            $(do examples $(#[doc = $fallible_examples:literal])*)?
            for fn $fallible:ident

            $(#[$attr_generic:meta])*
            use fn $generic:ident
            $(<{$($generic_params:tt)*} $({$($generic_params_lifetime:tt)*})?>)?
            ($(&$($self:ident)+ ,)? $($arg_pat:ident: $arg_ty:ty),* $(,)?)
            $(-> $return_ty:ty)?
            $(where { $($where:tt)* } in)?
            {
                $($body:tt)*
            }
        )*
    ) => {
        $(
            $(#[$attr])*
            $(#[$attr_infallible])*

            /// # Panics
            #[doc = concat!("Panics if ", $fail_if, ".")]
            $(#[doc = "\n"] $(#[doc = $panics])*)?
            $(#[doc = "\n"] $(#[doc = $infallible_panics])*)?

            #[doc = $crate::map!({ $($($errors)*)? $($($infallible_errors)*)? } become { "# Errors" } else { "" })]
            $(#[doc = "\n"] $(#[doc = $errors])*)?
            $(#[doc = "\n"] $(#[doc = $infallible_errors])*)?

            #[doc = $crate::map!({ $($($examples)*)? $($($infallible_examples)*)? } become { "# Examples" } else { "" })]
            $(#[doc = "\n"] $(#[doc = $examples])*)?
            $(#[doc = "\n"] $(#[doc = $infallible_examples])*)?

            #[inline(always)]
            #[cfg(feature = "panic-on-alloc")]
            pub fn $infallible
            $(<$($($generic_params_lifetime)*)? $($generic_params)*>)?
            ($(&$($self)+,)? $($arg_pat: $arg_ty),*)
            $(-> $return_ty)?
            $(where $($where)*)?
            {
                $crate::panic_on_error(Self::$generic($($crate::last!($($self)+), )? $($arg_pat),*))
            }
        )*

        $(
            $(#[$attr])*
            $(#[$attr_fallible])*

            #[doc = $crate::map!({ $($($panics)*)? $($($fallible_panics)*)? } become { "# Panics" } else { "" })]
            $(#[doc = "\n"] $(#[doc = $panics])*)?
            $(#[doc = "\n"] $(#[doc = $fallible_panics])*)?

            /// # Errors
            #[doc = concat!("Errors if ", $fail_if, ".")]
            $(#[doc = "\n"] $(#[doc = $errors])*)?
            $(#[doc = "\n"] $(#[doc = $fallible_errors])*)?

            #[doc = $crate::map!({ $($($examples)*)? $($($fallible_examples)*)? } become { "# Examples" } else { "" })]
            $(#[doc = "\n"] $(#[doc = $examples])*)?
            $(#[doc = "\n"] $(#[doc = $fallible_examples])*)?

            #[inline(always)]
            pub fn $fallible
            $(<$($($generic_params_lifetime)*)? $($generic_params)*>)?
            ($(&$($self)+,)? $($arg_pat: $arg_ty),*)
            -> $crate::wrap_result!($($return_ty)?, allocator_api2::alloc::AllocError)
            $(where $($where)*)?
            {
                Self::$generic($($crate::last!($($self)+), )? $($arg_pat),*)
            }
        )*

        $(
            $(#[$attr])*
            $(#[$attr_generic])*
            pub(crate) fn $generic
            <$($($($generic_params_lifetime)*)?)? B: $crate::ErrorBehavior $(, $($generic_params)*)?>
            ($(&$($self)+,)? $($arg_pat: $arg_ty),*)
            -> $crate::wrap_result!($($return_ty)?, B)
            $(where $($where)*)?
            {
                $($body)*
            }
        )*
    };
}

pub(crate) use error_behavior_generic_methods_if;

macro_rules! error_behavior_generic_methods_allocation_failure {
    ($($tt:tt)*) => {
        $crate::error_behavior_generic_methods_if!(if "the allocation fails" $($tt)*);
    };
}

pub(crate) use error_behavior_generic_methods_allocation_failure;

macro_rules! map {
    ({ } become { $($then:tt)* }) => { };
    ({ } become { $($then:tt)* } else { $($else:tt)* }) => { $($else)* };
    ({ $($from:tt)+ } become { $($then:tt)* }) => { $($then)* };
    ({ $($from:tt)+ } become { $($then:tt)* } else { $($else:tt)* }) => { $($then)* };
}

pub(crate) use map;

macro_rules! last {
    ($self:ident) => {
        $self
    };
    ($mut:ident $self:ident) => {
        $self
    };
}

pub(crate) use last;

macro_rules! as_scope {
    ($self:ident) => {
        $self.as_scope()
    };
    ($mut:ident $self:ident) => {
        $self.as_mut_scope()
    };
}

pub(crate) use as_scope;

macro_rules! define_alloc_methods {
    (
        macro $macro_name:ident

        $(
            $(#[$attr:meta])*
            $(do panics $(#[doc = $panics:literal])*)?
            $(do errors $(#[doc = $errors:literal])*)?
            impl

            $(#[$attr_infallible:meta])*
            $(do panics $(#[doc = $infallible_panics:literal])*)?
            $(do errors $(#[doc = $infallible_errors:literal])*)?
            $(do examples $(#[doc = $infallible_examples:literal])*)?
            for fn $infallible:ident

            $(#[$attr_fallible:meta])*
            $(do panics $(#[doc = $fallible_panics:literal])*)?
            $(do errors $(#[doc = $fallible_errors:literal])*)?
            $(do examples $(#[doc = $fallible_examples:literal])*)?
            for fn $fallible:ident

            use fn $generic:ident
            $(<{$($generic_params:tt)*}>)?
            (&$($self:ident)+ $(, $arg_pat:ident: $arg_ty:ty)* $(,)?)
            $(-> $return_ty:ty | $return_ty_scope:ty)?
            $(where { $($where:tt)* })?;
        )*
    ) => {
        macro_rules! $macro_name {
            (BumpScope) => {
                $(
                    #[cfg(feature = "panic-on-alloc")]
                    $(#[$attr])*
                    $(#[$attr_infallible])*

                    /// # Panics
                    /// Panics if the allocation fails.
                    $(#[doc = "\n"] $(#[doc = $panics])*)?
                    $(#[doc = "\n"] $(#[doc = $infallible_panics])*)?

                    #[doc = $crate::map!({ $($($errors)*)? $($($infallible_errors)*)? } become { "# Errors" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $errors])*)?
                    $(#[doc = "\n"] $(#[doc = $infallible_errors])*)?

                    #[doc = $crate::map!({ $($($infallible_examples)*)? } become { "# Examples" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $infallible_examples])*)?

                    #[inline(always)]
                    pub fn $infallible
                    $(<$($generic_params)*>)?
                    (&$($self)+ $(, $arg_pat: $arg_ty)*)
                    $(-> $return_ty_scope)?
                    $(where $($where)*)?
                    {
                        $crate::panic_on_error($crate::last!($($self)+).$generic($($arg_pat),*))
                    }
                )*

                $(
                    $(#[$attr])*
                    $(#[$attr_fallible])*

                    #[doc = $crate::map!({ $($($panics)*)? $($($fallible_panics)*)? } become { "# Panics" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $panics])*)?
                    $(#[doc = "\n"] $(#[doc = $fallible_panics])*)?

                    /// # Errors
                    /// Errors if the allocation fails.
                    $(#[doc = "\n"] $(#[doc = $errors])*)?
                    $(#[doc = "\n"] $(#[doc = $fallible_errors])*)?

                    #[doc = $crate::map!({ $($($fallible_examples)*)? } become { "# Examples" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $fallible_examples])*)?

                    #[inline(always)]
                    pub fn $fallible
                    $(<$($generic_params)*>)?
                    (&$($self)+ $(, $arg_pat: $arg_ty)*)
                    -> $crate::wrap_result!($($return_ty_scope)?, allocator_api2::alloc::AllocError)
                    $(where $($where)*)?
                    {
                        $crate::last!($($self)+).$generic($($arg_pat),*)
                    }
                )*
            };
            (Bump) => {
                $(
                    $(#[$attr])*
                    $(#[$attr_infallible])*

                    /// # Panics
                    /// Panics if the allocation fails.
                    $(#[doc = "\n"] $(#[doc = $panics])*)?
                    $(#[doc = "\n"] $(#[doc = $infallible_panics])*)?

                    #[doc = $crate::map!({ $($($errors)*)? $($($infallible_errors)*)? } become { "# Errors" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $errors])*)?
                    $(#[doc = "\n"] $(#[doc = $infallible_errors])*)?

                    #[doc = $crate::map!({ $($($infallible_examples)*)? } become { "# Examples" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $infallible_examples])*)?

                    #[inline(always)]
                    #[cfg(feature = "panic-on-alloc")]
                    pub fn $infallible
                    $(<$($generic_params)*>)?
                    (&$($self)+ $(, $arg_pat: $arg_ty)*)
                    $(-> $return_ty)?
                    $(where $($where)*)?
                    {
                        $crate::as_scope!($($self)+).$infallible($($arg_pat),*)
                    }
                )*

                $(
                    $(#[$attr])*
                    $(#[$attr_fallible])*

                    #[doc = $crate::map!({ $($($panics)*)? $($($fallible_panics)*)? } become { "# Panics" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $panics])*)?
                    $(#[doc = "\n"] $(#[doc = $fallible_panics])*)?

                    /// # Errors
                    /// Errors if the allocation fails.
                    $(#[doc = "\n"] $(#[doc = $errors])*)?
                    $(#[doc = "\n"] $(#[doc = $fallible_errors])*)?

                    #[doc = $crate::map!({ $($($fallible_examples)*)? } become { "# Examples" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $fallible_examples])*)?

                    #[inline(always)]
                    pub fn $fallible
                    $(<$($generic_params)*>)?
                    (&$($self)+ $(, $arg_pat: $arg_ty)*)
                    -> $crate::wrap_result!($($return_ty)?, allocator_api2::alloc::AllocError)
                    $(where $($where)*)?
                    {
                        $crate::as_scope!($($self)+).$fallible($($arg_pat),*)
                    }
                )*
            };
        }

        pub(crate) use $macro_name;
    };
}

pub(crate) use define_alloc_methods;

// The implementations of these methods lives in `bump_scope.rs`.
//
// TODO(blocked): once the nightly feature "clone_to_uninit" is stabilized we
// can do away with `alloc_slice_{clone,copy}`, `alloc_str`, `alloc_cstr` and have
// a generic `alloc_clone<T: ?Sized + CloneToUninit>(_: &T)` instead.
define_alloc_methods! {
    macro alloc_methods

    /// Allocate an object.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc(123);
    /// assert_eq!(allocated, 123);
    /// ```
    for fn alloc
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc(123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc
    use fn generic_alloc<{T}>(&self, value: T) -> BumpBox<T> | BumpBox<'a, T>;

    /// Pre-allocate space for an object. Once space is allocated `f` will be called to create the value to be put at that place.
    /// In some situations this can help the compiler realize that `T` can be constructed at the allocated space instead of having to copy it over.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_with(|| 123);
    /// assert_eq!(allocated, 123);
    /// ```
    for fn alloc_with
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_with(|| 123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_with
    use fn generic_alloc_with<{T}>(&self, f: impl FnOnce() -> T) -> BumpBox<T> | BumpBox<'a, T>;

    /// Allocate an object with its default value.
    impl
    /// This is equivalent to <code>[alloc_with](Self::alloc_with)(T::default)</code>.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_default::<i32>();
    /// assert_eq!(allocated, 0);
    /// ```
    for fn alloc_default
    /// This is equivalent to <code>[try_alloc_with](Self::try_alloc_with)(T::default)</code>.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_default()?;
    /// assert_eq!(allocated, 0);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_default
    use fn generic_alloc_default<{T: Default}>(&self) -> BumpBox<T> | BumpBox<'a, T>;

    /// Allocate a slice and `Copy` elements from an existing slice.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_copy(&[1, 2, 3]);
    /// assert_eq!(allocated, [1, 2, 3]);
    /// ```
    for fn alloc_slice_copy
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.alloc_slice_copy(&[1, 2, 3]);
    /// assert_eq!(allocated, [1, 2, 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_slice_copy
    use fn generic_alloc_slice_copy<{T: Copy}>(
        &self,
        slice: &[T],
    ) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocate a slice and `Clone` elements from an existing slice.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_clone(&[String::from("a"), String::from("b")]);
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// ```
    for fn alloc_slice_clone
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_clone(&[String::from("a"), String::from("b")])?;
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_slice_clone
    use fn generic_alloc_slice_clone<{T: Clone}>(
        &self,
        slice: &[T],
    ) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocate a slice and fill it with elements by cloning `value`.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill(3, "ho");
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// ```
    for fn alloc_slice_fill
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_fill(3, "ho")?;
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_slice_fill
    use fn generic_alloc_slice_fill<{T: Clone}>(&self, len: usize, value: T) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    impl
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`alloc_slice_fill`](Self::alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill_with::<i32>(3, Default::default);
    /// assert_eq!(allocated, [0, 0, 0]);
    /// ```
    for fn alloc_slice_fill_with
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`try_alloc_slice_fill`](Self::try_alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_fill_with::<i32>(3, Default::default)?;
    /// assert_eq!(allocated, [0, 0, 0]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_slice_fill_with
    use fn generic_alloc_slice_fill_with<{T}>(&self, len: usize, f: impl FnMut() -> T) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocate a `str`.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_str("Hello, world!");
    /// assert_eq!(allocated, "Hello, world!");
    /// ```
    for fn alloc_str
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_str("Hello, world!")?;
    /// assert_eq!(allocated, "Hello, world!");
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_str
    use fn generic_alloc_str(&self, src: &str) -> BumpBox<str> | BumpBox<'a, str>;

    /// Allocate a `str` from format arguments.
    impl
    #[doc = doc::use_mut_instead!(alloc_fmt_mut)]
    do panics
    /// Panics if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    for fn alloc_fmt
    #[doc = doc::use_mut_instead!(try_alloc_fmt_mut)]
    do errors
    /// Errors if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_fmt
    use fn generic_alloc_fmt(&self, args: fmt::Arguments) -> BumpBox<str> | BumpBox<'a, str>;

    /// Allocate a `str` from format arguments.
    impl
    #[doc = doc::mut_alloc_function!(alloc_fmt, "string buffer")]
    do panics
    /// Panics if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    for fn alloc_fmt_mut
    #[doc = doc::mut_alloc_function!(try_alloc_fmt, "string buffer")]
    do errors
    /// Errors if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_fmt_mut
    use fn generic_alloc_fmt_mut(&mut self, args: fmt::Arguments) -> BumpBox<str> | BumpBox<'a, str>;

    /// Allocate a `CStr`.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr(c"Hello, world!");
    /// assert_eq!(allocated, c"Hello, world!");
    /// ```
    for fn alloc_cstr
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_cstr(c"Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_cstr
    use fn generic_alloc_cstr(&self, src: &CStr) -> &CStr | &'a CStr;

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop there.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr_from_str("Hello, world!");
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.alloc_cstr_from_str("abc\0def");
    /// assert_eq!(allocated, c"abc");
    /// ```
    for fn alloc_cstr_from_str
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_cstr_from_str("Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.try_alloc_cstr_from_str("abc\0def")?;
    /// assert_eq!(allocated, c"abc");
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_cstr_from_str
    use fn generic_alloc_cstr_from_str(&self, src: &str) -> &CStr | &'a CStr;

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop there.
    impl
    #[doc = doc::use_mut_instead!(alloc_cstr_fmt_mut)]
    do panics
    /// Panics if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    for fn alloc_cstr_fmt
    #[doc = doc::use_mut_instead!(try_alloc_cstr_fmt_mut)]
    do errors
    /// Errors if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_cstr_fmt
    use fn generic_alloc_cstr_fmt(&self, args: fmt::Arguments) -> &CStr | &'a CStr;

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop there.
    impl
    #[doc = doc::mut_alloc_function!(alloc_cstr_fmt, "string buffer")]
    do panics
    /// Panics if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt_mut(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    for fn alloc_cstr_fmt_mut
    #[doc = doc::mut_alloc_function!(try_alloc_cstr_fmt, "string buffer")]
    do errors
    /// Errors if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt_mut(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_cstr_fmt_mut
    use fn generic_alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> &CStr | &'a CStr;

    /// Allocate elements of an iterator into a slice.
    impl
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`alloc_iter_mut`].
    ///
    /// [`alloc_iter_exact`]: Self::alloc_iter_exact
    /// [`alloc_iter_mut`]: Self::alloc_iter_mut
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    for fn alloc_iter
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`try_alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`try_alloc_iter_mut`].
    ///
    /// [`try_alloc_iter_exact`]: Self::try_alloc_iter_exact
    /// [`try_alloc_iter_mut`]: Self::try_alloc_iter_mut
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_iter
    use fn generic_alloc_iter<{T}>(&self, iter: impl IntoIterator<Item = T>) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_exact([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    for fn alloc_iter_exact
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_exact([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_iter_exact
    use fn generic_alloc_iter_exact<{T, I}>(&self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> BumpBox<[T]> | BumpBox<'a, [T]>
    where {
        I: ExactSizeIterator<Item = T>
    };

    /// Allocate elements of an iterator into a slice.
    impl
    #[doc = doc::mut_alloc_function!(alloc_iter, "vector")]
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`](Bump::alloc_iter_mut_rev) instead.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    for fn alloc_iter_mut
    #[doc = doc::mut_alloc_function!(try_alloc_iter, "vector")]
    ///
    /// When bumping downwards, prefer [`try_alloc_iter_mut_rev`](Bump::try_alloc_iter_mut_rev) instead.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_mut([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_iter_mut
    use fn generic_alloc_iter_mut<{T}>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocate elements of an iterator into a slice in reverse order.
    ///
    impl
    /// Compared to [`alloc_iter_mut`] this function is more performant
    /// for downwards bumping allocators as the allocation for the vector can be shrunk in place
    /// without any `ptr::copy`.
    ///
    /// The reverse is true when upwards allocating. In that case it's better to use [`alloc_iter_mut`] to prevent
    /// the `ptr::copy`.
    ///
    /// [`alloc_iter_mut`]: Self::alloc_iter_mut
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut_rev([1, 2, 3]);
    /// assert_eq!(slice, [3, 2, 1]);
    /// ```
    for fn alloc_iter_mut_rev
    /// Compared to [`try_alloc_iter_mut`] this function is more performant
    /// for downwards bumping allocators as the allocation for the vector can be shrunk in place
    /// without any `ptr::copy`.
    ///
    /// The reverse is true when upwards allocating. In that case it's better to use [`try_alloc_iter_mut`] to prevent
    /// the `ptr::copy`.
    ///
    /// [`try_alloc_iter_mut`]: Self::try_alloc_iter_mut
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_mut_rev([1, 2, 3])?;
    /// assert_eq!(slice, [3, 2, 1]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_iter_mut_rev
    use fn generic_alloc_iter_mut_rev<{T}>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocate an unitialized object.
    ///
    /// You can safely initialize the object with [`init`](BumpBox::init) or unsafely with [`assume_init`](BumpBox::assume_init).
    impl
    do examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let five = bump.alloc_uninit();
    ///
    /// let five = five.init(5);
    ///
    /// assert_eq!(*five, 5)
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let mut five = bump.alloc_uninit();
    ///
    /// let five = unsafe {
    ///     five.write(5);
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    for fn alloc_uninit
    do examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let five = bump.try_alloc_uninit()?;
    ///
    /// let five = five.init(5);
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut five = bump.try_alloc_uninit()?;
    ///
    /// let five = unsafe {
    ///     five.write(5);
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_uninit
    use fn generic_alloc_uninit<{T}>(&self) -> BumpBox<MaybeUninit<T>> | BumpBox<'a, MaybeUninit<T>>;

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    impl
    do examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let values = bump.alloc_uninit_slice(3);
    ///
    /// let values = values.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(values, [1, 2, 3])
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut values = bump.alloc_uninit_slice(3);
    ///
    /// let values = unsafe {
    ///     values[0].write(1);
    ///     values[1].write(2);
    ///     values[2].write(3);
    ///
    ///     values.assume_init()
    /// };
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// ```
    for fn alloc_uninit_slice
    do examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let values = bump.try_alloc_uninit_slice(3)?;
    ///
    /// let values = values.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut values = bump.try_alloc_uninit_slice(3)?;
    ///
    /// let values = unsafe {
    ///     values[0].write(1);
    ///     values[1].write(2);
    ///     values[2].write(3);
    ///
    ///     values.assume_init()
    /// };
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_uninit_slice
    use fn generic_alloc_uninit_slice<{T}>(&self, len: usize) -> BumpBox<[MaybeUninit<T>]> | BumpBox<'a, [MaybeUninit<T>]>;

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    impl
    /// This is just like [`alloc_uninit_slice`](Self::alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = &[1, 2, 3];
    /// let other_slice = bump.alloc_uninit_slice_for(slice);
    /// assert_eq!(other_slice.len(), 3);
    /// ```
    for fn alloc_uninit_slice_for
    /// This is just like [`try_alloc_uninit_slice`](Self::try_alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = &[1, 2, 3];
    /// let other_slice = bump.try_alloc_uninit_slice_for(slice)?;
    /// assert_eq!(other_slice.len(), 3);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_uninit_slice_for
    use fn generic_alloc_uninit_slice_for<{T}>(&self, slice: &[T]) -> BumpBox<[MaybeUninit<T>]> | BumpBox<'a, [MaybeUninit<T>]>;

    /// Allocate a [`FixedBumpVec`] with the given `capacity`.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut values = bump.alloc_fixed_vec(3);
    /// values.push(1);
    /// values.push(2);
    /// values.push(3);
    /// assert_eq!(values, [1, 2, 3])
    /// ```
    for fn alloc_fixed_vec
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut values = bump.try_alloc_fixed_vec(3)?;
    /// values.push(1);
    /// values.push(2);
    /// values.push(3);
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_fixed_vec
    use fn generic_alloc_fixed_vec<{T}>(&self, capacity: usize) -> FixedBumpVec<T> | FixedBumpVec<'a, T>;

    /// Allocate a [`FixedBumpString`] with the given `capacity` in bytes.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut string = bump.alloc_fixed_string(13);
    /// string.push_str("Hello,");
    /// string.push_str(" world!");
    /// assert_eq!(string, "Hello, world!");
    /// ```
    for fn alloc_fixed_string
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut string = bump.try_alloc_fixed_string(13)?;
    /// string.push_str("Hello,");
    /// string.push_str(" world!");
    /// assert_eq!(string, "Hello, world!");
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_fixed_string
    use fn generic_alloc_fixed_string(&self, capacity: usize) -> FixedBumpString | FixedBumpString<'a>;

    /// Allocates memory as described by the given `Layout`.
    impl
    for fn alloc_layout
    for fn try_alloc_layout
    use fn generic_alloc_layout(&self, layout: Layout) -> NonNull<u8> | NonNull<u8>;

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_bytes`, <code>self.[stats](Self::stats)().[remaining](Stats::remaining)()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    impl
    do examples
    /// ```
    /// # use bump_scope::{ Bump };
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve_bytes(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    for fn reserve_bytes
    do examples
    /// ```
    /// # use bump_scope::{ Bump };
    /// let bump: Bump = Bump::try_new()?;
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.try_reserve_bytes(4096)?;
    /// assert!(bump.stats().capacity() >= 4096);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_reserve_bytes
    use fn generic_reserve_bytes(&self, additional: usize);
}

define_alloc_methods! {
    macro alloc_try_with_methods

    #[allow(clippy::missing_errors_doc)]
    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    impl
    ///
    /// There is also [`alloc_try_with_mut`](Self::alloc_try_with_mut), optimized for a mutable reference.
    do examples
    /// ```
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::{ Bump };
    /// # let bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// ```
    /// ```
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::{ Bump };
    /// # let bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    for fn alloc_try_with
    ///
    /// There is also [`try_alloc_try_with_mut`](Self::try_alloc_try_with_mut), optimized for a mutable reference.
    do examples
    /// ```
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    /// ```
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_try_with
    use fn generic_alloc_try_with<{T, E}>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpBox<T>, E> | Result<BumpBox<'a, T>, E>;

    #[allow(clippy::missing_errors_doc)]
    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    impl
    ///
    /// This is just like [`alloc_try_with`](Self::alloc_try_with), but optimized for a mutable reference.
    do examples
    /// ```
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::{ Bump };
    /// # let mut bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with_mut(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// ```
    /// ```
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::{ Bump };
    /// # let mut bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with_mut(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    for fn alloc_try_with_mut
    ///
    /// This is just like [`try_alloc_try_with`](Self::try_alloc_try_with), but optimized for a mutable reference.
    do examples
    /// ```
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    /// ```
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_try_with_mut
    use fn generic_alloc_try_with_mut<{T, E}>(&mut self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpBox<T>, E> | Result<BumpBox<'a, T>, E>;
}

/// Functions to allocate. Available as fallible or infallible.
impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    alloc_methods!(BumpScope);
}

/// Functions to allocate. Available as fallible or infallible.
impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScope<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    alloc_try_with_methods!(BumpScope);
}

/// Functions to allocate. Available as fallible or infallible.
///
/// These require a [guaranteed allocated](crate#guaranteed_allocated-parameter) bump allocator.
impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    alloc_methods!(Bump);
}

/// Functions to allocate. Available as fallible or infallible.
///
/// These require a [guaranteed allocated](crate#guaranteed_allocated-parameter) bump allocator.
impl<A, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    alloc_try_with_methods!(Bump);
}

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

/// Call this with a macro that accepts tokens of either `A` or `A = allocator_api2::alloc::Global`.
///
/// We do it this way instead of having a parameter like
/// ```ignore
/// #[cfg(feature = "alloc")] A = allocator_api2::alloc::Global,
/// #[cfg(not(feature = "alloc"))] A,
/// ```
/// because Rust Analyzer thinks those are two parameters and gets confused.
macro_rules! maybe_default_allocator {
    ($macro:ident) => {
        #[cfg(feature = "alloc")]
        $macro!(A = allocator_api2::alloc::Global);

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
        pub fn allocator_stats(&self) -> Stats {
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
        pub fn allocator_stats(&self) -> Stats {
            self.allocator.stats()
        }
    };
}

pub(crate) use mut_collection_method_allocator_stats;

/// We don't use `document-features` the usual way because then we can't have our features
/// be copied into the `README.md` via [`cargo-rdme`](https://github.com/orium/cargo-rdme).
#[test]
#[ignore = "this is not a real test, it's just to insert documentation"]
fn insert_feature_docs() {
    use alloc::{format, vec::Vec};

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
