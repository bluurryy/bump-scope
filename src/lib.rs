// NB: We avoid using closures to map `Result` and `Option`s in various places because they result in less readable assembly output.
// When using closures, functions like `capacity_overflow` can get the name of some closure that invokes it instead, like `bump_scope::bump_vec::BumpVec<T,_,_,A>::generic_grow_cold::{{closure}}`.

// This crate uses modified code from the rust standard library. <https://github.com/rust-lang/rust/tree/master/library>.
// Especially `BumpVec(Rev)`, `BumpString`, `polyfill` and `tests/from_std` are based on code from the standard library.

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api, vec_into_raw_parts))]
#![cfg_attr(feature = "nightly-coerce-unsized", feature(coerce_unsized, unsize))]
#![cfg_attr(feature = "nightly-exact-size-is-empty", feature(exact_size_is_empty))]
#![cfg_attr(feature = "nightly-trusted-len", feature(trusted_len))]
#![cfg_attr(
    test,
    feature(
        exclusive_wrapper,
        pointer_is_aligned,
        assert_matches,
        inplace_iteration,
        drain_keep_rest,
        iter_next_chunk,
        iter_advance_by,
        extract_if,
        slice_flatten,
        slice_partition_dedup,
        iter_partition_in_place,
        strict_provenance,
    )
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg_hide), doc(cfg_hide(no_global_oom_handling)))]
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
    unknown_lints,
    unused_unsafe
)]
//! A fast bump allocator that supports allocation scopes / checkpoints. Aka an arena for values of arbitrary types.
//!
//! # What is bump allocation?
//! A bump allocator owns a big chunk of memory. It has a pointer that starts at one end of that chunk.
//! When an allocation is made that pointer gets aligned and bumped towards the other end of the chunk by the allocation's size.
//! When its chunk is full, it allocates another chunk with twice the size.
//!
//! This makes allocations very fast. The drawback is that you can't reclaim memory like you do with a more general allocator.
//! Memory for the most recent allocation *can* be reclaimed. You can also use [scopes, checkpoints](#scopes-and-checkpoints) and `reset` to reclaim memory.
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
//! The fact that the bump allocator allocates ever larger chunks and `reset` only keeps around the largest one means that after a few iterations, every bump allocation
//! will be done on the same chunk and no more chunks need to be allocated.
//!
//! The introduction of scopes makes this bump allocator also great for temporary allocations and stack-like usage.
//!
//! # Comparison to [`bumpalo`](https://docs.rs/bumpalo)
//!
//! Bumpalo is a popular crate for bump allocation. This crate was inspired by bumpalo and [Always Bump Downwards](https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html).
//!
//! Unlike `bumpalo`, this crate...
//! - Supports [scopes and checkpoints](#scopes-and-checkpoints).
//! - Drop is always called for allocated values unless explicitly [leaked](BumpBox::leak) or [forgotten](core::mem::forget).
//!   - `alloc*` methods return a [`BumpBox<T>`](BumpBox) which owns and drops `T`. Types that don't need dropping can be turned into references with [`into_ref`](BumpBox::into_ref) and [`into_mut`](BumpBox::into_mut).
//! - You can efficiently allocate items from *any* `Iterator` with [`alloc_iter_mut`](Bump::alloc_iter_mut)([`_rev`](Bump::alloc_iter_mut_rev)).
//! - Every method that panics on allocation failure has a fallible `try_*` counterpart.
//! - `Bump`'s base allocator is generic.
//! - `Bump` needs to allocate on construction.
//! - `Bump` and `BumpScope` have the same repr as `NonNull<u8>`. (vs 3x pointer sized)
//! - Won't try to allocate a smaller chunk if allocation failed.
//! - No built-in allocation limit. You can provide an allocator that enforces an allocation limit (see `tests/limit_memory_usage.rs`).
//! - Allocations are a bit more optimized. (see `crates/inspect-asm/out/x86-64`)
//! - [You can choose the bump direction.](#bumping-upwards-or-downwards) Bumps upwards by default.
//! - [You can choose the minimum alignment.](#minimum-alignment)
//!
//! # Scopes and Checkpoints
//! You can create scopes to make allocations that live only for a part of its parent scope.
//! Creating and exiting scopes is virtually free. Allocating within a scope has no overhead.
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
//! You can also use the unsafe [`checkpoint`](Bump::checkpoint) api to reset the bump pointer to a previous location.
//! ```
//! # use bump_scope::Bump;
//! # let mut bump: Bump = Bump::new();
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
//! # Allocator API
//! `Bump` and `BumpScope` implement [`allocator_api2::alloc::Allocator`].
//! With this you can bump allocate [`allocator_api2::boxed::Box`], [`allocator_api2::vec::Vec`] and collections
//! from other crates that support it like [`hashbrown::HashMap`](https://docs.rs/hashbrown/latest/hashbrown/struct.HashMap.html).
//!
//! A bump allocator can grow, shrink and deallocate the most recent allocation.
//! When bumping upwards it can even do so in place.
//! Growing other allocations will require a new allocation and the old memory block becomes wasted space.
//! Shrinking or deallocating other allocations does nothing which means wasted space.
//!
//! A bump allocator does not *require* `deallocate` or `shrink` to free memory.
//! After all, memory will be reclaimed when exiting a scope or calling `reset`.
//! You can wrap a bump allocator in a type that makes `deallocate` and `shrink` a no-op using [`without_dealloc`](Bump::without_dealloc) and [`without_shrink`](Bump::without_shrink).
//! ```
//! # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
//! use bump_scope::Bump;
//! use allocator_api2::boxed::Box;
//! let bump: Bump = Bump::new();
//!
//! let boxed = Box::new_in(5, &bump);
//! assert_eq!(bump.stats().allocated(), 4);
//! drop(boxed);
//! assert_eq!(bump.stats().allocated(), 0);
//!
//! let boxed = Box::new_in(5, bump.without_dealloc());
//! assert_eq!(bump.stats().allocated(), 4);
//! drop(boxed);
//! assert_eq!(bump.stats().allocated(), 4);
//! ```
//!
//! # Feature Flags
//! This crate supports `no_std`, unless the `std` feature is enabled.
//!
//! - `std` *(default)*:
//!
//!   Adds implementations of `std::io` traits for `BumpBox` and `(Fixed)BumpVec`. Activates `alloc` feature.
//!
//! <p></p>
//!
//! - `alloc` *(default)*:
//!
//!   Adds implementations interacting with `String` and `Cow<str>`. Is required for `alloc_iter` and `alloc_fmt`.
//!
//! <p></p>
//!
//! - `nightly-allocator-api` *(requires nightly)*:
//!
//!   Enables `allocator-api2`'s `nightly` feature which makes it reexport the nightly allocator api instead of its own implementation.
//!   With this you can bump allocate collections from the standard library.
//!
//! <p></p>
//!
//! - `nightly-coerce-unsized` *(requires nightly)*:
//!   
//!   Makes `BumpBox<T>` implement [`CoerceUnsized`](core::ops::CoerceUnsized).
//!   With this `BumpBox<[i32;3]>` coerces to `BumpBox<[i32]>`, `BumpBox<dyn Debug>` and so on.
//!
//! # Bumping upwards or downwards?
//! Bump direction is controlled by the generic parameter `const UP: bool`. By default, `UP` is `true`, so the allocator bumps upwards.
//!
//! - Bumping upwards...
//!   - has the advantage that the most recent allocation can be grown and shrunk in place.
//!   - makes [`alloc_iter(_mut)`](Bump::alloc_iter) and [`alloc_fmt(_mut)`](Bump::alloc_fmt) faster.
//! - Bumping downwards...
//!   - uses slightly fewer instructions per allocation.
//!   - makes [`alloc_iter_mut_rev`](Bump::alloc_iter_mut_rev) faster.
//!
//! # Minimum alignment?
//! The minimum alignment is controlled by the generic parameter `const MIN_ALIGN: usize`. By default, `MIN_ALIGN` is `1`.
//!
//! Changing the minimum alignment to e.g. `4` makes it so allocations with the alignment of `4` don't need to align the bump pointer anymore.
//! This will penalize allocations of a smaller alignment as their size now needs to be rounded up the next multiple of `4`.
//!
//! This amounts to about 1 or 2 instructions per allocation.

#[doc(hidden)]
#[cfg(feature = "alloc")]
extern crate alloc;

use core::{
    convert::Infallible,
    fmt::{self, Debug},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    num::NonZeroUsize,
    ptr::NonNull,
};

mod allocator;
#[cfg(test)]
pub(crate) mod any_bump;
mod bump;
mod bump_align_guard;

/// Contains [`BumpBox`] and associated types.
mod bump_box;
mod bump_scope;
mod bump_scope_guard;
mod bump_string;

/// Contains [`BumpVec`] and associated types.
mod bump_vec;

mod array_layout;
/// Contains [`BumpVecRev`] and associated types.
mod bump_vec_rev;
mod chunk_raw;
mod chunk_size;
mod drain;
mod extract_if;
mod fixed_bump_vec;
mod from_utf8_error;
mod into_iter;
mod polyfill;
mod set_len_on_drop;
mod set_len_on_drop_by_ptr;
mod stats;
#[cfg(test)]
mod with_drop;
mod without_dealloc;

use allocator_api2::alloc::{AllocError, Allocator};

#[cfg(feature = "alloc")]
use allocator_api2::alloc::handle_alloc_error;

pub use allocator_api2;
#[cfg(test)]
pub use any_bump::AnyBump;
pub use bump::Bump;
pub use bump_box::BumpBox;
pub use bump_scope::BumpScope;
pub use bump_scope_guard::{BumpScopeGuard, BumpScopeGuardRoot, Checkpoint};
pub use bump_string::BumpString;
pub use bump_vec::BumpVec;
pub use bump_vec_rev::BumpVecRev;
pub use drain::Drain;
pub use extract_if::ExtractIf;
pub use fixed_bump_vec::FixedBumpVec;
pub use from_utf8_error::FromUtf8Error;
pub use into_iter::IntoIter;
pub use stats::{Chunk, ChunkNextIter, ChunkPrevIter, Stats};
#[cfg(test)]
pub use with_drop::WithDrop;
pub use without_dealloc::{WithoutDealloc, WithoutShrink};

use array_layout::{ArrayLayout, LayoutTrait};
use chunk_header::ChunkHeader;
use chunk_raw::RawChunk;
use chunk_size::ChunkSize;
use core::alloc::Layout;
use polyfill::{nonnull, pointer};
use set_len_on_drop::SetLenOnDrop;
use set_len_on_drop_by_ptr::SetLenOnDropByPtr;

// This must be kept in sync with ChunkHeaders `repr(align(16))`.
const CHUNK_ALIGN_MIN: usize = 16;

/// This trait marks types that don't need dropping.
///
/// It is used as a bound for [`BumpBox`]'s [`into_ref`](BumpBox::into_ref) and [`into_mut`](BumpBox::into_mut) so you don't accidentally omit a drop that does matter.
pub trait NoDrop {}

impl NoDrop for str {}
impl<T: Copy> NoDrop for T {}
impl<T: Copy> NoDrop for [T] {}

/// Specifies the current minimum alignment of a bump allocator.
pub struct MinimumAlignment<const ALIGNMENT: usize>;

mod supported_minimum_alignment {
    use crate::ArrayLayout;

    pub trait Sealed {
        /// We'd be fine with just an [`core::ptr::Alignment`], but that's not stable.
        const LAYOUT: ArrayLayout;
    }
}

/// Statically guarantees that a minimum alignment is marked as supported.
///
/// This trait is *sealed*: the list of implementors below is total. Users do not have the ability to mark additional
/// `MinimumAlignment<N>` values as supported. Only bump allocators with the supported minimum alignments are constructable.
#[allow(private_bounds)]
pub trait SupportedMinimumAlignment: supported_minimum_alignment::Sealed {}

macro_rules! supported_alignments {
    ($($i:literal)*) => {
        $(
            impl supported_minimum_alignment::Sealed for MinimumAlignment<$i> {
                const LAYOUT: ArrayLayout = match ArrayLayout::from_size_align(0, $i) {
                    Ok(layout) => layout,
                    Err(_) => unreachable!(),
                };
            }
            impl SupportedMinimumAlignment for MinimumAlignment<$i> {}
        )*
    };
}

supported_alignments!(1 2 4 8 16);

// TODO: this returns `None` when addr is 0; that's very unintuitive; why did i do this?; is this a way to guard overflows?
#[inline(always)]
fn up_align_usize(addr: usize, align: NonZeroUsize) -> Option<NonZeroUsize> {
    debug_assert!(align.get().is_power_of_two());
    let mask = align.get() - 1;
    let aligned = addr.checked_add(mask)? & !mask;
    NonZeroUsize::new(aligned)
}

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
unsafe fn up_align_nonzero_unchecked(addr: NonZeroUsize, align: usize) -> NonZeroUsize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    let aligned = (addr.get() + mask) & !mask;
    NonZeroUsize::new_unchecked(aligned)
}

#[inline(always)]
fn down_align_usize(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}

#[inline(always)]
const unsafe fn assume_unchecked(condition: bool) {
    if !condition {
        core::hint::unreachable_unchecked();
    }
}

struct FmtFn<F>(F)
where
    F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result;

impl<F> Debug for FmtFn<F>
where
    F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0(f)
    }
}

mod chunk_header;
#[cfg(test)]
mod tests;

/// This is not part of the public api!
#[doc(hidden)]
pub mod private {
    pub use core;

    #[inline(never)]
    #[cold]
    pub const fn capacity_overflow() -> ! {
        panic!("capacity overflow");
    }
}

use private::capacity_overflow;

#[cold]
#[inline(never)]
fn exact_size_iterator_bad_len() -> ! {
    panic!("ExactSizeIterator did not return as many items as promised")
}

macro_rules! doc_fn_stats {
    () => {
        "Returns `Stats`, which provides statistics about the memory usage of the bump allocator."
    };
}

macro_rules! doc_fn_stats_greedy {
    ($name:ident) => {
        concat!("\n\n`", stringify!($name), "` does not update the bump pointer until it has been turned into a slice, so it also doesn't contribute to the `remaining` and `allocated` stats.")
    };
}

macro_rules! doc_fn_allocator {
    () => {
        "Returns a reference to the underlying allocator."
    };
}

macro_rules! doc_fn_reset {
    () => {
        "This will only keep around the newest chunk, which is also the biggest."
    };
}

macro_rules! doc_fn_scope {
    () => {
        "Returns a new `BumpScope`."
    };
}

pub(crate) use doc_fn_allocator;
pub(crate) use doc_fn_reset;
pub(crate) use doc_fn_scope;
pub(crate) use doc_fn_stats;
pub(crate) use doc_fn_stats_greedy;

/// Allocators that don't need to have `deallocate` or `shrink` being called, because they have another way of reclamation.
///
/// # Safety
/// The `deallocate` and `shrink` implementation must be ok to be called with a pointer that was not allocated by this Allocator and with a size that is smaller than what was allocated.
pub unsafe trait BumpAllocator: Allocator {}

unsafe impl<A: BumpAllocator> BumpAllocator for &A {}

unsafe impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> BumpAllocator for BumpScope<'_, A, MIN_ALIGN, UP> where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment
{
}

unsafe impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> BumpAllocator for Bump<A, MIN_ALIGN, UP> where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment
{
}

/// Associates a lifetime to a wrapped type.
///
/// This is used for [`BumpBox::into_box`] to attach a lifetime to the `Box`.
#[derive(Debug, Clone)]
pub struct WithLifetime<'a, A> {
    inner: A,
    marker: PhantomData<&'a mut ()>,
}

#[allow(missing_docs)]
impl<'a, A> WithLifetime<'a, A> {
    #[inline(always)]
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> A {
        self.inner
    }
}

unsafe impl<'a, A: Allocator> Allocator for WithLifetime<'a, A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate(layout)
    }

    #[inline(always)]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate_zeroed(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.inner.deallocate(ptr, layout);
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.grow(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.grow_zeroed(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.shrink(ptr, old_layout, new_layout)
    }
}

trait ErrorBehavior: Sized {
    const IS_FALLIBLE: bool;
    fn allocation(layout: Layout) -> Self;
    fn capacity_overflow() -> Self;
}

impl ErrorBehavior for Infallible {
    const IS_FALLIBLE: bool = false;

    #[inline(always)]
    fn allocation(layout: Layout) -> Self {
        handle_alloc_error(layout)
    }

    #[inline(always)]
    fn capacity_overflow() -> Self {
        capacity_overflow()
    }
}

#[cold]
#[inline(never)]
#[cfg(not(feature = "alloc"))]
fn handle_alloc_error(_layout: Layout) -> ! {
    panic!("allocation failed")
}

impl ErrorBehavior for AllocError {
    const IS_FALLIBLE: bool = true;

    #[inline(always)]
    fn allocation(_: Layout) -> Self {
        Self
    }

    #[inline(always)]
    fn capacity_overflow() -> Self {
        Self
    }
}

// this is just `Result::into_ok` but with a name to match our use case
#[inline(always)]
#[cfg(not(no_global_oom_handling))]
fn infallible<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => unreachable!(),
    }
}

trait SizedTypeProperties: Sized {
    const SIZE: usize = mem::size_of::<Self>();
    const ALIGN: usize = mem::align_of::<Self>();

    const IS_ZST: bool = mem::size_of::<Self>() == 0;
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

macro_rules! doc_align_cant_decrease {
    () => {
        "**This can not decrease the alignment.** Trying to decrease alignment will result in a compile error. \
        You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment."
        // To decrease alignment we need to ensure that we return to our original alignment. We can only do this
        // using a closure which uses "guard" type that resets to the original alignment on drop.
    };
}

pub(crate) use doc_align_cant_decrease;

macro_rules! condition {
    (if true { $($then:tt)* } else { $($else:tt)* }) => { $($then)* };
    (if false { $($then:tt)* } else { $($else:tt)* }) => { $($else)* };
}

pub(crate) use condition;

macro_rules! bump_common_methods {
    ($scope_guard:ident, $is_scope:ident) => {
        /// Calls `f` with a new child scope.
        ///
        /// # Examples
        ///
        /// ```
        /// # use bump_scope::Bump;
        /// let mut bump: Bump = Bump::new();
        ///
        /// bump.scoped(|bump| {
        ///     bump.alloc_str("Hello world!");
        ///     assert_eq!(bump.stats().allocated(), 12);
        /// });
        ///
        /// assert_eq!(bump.stats().allocated(), 0);
        /// ```
        #[inline(always)]
        pub fn scoped(&mut self, f: impl FnOnce(BumpScope<A, MIN_ALIGN, UP>)) {
            let mut guard = self.scope_guard();
            f(guard.scope());
        }

        /// Calls `f` with a new child scope of a new minimum alignment.
        #[inline(always)]
        pub fn scoped_aligned<const NEW_MIN_ALIGN: usize>(&mut self, f: impl FnOnce(BumpScope<A, MIN_ALIGN, UP>))
        where
            MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
        {
            let mut guard = self.scope_guard();

            // SAFETY: We can cast the alignment to anything here, because once `guard` drops we return to our original alignment.
            f(unsafe { guard.scope().cast_align() });
        }

        #[doc = concat!("Creates a new [`", stringify!($scope_guard), "`].")]
        /// This allows for creation of child scopes.
        ///
        /// # Examples
        ///
        /// ```
        /// # use bump_scope::Bump;
        /// let mut bump: Bump = Bump::new();
        ///
        /// {
        ///     let mut guard = bump.scope_guard();
        ///     let bump = guard.scope();
        ///     bump.alloc_str("Hello world!");
        ///     assert_eq!(bump.stats().allocated(), 12);
        /// }
        ///
        /// assert_eq!(bump.stats().allocated(), 0);
        /// ```
        #[must_use]
        #[inline(always)]
        pub fn scope_guard(&mut self) -> $scope_guard<A, MIN_ALIGN, UP> {
            $scope_guard::new(self)
        }

        /// Calls `f` with this scope but with a new minimum alignment.
        #[inline(always)]
        pub fn aligned<const NEW_MIN_ALIGN: usize>(&mut self, f: impl FnOnce(BumpScope<A, NEW_MIN_ALIGN, UP>))
        where
            MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
        {
            $crate::condition! {
                if $is_scope {
                    if NEW_MIN_ALIGN < MIN_ALIGN {
                        let guard = BumpAlignGuard::new(self);

                        // SAFETY: We can cast the alignment to anything here, because once `guard` drops we return to our original alignment.
                        f(unsafe { guard.scope.clone_unchecked().cast_align() });
                    } else {
                        f(unsafe {
                            self.clone_unchecked().cast_align()
                        })
                    }
                } else {
                    self.as_mut_scope().aligned(f)
                }
            }
        }

        #[doc = $crate::doc_fn_stats!()]
        #[must_use]
        #[inline(always)]
        pub fn stats(&self) -> $crate::condition! { if $is_scope { Stats<'a, UP> } else { Stats<UP> } } {
            Stats {
                current: $crate::Chunk::new(self.into()),
            }
        }

        /// Creates a checkpoint of the current bump position.
        ///
        /// # Examples
        /// ```
        /// # use bump_scope::Bump;
        /// # let mut bump: Bump = Bump::new();
        /// let checkpoint = bump.checkpoint();
        ///
        /// {
        ///     let hello = bump.alloc_str("hello");
        ///     assert_eq!(bump.stats().allocated(), 5);
        /// }
        ///
        /// unsafe { bump.reset_to(checkpoint); }
        /// assert_eq!(bump.stats().allocated(), 0);
        /// ```
        #[inline]
        pub fn checkpoint(&self) -> Checkpoint {
            Checkpoint::new(self.chunk.get())
        }

        /// Resets the bump position to a previously created checkpoint. The memory that has been allocated since then will be reused by future allocations.
        ///
        /// # Safety
        /// - the checkpoint must have been created by this bump allocator
        /// - the bump allocator must not have been [`reset`](crate::Bump::reset) since creation of this checkpoint
        /// - there must be no references to allocations made since creation of this checkpoint
        #[inline]
        pub unsafe fn reset_to(&mut self, checkpoint: Checkpoint) {
            $crate::condition! {
                if $is_scope {
                    debug_assert!(self.stats().big_to_small().any(|c| {
                        c.chunk.header_ptr() == checkpoint.chunk.cast() &&
                        c.chunk.contains_addr_or_end(checkpoint.address.get())
                    }));

                    checkpoint.reset_within_chunk();
                    let chunk = RawChunk::from_header(checkpoint.chunk.cast());
                    self.chunk.set(chunk);
                } else {
                    self.as_mut_scope().reset_to(checkpoint)
                }
            }
        }

        #[doc = crate::doc_fn_allocator!()]
        #[must_use]
        #[inline(always)]
        pub fn allocator(&self) -> &A {
            unsafe { self.chunk.get().allocator().as_ref() }
        }

        #[cfg(test)]
        /// Wraps `self` in [`WithDrop`] so that allocations return `&mut T`.
        pub fn with_drop(self) -> WithDrop<Self> {
            WithDrop::new(self)
        }

        #[cfg(test)]
        /// Wraps `&self` in [`WithDrop`] so that allocations return `&mut T`.
        pub fn with_drop_ref(&self) -> WithDrop<&Self> {
            WithDrop::new(self)
        }

        #[cfg(test)]
        /// Wraps `&mut self` in [`WithDrop`] so that allocations return `&mut T`.
        pub fn with_drop_mut(&mut self) -> WithDrop<&mut Self> {
            WithDrop::new(self)
        }

        /// Wraps `&self` in [`WithoutDealloc`] so that [`deallocate`] becomes a no-op.
        ///
        /// [`deallocate`]: Allocator::deallocate
        pub fn without_dealloc(&self) -> WithoutDealloc<&Self> {
            WithoutDealloc(self)
        }

        /// Wraps `&self` in [`WithoutShrink`] so that [`shrink`] becomes a no-op.
        ///
        /// [`shrink`]: Allocator::shrink
        pub fn without_shrink(&self) -> WithoutShrink<&Self> {
            WithoutShrink(self)
        }
    };
}

pub(crate) use bump_common_methods;

macro_rules! wrap_result {
    ($ok:ty, $err:ty) => { Result<$ok, $err> };
    (, $err:ty) => { Result<(), $err> };
}

pub(crate) use wrap_result;

macro_rules! error_behavior_generic_methods {
    (
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

            fn $generic:ident
            $(<{$($generic_params:tt)*}>)?
            (
                $(&mut $self_mut:ident ,)?
                $($arg_pat:ident: $arg_ty:ty),* $(,)?
            )
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
            /// Panics if the allocation fails.
            $(#[doc = "\n"] $(#[doc = $panics])*)?
            $(#[doc = "\n"] $(#[doc = $infallible_panics])*)?

            #[doc = $crate::map!({ $($($errors)*)? $($($infallible_errors)*)? } become { "# Errors" } else { "" })]
            $(#[doc = "\n"] $(#[doc = $errors])*)?
            $(#[doc = "\n"] $(#[doc = $infallible_errors])*)?

            #[doc = $crate::map!({ $($($examples)*)? $($($infallible_examples)*)? } become { "# Examples" } else { "" })]
            $(#[doc = "\n"] $(#[doc = $examples])*)?
            $(#[doc = "\n"] $(#[doc = $infallible_examples])*)?

            #[inline(always)]
            #[cfg(not(no_global_oom_handling))]
            pub fn $infallible
            $(<$($generic_params)*>)?
            ($(&mut $self_mut,)?  $($arg_pat: $arg_ty),*) $(-> $return_ty)?
            $(where $($where)*)?
            {
                infallible(Self::$generic($($self_mut,)? $($arg_pat),*))
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

            #[doc = $crate::map!({ $($($examples)*)? $($($fallible_examples)*)? } become { "# Examples" } else { "" })]
            $(#[doc = "\n"] $(#[doc = $examples])*)?
            $(#[doc = "\n"] $(#[doc = $fallible_examples])*)?

            #[inline(always)]
            pub fn $fallible
            $(<$($generic_params)*>)?
            ($(&mut $self_mut,)? $($arg_pat: $arg_ty),*)
            -> $crate::wrap_result!($($return_ty)?, AllocError)
            $(where $($where)*)?
            {
                Self::$generic($($self_mut,)? $($arg_pat),*)
            }
        )*

        $(
            $(#[$attr])*
            #[inline]
            pub(crate) fn $generic
            <B: ErrorBehavior $(, $($generic_params)*)?>
            ($(&mut $self_mut,)? $($arg_pat: $arg_ty),*)
            -> $crate::wrap_result!($($return_ty)?, B)
            $(where $($where)*)?
            {
                $($body)*
            }
        )*
    };
}

pub(crate) use error_behavior_generic_methods;

macro_rules! map {
    ({ } become { $($then:tt)* }) => { };
    ({ } become { $($then:tt)* } else { $($else:tt)* }) => { $($else)* };
    ({ $($from:tt)+ } become { $($then:tt)* }) => { $($then)* };
    ({ $($from:tt)+ } become { $($then:tt)* } else { $($else:tt)* }) => { $($then)* };
}

pub(crate) use map;

macro_rules! doc_alloc_methods {
    () => {
        "Functions to allocate. Available as fallible or infallible."
    };
}

macro_rules! last {
    ($self:ident) => {
        $self
    };
    ($mut:ident $self:ident) => {
        $self
    };
}

macro_rules! as_scope {
    ($self:ident) => {
        $self.as_scope()
    };
    ($mut:ident $self:ident) => {
        $self.as_mut_scope()
    };
}

macro_rules! define_alloc_methods {
    (
        macro $macro_name:ident

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

            fn $generic:ident
            $(<{$($generic_params:tt)*}>)?
            (&$($self:ident)+ $(, $arg_pat:ident: $arg_ty:ty)* $(,)?)
            $(-> $return_ty:ty | $return_ty_scope:ty)?
            $(where { $($where:tt)* } in)?
            {
                $($body:tt)*
            }
        )*
    ) => {
        macro_rules! $macro_name {
            (BumpScope) => {
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

                    #[doc = $crate::map!({ $($($examples)*)? $($($infallible_examples)*)? } become { "# Examples" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $examples])*)?
                    $(#[doc = "\n"] $(#[doc = $infallible_examples])*)?

                    #[inline(always)]
                    #[cfg(not(no_global_oom_handling))]
                    pub fn $infallible
                    $(<$($generic_params)*>)?
                    (&$($self)+ $(, $arg_pat: $arg_ty)*)
                    $(-> $return_ty_scope)?
                    $(where $($where)*)?
                    {
                        infallible(last!($($self)+).$generic($($arg_pat),*))
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

                    #[doc = $crate::map!({ $($($examples)*)? $($($fallible_examples)*)? } become { "# Examples" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $examples])*)?
                    $(#[doc = "\n"] $(#[doc = $fallible_examples])*)?

                    #[inline(always)]
                    pub fn $fallible
                    $(<$($generic_params)*>)?
                    (&$($self)+ $(, $arg_pat: $arg_ty)*)
                    -> $crate::wrap_result!($($return_ty_scope)?, AllocError)
                    $(where $($where)*)?
                    {
                        last!($($self)+).$generic($($arg_pat),*)
                    }
                )*

                $(
                    $(#[$attr])*
                    #[inline(always)]
                    fn $generic
                    <B: ErrorBehavior $(, $($generic_params)*)?>
                    (&$($self)+ $(, $arg_pat: $arg_ty)*)
                    -> $crate::wrap_result!($($return_ty_scope)?, B)
                    $(where $($where)*)?
                    {
                        $($body)*
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

                    #[doc = $crate::map!({ $($($examples)*)? $($($infallible_examples)*)? } become { "# Examples" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $examples])*)?
                    $(#[doc = "\n"] $(#[doc = $infallible_examples])*)?

                    #[inline(always)]
                    #[cfg(not(no_global_oom_handling))]
                    pub fn $infallible
                    $(<$($generic_params)*>)?
                    (&$($self)+ $(, $arg_pat: $arg_ty)*)
                    $(-> $return_ty)?
                    $(where $($where)*)?
                    {
                        as_scope!($($self)+).$infallible($($arg_pat),*)
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

                    #[doc = $crate::map!({ $($($examples)*)? $($($fallible_examples)*)? } become { "# Examples" } else { "" })]
                    $(#[doc = "\n"] $(#[doc = $examples])*)?
                    $(#[doc = "\n"] $(#[doc = $fallible_examples])*)?

                    #[inline(always)]
                    pub fn $fallible
                    $(<$($generic_params)*>)?
                    (&$($self)+ $(, $arg_pat: $arg_ty)*)
                    -> $crate::wrap_result!($($return_ty)?, AllocError)
                    $(where $($where)*)?
                    {
                        as_scope!($($self)+).$fallible($($arg_pat),*)
                    }
                )*
            };
        }

        pub(crate) use $macro_name;
    };
}

define_alloc_methods! {
    macro alloc_methods

    /// Allocate an object.
    impl
    for fn alloc
    for fn try_alloc
    fn generic_alloc<{T}>(&self, value: T) -> BumpBox<T> | BumpBox<'a, T> {
        if T::IS_ZST {
            return Ok(BumpBox::zst());
        }

        self.generic_alloc_with(|| value)
    }

    /// Pre-allocate space for an object. Once space is allocated `f` will be called to create the value to be put at that place.
    /// In some situations this can help the compiler realize that `T` can be constructed at the allocated space instead of having to copy it over.
    impl
    for fn alloc_with
    for fn try_alloc_with
    fn generic_alloc_with<{T}>(&self, f: impl FnOnce() -> T) -> BumpBox<T> | BumpBox<'a, T> {
        if T::IS_ZST {
            return Ok(BumpBox::zst());
        }

        let ptr = self.do_alloc_sized::<B, T>()?;

        unsafe {
            pointer::write_with(ptr.as_ptr(), f);
            Ok(BumpBox::from_raw(ptr))
        }
    }

    /// Allocate an object with its default value.
    impl
    /// This is equivalent to `alloc_with(T::default())`.
    for fn alloc_default
    /// This is equivalent to `try_alloc_with(T::default())`.
    for fn try_alloc_default
    fn generic_alloc_default<{T: Default}>(&self) -> BumpBox<T> | BumpBox<'a, T> {
        self.generic_alloc_with(Default::default)
    }

    #[doc(hidden)]
    impl
    for fn alloc_try_with
    for fn try_alloc_try_with
    fn generic_alloc_try_with<{T, E}>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpBox<T>, E> | Result<BumpBox<'a, T>, E> {
        if T::IS_ZST {
            return match f() {
                Ok(_) => Ok(Ok(BumpBox::zst())),
                Err(error) => Ok(Err(error)),
            }
        }

        let before_chunk = self.chunk.get();
        let before_chunk_pos = nonnull::addr(before_chunk.pos()).get();

        let ptr = self.do_alloc_no_bump_for::<B, Result<T, E>>()?;

        Ok(unsafe {
            pointer::write_with(ptr.as_ptr(), f);

            match nonnull::result(ptr) {
                Ok(value) => Ok({
                    let new_pos = if UP {
                        let pos = nonnull::addr(nonnull::add(value, 1)).get();
                        up_align_usize_unchecked(pos, MIN_ALIGN)
                    } else {
                        let pos = nonnull::addr(value).get();
                        down_align_usize(pos, MIN_ALIGN)
                    };

                    self.chunk.get().set_pos_addr(new_pos);
                    BumpBox::from_raw(value)
                }),
                Err(error) => Err({
                    let error = error.as_ptr().read();
                    before_chunk.reset_to(before_chunk_pos);
                    self.chunk.set(before_chunk);
                    error
                }),
            }
        })
    }

    /// Allocate a slice and `Copy` elements from an existing slice.
    impl
    for fn alloc_slice_copy
    for fn try_alloc_slice_copy
    fn generic_alloc_slice_copy<{T: Copy}>(
        &self,
        slice: &[T],
    ) -> BumpBox<[T]> | BumpBox<'a, [T]> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(slice.len()));
        }

        let len = slice.len();
        let src = slice.as_ptr();
        let dst = self.do_alloc_slice_for(slice)?;

        unsafe {
            core::ptr::copy_nonoverlapping(src, dst.cast::<T>().as_ptr(), len);
            Ok(BumpBox::from_raw(dst))
        }
    }

    /// Allocate a slice and `Clone` elements from an existing slice.
    impl
    for fn alloc_slice_clone
    for fn try_alloc_slice_clone
    fn generic_alloc_slice_clone<{T: Clone}>(
        &self,
        slice: &[T],
    ) -> BumpBox<[T]> | BumpBox<'a, [T]> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(slice.len()));
        }

        Ok(self.generic_alloc_uninit_slice_for(slice)?.init_clone(slice))
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    impl
    for fn alloc_slice_fill
    for fn try_alloc_slice_fill
    fn generic_alloc_slice_fill<{T: Clone}>(&self, len: usize, value: T) -> BumpBox<[T]> | BumpBox<'a, [T]> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(len));
        }

        Ok(self.generic_alloc_uninit_slice(len)?.init_fill(value))
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`(try_)alloc_slice_fill`](Self::alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    impl
    for fn alloc_slice_fill_with
    for fn try_alloc_slice_fill_with
    fn generic_alloc_slice_fill_with<{T}>(&self, len: usize, f: impl FnMut() -> T) -> BumpBox<[T]> | BumpBox<'a, [T]> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(len));
        }

        Ok(self.generic_alloc_uninit_slice(len)?.init_fill_with(f))
    }

    /// Allocate a `str`.
    impl
    for fn alloc_str
    for fn try_alloc_str
    fn generic_alloc_str(&self, src: &str) -> BumpBox<str> | BumpBox<'a, str> {
        let slice = self.generic_alloc_slice_copy(src.as_bytes())?;

        // SAFETY: input is `str` so this is too
        Ok(unsafe { slice.into_boxed_str_unchecked() })
    }

    #[cfg(feature = "alloc")]
    /// Allocate a `str` from format arguments.
    impl
    /// For better performance prefer [`alloc_fmt_mut`](Bump::alloc_fmt_mut).
    do panics
    /// Panics if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    for fn alloc_fmt
    /// For better performance prefer [`try_alloc_fmt_mut`](Bump::try_alloc_fmt_mut).
    do errors
    /// Errors if a formatting trait implementation returned an error.
    for fn try_alloc_fmt
    fn generic_alloc_fmt(&self, args: fmt::Arguments) -> BumpBox<str> | BumpBox<'a, str> {
        use allocator_api2::vec::Vec;

        struct StringBuilder<A: Allocator, B: ErrorBehavior> {
            vec: Vec<u8, A>,
            marker: PhantomData<*const B>,
        }

        impl<A: Allocator, B: ErrorBehavior> StringBuilder<A, B> {
            fn new_in(allocator: A) -> Self {
                Self {
                    vec: Vec::new_in(allocator),
                    marker: PhantomData,
                }
            }
        }

        impl<A: Allocator, B: ErrorBehavior> fmt::Write for StringBuilder<A, B> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                if B::IS_FALLIBLE {
                    self.vec.try_reserve(s.len()).map_err(|_| fmt::Error)?;
                }

                self.vec.extend_from_slice(s.as_bytes());
                Ok(())
            }
        }

        if let Some(string) = args.as_str() {
            return self.generic_alloc_str(string);
        }

        let mut string = StringBuilder::<_, B>::new_in(self);

        if fmt::Write::write_fmt(&mut string, args).is_err() {
            return Err(B::capacity_overflow());
        }

        let (ptr, len, _) = string.vec.into_raw_parts();

        unsafe {
            let ptr = NonNull::new_unchecked(ptr);
            let slice = nonnull::slice_from_raw_parts(ptr, len);
            let boxed = BumpBox::from_raw(slice);
            let boxed = boxed.into_boxed_str_unchecked();
            Ok(boxed)
        }
    }

    /// Allocate a `str` from format arguments.
    impl
    /// Unlike [`alloc_fmt`](Bump::alloc_fmt), this function requires a mutable `Bump(Scope)`.
    do panics
    /// Panics if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    for fn alloc_fmt_mut
    /// Unlike [`try_alloc_fmt`](Bump::try_alloc_fmt), this function requires a mutable `Bump(Scope)`.
    do errors
    /// Errors if a formatting trait implementation returned an error.
    for fn try_alloc_fmt_mut
    fn generic_alloc_fmt_mut(&mut self, args: fmt::Arguments) -> BumpBox<str> | BumpBox<'a, str> {
        if let Some(string) = args.as_str() {
            return self.generic_alloc_str(string);
        }

        let mut string = BumpString::generic_with_capacity_in(0, self)?;

        if fmt::Write::write_fmt(&mut string, args).is_err() {
            return Err(B::capacity_overflow());
        }

        Ok(string.into_boxed_str())
    }

    #[cfg(feature = "alloc")]
    /// Allocate elements of an iterator into a slice.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let slice = bump.alloc_iter([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    impl
    /// For better performance prefer [`alloc_iter_exact`](Bump::try_alloc_iter_exact) or [`alloc_iter_mut(_rev)`](Bump::alloc_iter_mut).
    for fn alloc_iter
    /// For better performance prefer [`try_alloc_iter_exact`](Bump::try_alloc_iter_exact) or [`try_alloc_iter_mut(_rev)`](Bump::try_alloc_iter_mut).
    for fn try_alloc_iter
    fn generic_alloc_iter<{T}>(&self, iter: impl IntoIterator<Item = T>) -> BumpBox<[T]> | BumpBox<'a, [T]> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(iter.into_iter().count()));
        }

        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = allocator_api2::vec::Vec::new_in(self);

        if B::IS_FALLIBLE {
            vec.try_reserve(capacity).map_err(|_| B::capacity_overflow())?;
        } else {
            vec.reserve(capacity);
        }

        for value in iter {
            if B::IS_FALLIBLE {
                vec.try_reserve(1).map_err(|_| B::capacity_overflow())?;
            }

            vec.push(value);
        }

        vec.shrink_to_fit();
        let (ptr, len, _) = vec.into_raw_parts();

        unsafe {
            let ptr = NonNull::new_unchecked(ptr);
            let slice = nonnull::slice_from_raw_parts(ptr, len);
            Ok(BumpBox::from_raw(slice))
        }
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    do panics
    /// Panics if the supplied iterator returns fewer elements than it promised.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// #
    /// let slice = bump.alloc_iter_exact([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    for fn alloc_iter_exact
    for fn try_alloc_iter_exact
    fn generic_alloc_iter_exact<{T, I}>(&self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> BumpBox<[T]> | BumpBox<'a, [T]>
    where {
        I: ExactSizeIterator<Item = T>
    } in {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(iter.into_iter().count()));
        }

        let mut iter = iter.into_iter();
        let len = iter.len();

        let uninit = self.generic_alloc_uninit_slice(len)?;
        let mut initializer = uninit.initializer();

        unsafe {
            while !initializer.is_full() {
                let value = match iter.next() {
                    Some(value) => value,
                    None => exact_size_iterator_bad_len(),
                };

                initializer.push(value);
            }

            Ok(initializer.into_init())
        }
    }

    /// Allocate elements of an iterator into a slice.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let slice = bump.alloc_iter_mut([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    impl
    /// Unlike [`alloc_iter`](Bump::alloc_iter), this function requires a mutable `Bump(Scope)`.
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`](Bump::alloc_iter_mut_rev) or [`alloc_iter_exact`](Bump::alloc_iter_exact) as in this case this function incurs an additional copy of the slice internally.
    for fn alloc_iter_mut
    /// Unlike [`try_alloc_iter`](Bump::try_alloc_iter), this function requires a mutable `Bump(Scope)`.
    ///
    /// When bumping downwards, prefer [`try_alloc_iter_mut_rev`](Bump::try_alloc_iter_mut_rev) or [`try_alloc_iter_exact`](Bump::try_alloc_iter_exact) as in this case this function incurs an additional copy of the slice internally.
    for fn try_alloc_iter_mut
    fn generic_alloc_iter_mut<{T}>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<[T]> | BumpBox<'a, [T]> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(iter.into_iter().count()));
        }

        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = BumpVec::<T, A, MIN_ALIGN, UP>::generic_with_capacity_in(capacity, self)?;

        for value in iter {
            vec.generic_push(value)?;
        }

        Ok(vec.into_boxed_slice())
    }

    /// Allocate elements of an iterator into a slice in reverse order.
    impl
    ///
    /// When bumping upwards, prefer [`alloc_iter_mut`](Bump::alloc_iter_mut) or [`alloc_iter_exact`](Bump::alloc_iter_exact) as in this case this function incurs an additional copy of the slice internally.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// #
    /// let slice = bump.alloc_iter_mut_rev([1, 2, 3]);
    /// assert_eq!(slice, [3, 2, 1]);
    /// ```
    for fn alloc_iter_mut_rev
    ///
    /// When bumping upwards, prefer [`try_alloc_iter_mut`](Bump::try_alloc_iter) or [`try_alloc_iter_exact`](Bump::try_alloc_iter_exact) as in this case this function incurs an additional copy of the slice internally.
    for fn try_alloc_iter_mut_rev
    fn generic_alloc_iter_mut_rev<{T}>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<[T]> | BumpBox<'a, [T]> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(iter.into_iter().count()));
        }

        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = BumpVecRev::<T, A, MIN_ALIGN, UP>::generic_with_capacity_in(capacity, self)?;

        for value in iter {
            vec.generic_push(value)?;
        }

        Ok(vec.into_boxed_slice())
    }

    /// Allocate an unitialized object.
    ///
    /// You can safely initialize the object with [`init`](BumpBox::init) or unsafely with [`assume_init`](BumpBox::assume_init).
    do examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let mut five = bump.alloc_uninit();
    ///
    /// let five = five.init(5);
    ///
    /// assert_eq!(*five, 5)
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let mut five = bump.alloc_uninit();
    ///
    /// let five = unsafe {
    ///     five.write(5);
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    impl
    for fn alloc_uninit
    for fn try_alloc_uninit
    fn generic_alloc_uninit<{T}>(&self) -> BumpBox<MaybeUninit<T>> | BumpBox<'a, MaybeUninit<T>> {
        if T::IS_ZST {
            return Ok(BumpBox::zst());
        }

        let ptr = self.do_alloc_sized::<B, T>()?.cast::<MaybeUninit<T>>();
        unsafe { Ok(BumpBox::from_raw(ptr)) }
    }

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with [`init_fill`](BumpBox::init_fill), [`init_fill_with`](BumpBox::init_fill_with), [`init_copy`](BumpBox::init_copy), [`init_clone`](BumpBox::init_clone) or unsafely with [`assume_init`](BumpBox::assume_init).
    do examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// let mut values = bump.alloc_uninit_slice(3);
    ///
    /// let values = values.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(*values, [1, 2, 3])
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
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
    /// assert_eq!(*values, [1, 2, 3])
    /// ```
    impl
    for fn alloc_uninit_slice
    for fn try_alloc_uninit_slice
    fn generic_alloc_uninit_slice<{T}>(&self, len: usize) -> BumpBox<[MaybeUninit<T>]> | BumpBox<'a, [MaybeUninit<T>]> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(len));
        }

        let ptr = self.do_alloc_slice::<B, T>(len)?.cast::<MaybeUninit<T>>();

        unsafe {
            let ptr = nonnull::slice_from_raw_parts(ptr, len);
            Ok(BumpBox::from_raw(ptr))
        }
    }

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with [`init_fill`](BumpBox::init_fill), [`init_fill_with`](BumpBox::init_fill_with), [`init_copy`](BumpBox::init_copy), [`init_clone`](BumpBox::init_clone) or unsafely with [`assume_init`](BumpBox::assume_init).
    ///
    /// This is just like `(try_)alloc_uninit_slice` but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    impl
    for fn alloc_uninit_slice_for
    for fn try_alloc_uninit_slice_for
    fn generic_alloc_uninit_slice_for<{T}>(&self, slice: &[T]) -> BumpBox<[MaybeUninit<T>]> | BumpBox<'a, [MaybeUninit<T>]> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice(slice.len()));
        }

        let ptr = self.do_alloc_slice_for::<B, T>(slice)?.cast::<MaybeUninit<T>>();

        unsafe {
            let ptr = nonnull::slice_from_raw_parts(ptr, slice.len());
            Ok(BumpBox::from_raw(ptr))
        }
    }

    /// Allocate a [`FixedBumpVec`] with the given `capacity`.
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
    impl
    for fn alloc_fixed_vec
    for fn try_alloc_fixed_vec
    fn generic_alloc_fixed_vec<{T}>(&self, len: usize) -> FixedBumpVec<T> | FixedBumpVec<'a, T> {
        Ok(self.generic_alloc_uninit_slice(len)?.into_fixed_vec())
    }

    /// Allocates memory as described by the given `Layout`.
    impl
    for fn alloc_layout
    for fn try_alloc_layout
    fn generic_alloc_layout(&self, layout: Layout) -> NonNull<u8> | NonNull<u8> {
        match self.chunk.get().alloc::<MIN_ALIGN, false, false, _>(layout) {
            Some(ptr) => Ok(ptr),
            None => self.alloc_in_another_chunk(layout),
        }
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve`, `chunks().remaining()` will be greater than or equal to
    /// `additional`. Does nothing if capacity is already sufficient.
    do examples
    /// ```
    /// # use bump_scope::{ Bump };
    /// #
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve_bytes(4096);
    /// assert!(bump.stats().capacity() > 4096);
    /// ```
    impl
    for fn reserve_bytes
    for fn try_reserve_bytes
    fn generic_reserve_bytes(&self, additional: usize) {
        let mut additional = additional;
        let mut chunk = self.chunk.get();

        loop {
            if let Some(rest) = additional.checked_sub(chunk.remaining()) {
                additional = rest;
            } else {
                return Ok(());
            }

            if let Some(next) = chunk.next() {
                chunk = next;
            } else {
                break;
            }
        }

        let layout = match Layout::from_size_align(additional, 1) {
            Ok(ok) => ok,
            Err(_) => return Err(B::capacity_overflow()),
         };
        chunk.append_for(layout).map(drop)
    }
}

#[doc = doc_alloc_methods!()]
impl<'a, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> BumpScope<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    alloc_methods!(BumpScope);
}

#[doc = doc_alloc_methods!()]
impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    alloc_methods!(Bump);
}
