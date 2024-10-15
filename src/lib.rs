// NB: We avoid using closures to map `Result` and `Option`s in various places because they result in less readable assembly output.
// When using closures, functions like `capacity_overflow` can get the name of some closure that invokes it instead, like `bump_scope::mut_bump_vec::MutBumpVec<T,_,_,A>::generic_grow_cold::{{closure}}`.

// This crate uses modified code from the rust standard library. <https://github.com/rust-lang/rust/tree/master/library>.
// Especially `BumpBox` methods, vectors, strings, `polyfill` and `tests/from_std` are based on code from the standard library.

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api, vec_into_raw_parts))]
#![cfg_attr(feature = "nightly-coerce-unsized", feature(coerce_unsized, unsize))]
#![cfg_attr(feature = "nightly-exact-size-is-empty", feature(exact_size_is_empty))]
#![cfg_attr(feature = "nightly-trusted-len", feature(trusted_len))]
#![cfg_attr(feature = "nightly-const-refs-to-static", feature(const_refs_to_static))]
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
        extract_if,
        slice_flatten,
        slice_partition_dedup,
        iter_partition_in_place,
        strict_provenance,
        offset_of_enum,
    )
)]
#![cfg_attr(
    docsrs,
    feature(doc_auto_cfg, doc_cfg_hide),
    doc(cfg_hide(no_global_oom_handling, feature = "nightly-const-refs-to-static"))
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
    unknown_lints,
    rustdoc::redundant_explicit_links, // for cargo-rdme
    stable_features, // for const_refs_to_static
)]
#![doc(test(attr(warn(dead_code))))]
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
//! - `Bump` and `BumpScope` have the same repr as `NonNull<u8>`. (vs 3x pointer sized)
//! - Won't try to allocate a smaller chunk if allocation failed.
//! - No built-in allocation limit. You can provide an allocator that enforces an allocation limit (see `tests/limit_memory_usage.rs`).
//! - Allocations are a bit more optimized. (see `crates/inspect-asm/out/x86-64` and [benchmarks](https://bluurryy.github.io/bump-scope/criterion/report/))
//! - [You can choose the bump direction.](#bumping-upwards-or-downwards) Bumps upwards by default.
//! - [You can choose the minimum alignment.](#minimum-alignment)
//!
//! # Scopes and Checkpoints
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
//! # Collections
//! `bump-scope` provides bump allocated variants of `Vec` and `String` called [`BumpVec`](crate::BumpVec) and [`BumpString`](crate::BumpString). They also come in a different flavors:
//! - [`Fixed*`](crate::FixedBumpVec) for fixed capacity collections
//! - [`Mut*`](crate::MutBumpVec) for collections optimized for a mutable bump allocator
//!
//! # Parallel Allocation
//! [`Bump`](crate::Bump) is `!Sync` which means it can't be shared between threads.
//!
//! To bump allocate in parallel you can use a [`BumpPool`](crate::BumpPool).
//!
//! # Allocator API
//! `Bump` and `BumpScope` implement [`allocator_api2::alloc::Allocator`](https://docs.rs/allocator-api2/0.2.16/allocator_api2/alloc/trait.Allocator.html).
//! With this you can bump allocate [`allocator_api2::boxed::Box`](https://docs.rs/allocator-api2/0.2.16/allocator_api2/boxed/struct.Box.html), [`allocator_api2::vec::Vec`](https://docs.rs/allocator-api2/0.2.16/allocator_api2/vec/struct.Vec.html) and collections
//! from other crates that support it like [`hashbrown::HashMap`](https://docs.rs/hashbrown/latest/hashbrown/struct.HashMap.html).
//! They also implement the nightly allocator api with a [feature flag](#nightly-features).
//!
//! A bump allocator can grow, shrink and deallocate the most recent allocation.
//! When bumping upwards it can even do so in place.
//! Growing allocations other than the most recent one will require a new allocation and the old memory block becomes wasted space.
//! Shrinking or deallocating allocations other than the most recent one does nothing, which means wasted space.
//!
//! A bump allocator does not *require* `deallocate` or `shrink` to free memory.
//! After all, memory will be reclaimed when exiting a scope or calling `reset`.
//! You can wrap a bump allocator in a type that makes `deallocate` and `shrink` a no-op using [`without_dealloc`](crate::Bump::without_dealloc) and [`without_shrink`](crate::Bump::without_shrink).
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
//! * **`std`** *(enabled by default)* —  Adds `BumpPool` and implementations of `std::io` traits for `BumpBox` and vectors.
//! * **`alloc`** —  Adds `Global` as the default base allocator, `BumpBox::into_box` and some interactions with `alloc` collections.
//! * **`serde`** —  Adds `Serialize` implementations for `BumpBox`, strings and vectors, and `DeserializeSeed` for strings and vectors.
//! * **`zerocopy`** —  Adds `alloc_zeroed(_slice)`, `init_zeroed`, `resize_zeroed` and `extend_zeroed`.
//!
//!  ### Nightly features
//! * **`nightly-allocator-api`** —  Enables `allocator-api2`'s `nightly` feature which makes it reexport the nightly allocator api instead of its own implementation.
//!   With this you can bump allocate collections from the standard library.
//! * **`nightly-coerce-unsized`** —  Makes `BumpBox<T>` implement [`CoerceUnsized`](core::ops::CoerceUnsized).
//!   With this `BumpBox<[i32;3]>` coerces to `BumpBox<[i32]>`, `BumpBox<dyn Debug>` and so on.
//! * **`nightly-const-refs-to-static`** —  Makes `Bump::unallocated` a `const fn`.
//! * **`nightly-exact-size-is-empty`** —  Implements `is_empty` manually for some iterators.
//! * **`nightly-trusted-len`** —  Implements `TrustedLen` for some iterators.
//!
//! # Bumping upwards or downwards?
//! Bump direction is controlled by the generic parameter `const UP: bool`. By default, `UP` is `true`, so the allocator bumps upwards.
//!
//! - Bumping upwards...
//!   - has the advantage that the most recent allocation can be grown and shrunk in place.
//!   - makes <code>[alloc_iter](crate::Bump::alloc_iter)([_mut](crate::Bump::alloc_iter_mut))</code> and <code>[alloc_fmt](crate::Bump::alloc_fmt)([_mut](crate::Bump::alloc_fmt_mut))</code> faster.
//! - Bumping downwards...
//!   - uses slightly fewer instructions per allocation.
//!   - makes [`alloc_iter_mut_rev`](crate::Bump::alloc_iter_mut_rev) faster.
//!
//! # Minimum alignment?
//! The minimum alignment is controlled by the generic parameter `const MIN_ALIGN: usize`. By default, `MIN_ALIGN` is `1`.
//!
//! Changing the minimum alignment to e.g. `4` makes it so allocations with the alignment of `4` don't need to align the bump pointer anymore.
//! This will penalize allocations of a smaller alignment as their size now needs to be rounded up the next multiple of `4`.
//!
//! This amounts to about 1 or 2 non-branch assembly instructions per allocation.
//!
//! # `GUARANTEED_ALLOCATED` parameter?
//! When `GUARANTEED_ALLOCATED` is `true`, the bump allocator is guaranteed to have at least one allocated chunk.
//! This is usually the case unless you create it with [`Bump::unallocated`](crate::Bump::unallocated).
//!
//! You need a guaranteed allocated `Bump(Scope)` to create scopes via `scoped` and `scope_guard`.
//! You can convert a maybe unallocated `Bump(Scope)` into a guaranteed allocated one with `into_guaranteed_allocated` or `as_guaranteed_allocated(_mut)`.
//!
//! The point of this is so `Bump`s can be created without allocating memory and even `const` constructed when the feature `nightly-const-refs-to-static` is enabled.
//! At the same time `Bump`'s that have already allocated a chunk don't suffer runtime checks for entering scopes and creating checkpoints.

#[doc(hidden)]
#[cfg(feature = "alloc")]
extern crate alloc;

mod allocator;
mod bump;
mod bump_align_guard;
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
mod chunk_raw;
mod chunk_size;
mod error_behavior;
mod features;
mod fixed_bump_string;
mod fixed_bump_vec;
mod from_utf8_error;
mod layout;
mod mut_bump_string;
/// Contains [`MutBumpVec`] and associated types.
mod mut_bump_vec;
/// Contains [`MutBumpVecRev`] and associated types.
mod mut_bump_vec_rev;
/// Contains types associated with owned slices.
pub mod owned_slice;
mod polyfill;
mod set_len_on_drop;
mod set_len_on_drop_by_ptr;
mod stats;
mod without_dealloc;

pub use allocator_api2;
#[cfg(all(feature = "alloc", not(no_global_oom_handling)))]
use allocator_api2::alloc::handle_alloc_error;
use allocator_api2::alloc::{AllocError, Allocator};
pub use bump::Bump;
pub use bump_box::BumpBox;
#[cfg(feature = "std")]
pub use bump_pool::{BumpPool, BumpPoolGuard};
pub use bump_scope::BumpScope;
pub use bump_scope_guard::{BumpScopeGuard, BumpScopeGuardRoot, Checkpoint};
pub use bump_string::BumpString;
#[doc(inline)]
pub use bump_vec::BumpVec;
use chunk_header::{unallocated_chunk_header, ChunkHeader};
use chunk_raw::RawChunk;
use chunk_size::ChunkSize;
#[cfg(not(no_global_oom_handling))]
use core::convert::Infallible;
use core::{
    alloc::Layout,
    fmt::{self, Debug},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    num::NonZeroUsize,
    ptr::NonNull,
};
use error_behavior::ErrorBehavior;
pub use fixed_bump_string::FixedBumpString;
pub use fixed_bump_vec::FixedBumpVec;
pub use from_utf8_error::FromUtf8Error;
use layout::ArrayLayout;
pub use mut_bump_string::MutBumpString;
pub use mut_bump_vec::MutBumpVec;
pub use mut_bump_vec_rev::MutBumpVecRev;
#[cfg(not(no_global_oom_handling))]
use private::{capacity_overflow, format_trait_error, Infallibly};
use set_len_on_drop::SetLenOnDrop;
use set_len_on_drop_by_ptr::SetLenOnDropByPtr;
pub use stats::{Chunk, ChunkNextIter, ChunkPrevIter, GuaranteedAllocatedStats, Stats};
pub use without_dealloc::{WithoutDealloc, WithoutShrink};

// This must be kept in sync with ChunkHeaders `repr(align(16))`.
const CHUNK_ALIGN_MIN: usize = 16;

const _: () = assert!(CHUNK_ALIGN_MIN == bumping::MIN_CHUNK_ALIGN);

/// This trait marks types that don't need dropping.
///
/// This trait is a best effort for modeling such a constraint. It is not implemented for all types that don't need dropping.
///
/// Every `T where T: Copy` and every `[T] where T: NoDrop` automatically implements `NoDrop`.
///
/// It is used as a bound for [`BumpBox`]'s [`into_ref`](BumpBox::into_ref) and [`into_mut`](BumpBox::into_mut) so you don't accidentally omit a drop that does matter.
pub trait NoDrop {}

impl NoDrop for str {}
impl<T: Copy> NoDrop for T {}
impl<T: NoDrop> NoDrop for [T] {}

/// Specifies the current minimum alignment of a bump allocator.
#[derive(Clone, Copy)]
pub struct MinimumAlignment<const ALIGNMENT: usize>;

mod supported_minimum_alignment {
    use crate::ArrayLayout;

    pub trait Sealed {
        /// We'd be fine with just an [`core::ptr::Alignment`], but that's not stable.
        const LAYOUT: ArrayLayout;

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
///
/// Any changes to this module are semver-exempt!
#[doc(hidden)]
pub mod private {
    pub use core;

    #[cfg(not(no_global_oom_handling))]
    /// Wrapper type, used for ad hoc overwriting of trait implementations, like for `Write` in `alloc_fmt`.
    pub struct Infallibly<T>(pub T);

    #[cold]
    #[inline(never)]
    #[cfg(not(no_global_oom_handling))]
    pub const fn capacity_overflow() -> ! {
        panic!("capacity overflow");
    }

    #[cold]
    #[inline(never)]
    #[cfg(not(no_global_oom_handling))]
    pub const fn format_trait_error() -> ! {
        panic!("formatting trait implementation returned an error");
    }
}

#[cold]
#[inline(never)]
fn exact_size_iterator_bad_len() -> ! {
    panic!("ExactSizeIterator did not return as many items as promised")
}

/// An allocator that allows `grow(_zeroed)`, `shrink` and `deallocate` calls with pointers that were not allocated by this allocator.
///
/// This trait is used for [`BumpBox::into_box`](BumpBox::into_box) to allow safely converting a `BumpBox` into a `Box`.
///
/// # Safety
/// - `grow(_zeroed)`, `shrink` and `deallocate` must be ok to be called with a pointer that was not allocated by this Allocator
pub unsafe trait BumpAllocator: Allocator {}

unsafe impl<A: BumpAllocator> BumpAllocator for &A {}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
impl<A> WithLifetime<'_, A> {
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

unsafe impl<A: Allocator> Allocator for WithLifetime<'_, A> {
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

#[cold]
#[inline(never)]
#[cfg(not(feature = "alloc"))]
fn handle_alloc_error(_layout: Layout) -> ! {
    panic!("allocation failed")
}

// this is just `Result::into_ok` but with a name to match our use case
#[inline(always)]
#[cfg(not(no_global_oom_handling))]
#[allow(unreachable_patterns)] // msrv 1.64.0 does not allow omitting the `Err` arm
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

macro_rules! bump_scope_methods {
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
        pub fn scoped<R>(&mut self, f: impl FnOnce(BumpScope<A, MIN_ALIGN, UP>) -> R) -> R {
            let mut guard = self.scope_guard();
            f(guard.scope())
        }

        /// Calls `f` with a new child scope of a new minimum alignment.
        #[inline(always)]
        pub fn scoped_aligned<const NEW_MIN_ALIGN: usize, R>(
            &mut self,
            f: impl FnOnce(BumpScope<A, MIN_ALIGN, UP>) -> R,
        ) -> R
        where
            MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
        {
            $crate::condition! {
                if $is_scope {
                    // This guard will reset the bump pointer to the current position, which is aligned to `MIN_ALIGN`.
                    let mut guard = self.scope_guard();
                    let scope = guard.scope();
                    scope.align::<NEW_MIN_ALIGN>();
                    f(unsafe { scope.cast_align() })
                } else {
                    self.as_mut_scope().scoped_aligned::<NEW_MIN_ALIGN, R>(f)
                }
            }
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
        pub fn aligned<const NEW_MIN_ALIGN: usize, R>(&mut self, f: impl FnOnce(BumpScope<A, NEW_MIN_ALIGN, UP>) -> R) -> R
        where
            MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
        {
            $crate::condition! {
                if $is_scope {
                    if NEW_MIN_ALIGN < MIN_ALIGN {
                        // This guard will align whatever the future bump position is to `MIN_ALIGN`.
                        let guard = BumpAlignGuard::new(self);
                        f(unsafe { guard.scope.clone_unchecked().cast_align() })
                    } else {
                        self.align::<NEW_MIN_ALIGN>();
                        f(unsafe { self.clone_unchecked().cast_align() })
                    }
                } else {
                    self.as_mut_scope().aligned(f)
                }
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
        pub unsafe fn reset_to(&self, checkpoint: Checkpoint) {
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
                    self.as_scope().reset_to(checkpoint)
                }
            }
        }

        #[doc = include_str!("docs/stats.md")]
        #[must_use]
        #[inline(always)]
        pub fn guaranteed_allocated_stats(
            &self,
        ) -> $crate::condition! { if $is_scope { GuaranteedAllocatedStats<'a, UP> } else { GuaranteedAllocatedStats<UP> } } {
            GuaranteedAllocatedStats {
                current: crate::Chunk::new_guaranteed_allocated(self.as_scope()),
            }
        }
    };
}

pub(crate) use bump_scope_methods;

macro_rules! bump_common_methods {
    ($is_scope:ident) => {
        #[inline(always)]
        pub(crate) fn is_unallocated(&self) -> bool {
            !GUARANTEED_ALLOCATED && self.chunk.get().is_unallocated()
        }

        $crate::condition! {
            if $is_scope {
                #[doc = include_str!("docs/stats.md")]
                #[must_use]
                #[inline(always)]
                pub fn stats(&self) -> Stats<'a, UP> {
                    Stats {
                        current: crate::Chunk::new(self.as_scope()),
                    }
                }
            } else {
                #[doc = include_str!("docs/stats.md")]
                #[must_use]
                #[inline(always)]
                pub fn stats(&self) -> Stats<UP> {
                    self.as_scope().stats()
                }
            }
        }

        #[doc = include_str!("docs/allocator.md")]
        #[must_use]
        #[inline(always)]
        pub fn allocator(&self) -> &A {
            unsafe { self.chunk.get().allocator().as_ref() }
        }

        /// Wraps `&self` in [`WithoutDealloc`] so that [`deallocate`] becomes a no-op.
        ///
        /// [`deallocate`]: allocator_api2::alloc::Allocator::deallocate
        pub fn without_dealloc(&self) -> WithoutDealloc<&Self> {
            WithoutDealloc(self)
        }

        /// Wraps `&self` in [`WithoutShrink`] so that [`shrink`] becomes a no-op.
        ///
        /// [`shrink`]: allocator_api2::alloc::Allocator::shrink
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

            use fn $generic:ident
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
            #[cfg(not(no_global_oom_handling))]
            pub fn $infallible
            $(<$($generic_params)*>)?
            ($(&mut $self_mut,)?  $($arg_pat: $arg_ty),*) $(-> $return_ty)?
            $(where $($where)*)?
            {
                $crate::infallible(Self::$generic($($self_mut,)? $($arg_pat),*))
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
            $(<$($generic_params)*>)?
            ($(&mut $self_mut,)? $($arg_pat: $arg_ty),*)
            -> $crate::wrap_result!($($return_ty)?, allocator_api2::alloc::AllocError)
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
                    #[cfg(not(no_global_oom_handling))]
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
                        $crate::infallible($crate::last!($($self)+).$generic($($arg_pat),*))
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
                    #[cfg(not(no_global_oom_handling))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_fill(3, "ho")?;
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_slice_fill
    use fn generic_alloc_slice_fill<{T: Clone}>(&self, len: usize, value: T) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use <code>([try_](Self::try_alloc_slice_fill))[alloc_slice_fill](Self::alloc_slice_fill)</code>. If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill_with::<i32>(3, Default::default);
    /// assert_eq!(allocated, [0, 0, 0]);
    /// ```
    for fn alloc_slice_fill_with
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// let allocated = bump.alloc_str("Hello world!");
    /// assert_eq!(allocated, "Hello world!");
    /// ```
    for fn alloc_str
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_str("Hello world!")?;
    /// assert_eq!(allocated, "Hello world!");
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_str
    use fn generic_alloc_str(&self, src: &str) -> BumpBox<str> | BumpBox<'a, str>;

    /// Allocate a `str` from format arguments.
    impl
    /// For better performance prefer [`alloc_fmt_mut`](Bump::alloc_fmt_mut).
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
    /// For better performance prefer [`try_alloc_fmt_mut`](Bump::try_alloc_fmt_mut).
    do errors
    /// Errors if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// Unlike [`alloc_fmt`](Self::alloc_fmt), this function requires a mutable `Bump(Scope)`.
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
    /// Unlike [`try_alloc_fmt`](Bump::try_alloc_fmt), this function requires a mutable `Bump(Scope)`.
    do errors
    /// Errors if a formatting trait implementation returned an error.
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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

    /// Allocate elements of an iterator into a slice.
    impl
    /// For better performance prefer [`alloc_iter_exact`](Bump::alloc_iter_exact) or <code>[alloc_iter_mut](Bump::alloc_iter_mut)</code>.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    for fn alloc_iter
    /// For better performance prefer [`try_alloc_iter_exact`](Bump::try_alloc_iter_exact) or <code>[try_alloc_iter_mut](Bump::try_alloc_iter_mut)</code>.
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_iter
    use fn generic_alloc_iter<{T}>(&self, iter: impl IntoIterator<Item = T>) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    do panics
    /// Panics if the supplied iterator returns fewer elements than it promised.
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// Unlike [`alloc_iter`](Bump::alloc_iter), this function requires a mutable `Bump(Scope)`.
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`](Bump::alloc_iter_mut_rev) or [`alloc_iter_exact`](Bump::alloc_iter_exact) to avoid a shift of the slice.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    for fn alloc_iter_mut
    /// Unlike [`try_alloc_iter`](Bump::try_alloc_iter), this function requires a mutable `Bump(Scope)`.
    ///
    /// When bumping downwards, prefer [`try_alloc_iter_mut_rev`](Bump::try_alloc_iter_mut_rev) or [`try_alloc_iter_exact`](Bump::try_alloc_iter_exact) to avoid a shift of the slice.
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_mut([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
    /// ```
    for fn try_alloc_iter_mut
    use fn generic_alloc_iter_mut<{T}>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<[T]> | BumpBox<'a, [T]>;

    /// Allocate elements of an iterator into a slice in reverse order.
    impl
    ///
    /// When bumping upwards, prefer [`alloc_iter_mut`](Self::alloc_iter_mut) or [`alloc_iter_exact`](Self::alloc_iter_exact) to avoid a shift of the slice.
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut_rev([1, 2, 3]);
    /// assert_eq!(slice, [3, 2, 1]);
    /// ```
    for fn alloc_iter_mut_rev
    ///
    /// When bumping upwards, prefer [`try_alloc_iter_mut`](Self::try_alloc_iter_mut) or [`try_alloc_iter_exact`](Self::try_alloc_iter_exact) to avoid a shift of the slice.
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// let mut string = bump.alloc_fixed_string(12);
    /// string.push_str("Hello");
    /// string.push_str(" world!");
    /// assert_eq!(string, "Hello world!");
    /// ```
    for fn alloc_fixed_string
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut string = bump.try_alloc_fixed_string(12)?;
    /// string.push_str("Hello");
    /// string.push_str(" world!");
    /// assert_eq!(string, "Hello world!");
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
    /// assert!(bump.stats().capacity() > 4096);
    /// ```
    for fn reserve_bytes
    do examples
    /// ```
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
    /// # use bump_scope::{ Bump };
    /// let bump: Bump = Bump::try_new()?;
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.try_reserve_bytes(4096)?;
    /// assert!(bump.stats().capacity() > 4096);
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
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

/// We don't use `document-features` the usual way because then we can't have our features
/// be copied into the `README.md` via [`cargo-rdme`](https://github.com/orium/cargo-rdme).
#[test]
#[ignore = "this is not a real test, it's just to insert documentation"]
fn insert_feature_docs() {
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
