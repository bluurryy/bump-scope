//! Contains types to configure bump allocation.
//!
//! You can configure various settings of the bump allocator:
//! - **`MIN_ALIGN`** *default: 1* —
//!   The alignment the bump pointer maintains when doing allocations.
//!
//!   When allocating a type in a bump allocator with a sufficient minimum alignment,
//!   the bump pointer will not have to be aligned for the allocation but the allocation size
//!   will need to be rounded up to the next multiple of the minimum alignment.
//!
//!   For the performance impact see [crates/callgrind-benches][benches].
//! - **`UP`** *default: true* —
//!   Controls the bump direction.
//!
//!   Bumping upwards has the advantage that the most recent allocation can be grown and shrunk in place.
//!   This benefits collections as well as <code>[alloc_iter][]([_mut][alloc_iter_mut])</code> and <code>[alloc_fmt][]([_mut][alloc_fmt_mut])</code>
//!   with the exception of [`MutBumpVecRev`] and [`alloc_iter_mut_rev`] which
//!   can be grown and shrunk in place if and only if bumping downwards.
//!
//!   Bumping downwards can be done in less instructions.
//!
//!   For the performance impact see [crates/callgrind-benches][benches].
//! - **`GUARANTEED_ALLOCATED`** *default: true* —
//!   Whether at least one chunk has been allocated.
//!
//!   The <code>[unallocated]</code> constructor will create a bump allocator without allocating a chunk.
//!   It will only compile when `GUARANTEED_ALLOCATED` is `false`.
//!
//!   The constructors <code>[new]\([_in][new_in])</code>, <code>[default]</code>, <code>[with_size]\([_in][with_size_in])</code> and <code>[with_capacity]\([_in][with_capacity_in])</code>
//!   will allocate a chunk and are always available.
//!
//!   Setting `GUARANTEED_ALLOCATED` to `false` adds additional checks and code paths for handling the no-chunk-allocated state when calling [`reset_to`], exiting scopes or calling [`by_value`].
//! - **`CLAIMABLE`** *default: true* — Enables the [`claim`] api.
//!
//!   When this is `false`, calling `claim` will fail to compile.
//! - **`DEALLOCATES`** *default: true* — Toggles deallocation.
//!
//!   When this is `false`, [`Allocator::deallocate`] does nothing.
//! - **`SHRINKS`** *default: true* — Toggles shrinking.
//!
//!   When this is `false`, [`Allocator::shrink`] and [`BumpAllocatorTyped::shrink_slice`] do nothing.
//!   Calling `shrink` with a new layout of a greater alignment may still reallocate.
//!   
//!   This also affects the temporary collections used in [`alloc_iter`][alloc_iter], [`alloc_fmt`][alloc_fmt], etc.
//! - **`MINIMUM_CHUNK_SIZE`** *default: 512* — Configures the minimum chunk size.
//!
//!   The final chunk size is calculated like described in [`with_size`],
//!   thus it can be slightly smaller than requested.
//!
//! # Example
//!
//! You can configure the allocator settings using [`BumpSettings`]:
//! ```
//! use bump_scope::{ Bump, alloc::Global, settings::BumpSettings };
//!
//! type MyBumpSettings = BumpSettings<
//!     /* MIN_ALIGN */ 8,
//!     /* UP */ false,
//!     /* GUARANTEED_ALLOCATED */ false,
//!     /* CLAIMABLE */ false,
//!     /* DEALLOCATES */ false,
//!     /* SHRINKS */ false,
//!     /* MINIMUM_CHUNK_SIZE */ 4096,
//! >;
//!
//! type MyBump = Bump<Global, MyBumpSettings>;
//!
//! let bump = MyBump::unallocated();
//! assert_eq!(bump.stats().size(), 0);
//!
//! # let str =
//! bump.alloc_str("Hello, world!");
//! # assert_eq!(str, "Hello, world!");
//! assert_eq!(bump.stats().size(), 4096 - size_of::<[usize; 2]>());
//! ```
//!
//! [benches]: https://github.com/bluurryy/bump-scope/tree/main/crates/callgrind-benches
//! [unallocated]: crate::Bump::unallocated
//! [new]: crate::Bump::new
//! [new_in]: crate::Bump::new_in
//! [default]: crate::Bump::default
//! [with_size]: crate::Bump::with_size
//! [`with_size`]: crate::Bump::with_size
//! [with_size_in]: crate::Bump::with_size_in
//! [with_capacity]: crate::Bump::with_capacity
//! [with_capacity_in]: crate::Bump::with_capacity_in
//! [`scoped`]: crate::Bump::scoped
//! [`scoped_aligned`]: crate::Bump::scoped_aligned
//! [`aligned`]: crate::Bump::aligned
//! [`scope_guard`]: crate::Bump::scope_guard
//! [`BumpSettings`]: crate::settings::BumpSettings
//! [`MutBumpVecRev`]: crate::MutBumpVecRev
//! [`reset_to`]: crate::traits::BumpAllocatorCore::reset_to
//! [`claim`]: crate::traits::BumpAllocatorScope::claim
//! [alloc_iter]: crate::traits::BumpAllocatorTypedScope::alloc_iter
//! [alloc_iter_mut]: crate::traits::MutBumpAllocatorTypedScope::alloc_iter_mut
//! [alloc_fmt]: crate::traits::BumpAllocatorTypedScope::alloc_fmt
//! [alloc_fmt_mut]: crate::traits::MutBumpAllocatorTypedScope::alloc_fmt_mut
//! [`alloc_iter_mut_rev`]: crate::traits::MutBumpAllocatorTypedScope::alloc_iter_mut_rev
//! [`Allocator::allocate`]: crate::alloc::Allocator::allocate
//! [`Allocator::deallocate`]: crate::alloc::Allocator::deallocate
//! [`Allocator::shrink`]: crate::alloc::Allocator::shrink
//! [`BumpAllocatorTyped::shrink_slice`]: crate::traits::BumpAllocatorTyped::shrink_slice
//! [`by_value`]: crate::BumpScope::by_value

use crate::ArrayLayout;

trait Sealed {}

/// The trait powering bump allocator configuration.
///
/// Read the [module documentation] to learn about the settings.
///
/// The setting values are provided as associated constants.
///
/// Additionally they are provided as types so they can be used in equality bounds like this:
/// ```ignore
/// S: BumpAllocatorSettings<GuaranteedAllocated = True>
/// ```
/// Doing the same with associated constants is not (yet) possible:
/// ```ignore,warn
/// // won't compile on stable
/// S: BumpAllocatorSettings<GUARANTEED_ALLOCATED = true>
/// ```
///
/// In the future this trait could be simplified when the following features are stabilized:
/// - [`generic_const_exprs`]
/// - [`associated_const_equality`]
///
/// [`generic_const_exprs`]: https://github.com/rust-lang/rust/issues/76560
/// [`associated_const_equality`]: https://github.com/rust-lang/rust/issues/92827
/// [module documentation]: crate::settings
#[expect(private_bounds)]
pub trait BumpAllocatorSettings: Sealed {
    /// The minimum alignment.
    const MIN_ALIGN: usize = Self::MinimumAlignment::VALUE;

    /// The bump direction.
    const UP: bool = Self::Up::VALUE;

    /// Whether the allocator is guaranteed to have a chunk allocated.
    const GUARANTEED_ALLOCATED: bool = Self::GuaranteedAllocated::VALUE;

    /// Whether the allocator can be [claimed].
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    const CLAIMABLE: bool = Self::Claimable::VALUE;

    /// Whether the allocator tries to free allocations.
    const DEALLOCATES: bool = Self::Deallocates::VALUE;

    /// Whether the allocator tries to shrink allocations.
    const SHRINKS: bool = Self::Shrinks::VALUE;

    /// The minimum size for bump allocation chunk.
    const MINIMUM_CHUNK_SIZE: usize;

    /// The minimum alignment.
    type MinimumAlignment: SupportedMinimumAlignment;

    /// The bump direction.
    type Up: Boolean;

    /// Whether the allocator is guaranteed to have a chunk allocated.
    type GuaranteedAllocated: Boolean;

    /// Whether the allocator can be [claimed].
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    type Claimable: Boolean;

    /// Whether the allocator tries to free allocations.
    type Deallocates: Boolean;

    /// Whether the allocator tries to shrink allocations.
    type Shrinks: Boolean;

    /// Changes the minimum alignment.
    type WithMinimumAlignment<const NEW_MIN_ALIGN: usize>: BumpAllocatorSettings<
            MinimumAlignment = MinimumAlignment<NEW_MIN_ALIGN>,
            Up = Self::Up,
            GuaranteedAllocated = Self::GuaranteedAllocated,
            Claimable = Self::Claimable,
            Deallocates = Self::Deallocates,
            Shrinks = Self::Shrinks,
        >
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment;

    /// Changes the bump direction.
    type WithUp<const VALUE: bool>: BumpAllocatorSettings<
            MinimumAlignment = Self::MinimumAlignment,
            Up = Bool<VALUE>,
            GuaranteedAllocated = Self::GuaranteedAllocated,
            Claimable = Self::Claimable,
            Deallocates = Self::Deallocates,
            Shrinks = Self::Shrinks,
        >;

    /// Changes whether the allocator is guaranteed to have a chunk allocated.
    type WithGuaranteedAllocated<const VALUE: bool>: BumpAllocatorSettings<
            MinimumAlignment = Self::MinimumAlignment,
            Up = Self::Up,
            GuaranteedAllocated = Bool<VALUE>,
            Claimable = Self::Claimable,
            Deallocates = Self::Deallocates,
            Shrinks = Self::Shrinks,
        >;

    /// Changes whether the allocator can be [claimed].
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    type WithClaimable<const VALUE: bool>: BumpAllocatorSettings<
            MinimumAlignment = Self::MinimumAlignment,
            Up = Self::Up,
            GuaranteedAllocated = Self::GuaranteedAllocated,
            Claimable = Bool<VALUE>,
            Deallocates = Self::Deallocates,
            Shrinks = Self::Shrinks,
        >;

    /// Changes whether the allocator tries to free allocations.
    type WithDeallocates<const VALUE: bool>: BumpAllocatorSettings<
            MinimumAlignment = Self::MinimumAlignment,
            Up = Self::Up,
            GuaranteedAllocated = Self::GuaranteedAllocated,
            Claimable = Self::Claimable,
            Deallocates = Bool<VALUE>,
            Shrinks = Self::Shrinks,
        >;

    /// Changes whether the allocator tries to shrink allocations.
    type WithShrinks<const VALUE: bool>: BumpAllocatorSettings<
            MinimumAlignment = Self::MinimumAlignment,
            Up = Self::Up,
            GuaranteedAllocated = Self::GuaranteedAllocated,
            Claimable = Self::Claimable,
            Deallocates = Self::Deallocates,
            Shrinks = Bool<VALUE>,
        >;

    /// Changes the minimum chunk size.
    type WithMinimumChunkSize<const VALUE: usize>: BumpAllocatorSettings<
            MinimumAlignment = Self::MinimumAlignment,
            Up = Self::Up,
            GuaranteedAllocated = Self::GuaranteedAllocated,
            Claimable = Self::Claimable,
            Deallocates = Self::Deallocates,
            Shrinks = Self::Shrinks,
        >;
}

/// Implementor of [`BumpAllocatorSettings`].
///
/// See the [module documentation](crate::settings) for how to use this type.
pub struct BumpSettings<
    const MIN_ALIGN: usize = 1,
    const UP: bool = true,
    const GUARANTEED_ALLOCATED: bool = true,
    const CLAIMABLE: bool = true,
    const DEALLOCATES: bool = true,
    const SHRINKS: bool = true,
    const MINIMUM_CHUNK_SIZE: usize = 512,
>;

impl<
    const MIN_ALIGN: usize,
    const UP: bool,
    const GUARANTEED_ALLOCATED: bool,
    const CLAIMABLE: bool,
    const DEALLOCATES: bool,
    const SHRINKS: bool,
    const MINIMUM_CHUNK_SIZE: usize,
> Sealed for BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, CLAIMABLE, DEALLOCATES, SHRINKS, MINIMUM_CHUNK_SIZE>
{
}

impl<
    const MIN_ALIGN: usize,
    const UP: bool,
    const GUARANTEED_ALLOCATED: bool,
    const CLAIMABLE: bool,
    const DEALLOCATES: bool,
    const SHRINKS: bool,
    const MINIMUM_CHUNK_SIZE: usize,
> BumpAllocatorSettings
    for BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, CLAIMABLE, DEALLOCATES, SHRINKS, MINIMUM_CHUNK_SIZE>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    const MINIMUM_CHUNK_SIZE: usize = MINIMUM_CHUNK_SIZE;

    type MinimumAlignment = MinimumAlignment<MIN_ALIGN>;
    type Up = Bool<UP>;
    type GuaranteedAllocated = Bool<GUARANTEED_ALLOCATED>;
    type Claimable = Bool<CLAIMABLE>;
    type Deallocates = Bool<DEALLOCATES>;
    type Shrinks = Bool<SHRINKS>;

    type WithMinimumAlignment<const VALUE: usize>
        = BumpSettings<VALUE, UP, GUARANTEED_ALLOCATED, CLAIMABLE, DEALLOCATES, SHRINKS, MINIMUM_CHUNK_SIZE>
    where
        MinimumAlignment<VALUE>: SupportedMinimumAlignment;
    type WithUp<const VALUE: bool> =
        BumpSettings<MIN_ALIGN, VALUE, GUARANTEED_ALLOCATED, CLAIMABLE, DEALLOCATES, SHRINKS, MINIMUM_CHUNK_SIZE>;
    type WithGuaranteedAllocated<const VALUE: bool> =
        BumpSettings<MIN_ALIGN, UP, VALUE, CLAIMABLE, DEALLOCATES, SHRINKS, MINIMUM_CHUNK_SIZE>;
    type WithClaimable<const VALUE: bool> =
        BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, VALUE, DEALLOCATES, SHRINKS, MINIMUM_CHUNK_SIZE>;
    type WithDeallocates<const VALUE: bool> =
        BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, CLAIMABLE, VALUE, SHRINKS, MINIMUM_CHUNK_SIZE>;
    type WithShrinks<const VALUE: bool> =
        BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, CLAIMABLE, DEALLOCATES, VALUE, MINIMUM_CHUNK_SIZE>;
    type WithMinimumChunkSize<const VALUE: usize> =
        BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, CLAIMABLE, DEALLOCATES, SHRINKS, VALUE>;
}

/// Either [`True`] or [`False`].
#[expect(private_bounds)]
pub trait Boolean: Sealed {
    /// The boolean's value.
    const VALUE: bool;
}

/// A type representing `true`.
pub type True = Bool<true>;

/// A type representing `false`.
pub type False = Bool<false>;

/// Used to create [`True`] and [`False`] types.
pub struct Bool<const VALUE: bool>;

impl<const VALUE: bool> Sealed for Bool<VALUE> {}

impl<const VALUE: bool> Boolean for Bool<VALUE> {
    const VALUE: bool = VALUE;
}

/// Specifies the current minimum alignment of a bump allocator.
pub struct MinimumAlignment<const ALIGNMENT: usize>;

mod supported_minimum_alignment {
    use crate::ArrayLayout;

    pub trait Sealed {
        /// We'd be fine with just an [`core::ptr::Alignment`], but that's not stable.
        #[doc(hidden)]
        #[expect(private_interfaces)]
        const LAYOUT: ArrayLayout;
    }
}

/// Statically guarantees that a minimum alignment is supported.
///
/// This trait is *sealed*: the list of implementors below is total. Users do not have the ability to mark additional
/// `MinimumAlignment<N>` values as supported. Only bump allocators with the supported minimum alignments are constructable.
pub trait SupportedMinimumAlignment: supported_minimum_alignment::Sealed {
    /// The minimum alignment in bytes.
    const VALUE: usize;
}

macro_rules! supported_alignments {
    ($($i:literal)*) => {
        $(
            impl supported_minimum_alignment::Sealed for MinimumAlignment<$i> {
                #[expect(private_interfaces)]
                const LAYOUT: ArrayLayout = match ArrayLayout::from_size_align(0, $i) {
                    Ok(layout) => layout,
                    Err(_) => unreachable!(),
                };
            }
            impl SupportedMinimumAlignment for MinimumAlignment<$i> {
                const VALUE: usize = $i;
            }
        )*
    };
}

supported_alignments!(1 2 4 8 16);
