//! Contains types to configure a bump allocation.
//!
//! You can configure various settings of the bump allocator:
//! - **`MIN_ALIGN`** — see [What is minimum alignment?](#what-is-minimum-alignment)
//! - **`UP`** — see [Bumping upwards or downwards?](#bumping-upwards-or-downwards)
//! - **`GUARANTEED_ALLOCATED`** — see [When is a bump allocator *guaranteed allocated*?](#when-is-a-bump-allocator-guaranteed-allocated)
//! - **`DEALLOCATES`** — toggles deallocation
//! - **`SHRINKS`** — toggles shrinking, including for temporary collections used in [`alloc_iter`](crate::Bump::alloc_iter), [`alloc_fmt`](crate::Bump::alloc_fmt), etc.
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
//!     /* DEALLOCATES */ false,
//!     /* SHRINKS */ false,
//! >;
//!
//! type MyBump = Bump<Global, MyBumpSettings>;
//!
//! let bump = MyBump::unallocated();
//!
//! # let str =
//! bump.alloc_str("Hello, world!");
//! # assert_eq!(str, "Hello, world!");
//! ```
//!
//! # What is minimum alignment?
//! Minimum alignment is the alignment the bump pointer maintains when doing allocations.
//!
//! When allocating a type in a bump allocator with a sufficient minimum alignment,
//! the bump pointer will not have to be aligned for the allocation but the allocation size
//! will need to be rounded up to the next multiple of the minimum alignment.
//!
//! The minimum alignment is controlled by the generic parameter `const MIN_ALIGN: usize`. By default, `MIN_ALIGN` is `1`.
//!
//! For the performance impact see [crates/callgrind-benches][benches].
//!
//! # Bumping upwards or downwards?
//! Bump direction is controlled by the generic parameter `const UP: bool`. By default, `UP` is `true`, so the allocator bumps upwards.
//!
//! Bumping upwards has the advantage that the most recent allocation can be grown and shrunk in place.
//! This benefits collections as well as <code>[alloc_iter](Bump::alloc_iter)([_mut](Bump::alloc_iter_mut))</code> and <code>[alloc_fmt](Bump::alloc_fmt)([_mut](Bump::alloc_fmt_mut))</code>
//! with the exception of [`MutBumpVecRev`] and [`alloc_iter_mut_rev`](Bump::alloc_iter_mut_rev) which
//! can be grown and shrunk in place if and only if bumping downwards.
//!
//! Bumping downwards can be done in less instructions.
//!
//! For the performance impact see [crates/callgrind-benches][benches].
//!
//! # When is a bump allocator *guaranteed allocated*?
//!
//! A *guaranteed allocated* bump allocator will own at least one chunk that it has allocated
//! from its base allocator.
//!
//! The constructors [`new`], [`with_size`], [`with_capacity`] and their variants always allocate
//! one chunk from the base allocator.
//!
//! The exception is the [`unallocated`] constructor which creates a `Bump` without allocating any
//! chunks. Such a `Bump` will have the `GUARANTEED_ALLOCATED` parameter set to `false`
//! which will make the [`scoped`], [`scoped_aligned`], [`aligned`] and [`scope_guard`] methods unavailable.
//!
//! You can turn any non-`GUARANTEED_ALLOCATED` bump allocator into a guaranteed allocated one using
//! [`as_guaranteed_allocated`], [`as_mut_guaranteed_allocated`] or [`into_guaranteed_allocated`].
//!
//! The point of this parameter is that `Bump`s can be `const` constructed and constructed without allocating.
//! At the same time `Bump`s that have already allocated a chunk don't suffer additional runtime checks.
//!
//! [benches]: https://github.com/bluurryy/bump-scope/tree/main/crates/callgrind-benches
//! [`new`]: Bump::new
//! [`with_size`]: Bump::with_size
//! [`with_capacity`]: Bump::with_capacity
//! [`unallocated`]: Bump::unallocated
//! [`scoped`]: Bump::scoped
//! [`scoped_aligned`]: Bump::scoped_aligned
//! [`aligned`]: Bump::aligned
//! [`scope_guard`]: Bump::scope_guard
//! [`as_guaranteed_allocated`]: Bump::as_guaranteed_allocated
//! [`as_mut_guaranteed_allocated`]: Bump::as_mut_guaranteed_allocated
//! [`into_guaranteed_allocated`]: Bump::into_guaranteed_allocated
//! [`BumpSettings`]: crate::settings::BumpSettings

// This settings situation could be improved with the nightly features
// [`generic_const_exprs`](https://github.com/rust-lang/rust/issues/76560) and
// [`associated_const_equality`](https://github.com/rust-lang/rust/issues/92827).

use crate::ArrayLayout;

trait Sealed {}

/// The trait powering bump allocator configuration.
///
/// The setting values are provided as associated constants.
///
/// Additionally they are provided as types, solely so they can be used in equality bounds like:
/// ```ignore
/// S: BumpAllocatorSettings<GuaranteedAllocated = True>
/// ```
/// doing the same with associated constants is not yet possible, see the nightly feature [`associated_const_equality`](https://github.com/rust-lang/rust/issues/92827).
/// ```ignore,warn
/// // not possible right now
/// S: BumpAllocatorSettings<GUARANTEED_ALLOCATED = true>
/// ```
#[expect(private_bounds)]
pub trait BumpAllocatorSettings: Sealed {
    /// The minimum alignment.
    const MIN_ALIGN: usize = Self::MinimumAlignment::VALUE;

    /// The bump direction.
    const UP: bool = Self::Up::VALUE;

    /// Whether the allocator is guaranteed to have a chunk allocated and thus is allowed to create scopes.
    const GUARANTEED_ALLOCATED: bool = Self::GuaranteedAllocated::VALUE;

    /// Whether the allocator tries to free allocations.
    const DEALLOCATES: bool = Self::Deallocates::VALUE;

    /// Whether the allocator tries to shrink allocations.
    const SHRINKS: bool = Self::Shrinks::VALUE;

    /// The minimum alignment.
    type MinimumAlignment: SupportedMinimumAlignment;

    /// The bump direction.
    type Up: Boolean;

    /// Whether the allocator is guaranteed to have a chunk allocated and thus is allowed to create scopes.
    type GuaranteedAllocated: Boolean;

    /// Whether the allocator tries to free allocations.
    type Deallocates: Boolean;

    /// Whether the allocator tries to shrink allocations.
    type Shrinks: Boolean;

    /// Changes the minimum alignment.
    type WithMinimumAlignment<const NEW_MIN_ALIGN: usize>: BumpAllocatorSettings<
            Up = Self::Up,
            GuaranteedAllocated = Self::GuaranteedAllocated,
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
            Deallocates = Self::Deallocates,
            Shrinks = Self::Shrinks,
        >;

    /// Changes whether the allocator is guaranteed to have a chunk allocated and thus is allowed to create scopes.
    type WithGuaranteedAllocated<const VALUE: bool>: BumpAllocatorSettings<
            MinimumAlignment = Self::MinimumAlignment,
            Up = Self::Up,
            GuaranteedAllocated = Bool<VALUE>,
            Deallocates = Self::Deallocates,
            Shrinks = Self::Shrinks,
        >;

    /// Changes whether the allocator tries to free allocations.
    type WithDeallocates<const VALUE: bool>: BumpAllocatorSettings<
            MinimumAlignment = Self::MinimumAlignment,
            Up = Self::Up,
            GuaranteedAllocated = Self::GuaranteedAllocated,
            Deallocates = Bool<VALUE>,
            Shrinks = Self::Shrinks,
        >;

    /// Changes whether the allocator tries to shrink allocations.
    type WithShrinks<const VALUE: bool>: BumpAllocatorSettings<
            MinimumAlignment = Self::MinimumAlignment,
            Up = Self::Up,
            GuaranteedAllocated = Self::GuaranteedAllocated,
            Deallocates = Self::Deallocates,
            Shrinks = Bool<VALUE>,
        >;
}

/// Implementor of [`BumpAllocatorSettings`].
///
/// See the [module documentation](crate::settings) for how to use this type.
pub struct BumpSettings<
    const MIN_ALIGN: usize = 1,
    const UP: bool = true,
    const GUARANTEED_ALLOCATED: bool = true,
    const DEALLOCATES: bool = true,
    const SHRINKS: bool = true,
>;

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, const DEALLOCATES: bool, const SHRINKS: bool>
    Sealed for BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, DEALLOCATES, SHRINKS>
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, const DEALLOCATES: bool, const SHRINKS: bool>
    BumpAllocatorSettings for BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, DEALLOCATES, SHRINKS>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    type MinimumAlignment = MinimumAlignment<MIN_ALIGN>;
    type Up = Bool<UP>;
    type GuaranteedAllocated = Bool<GUARANTEED_ALLOCATED>;
    type Deallocates = Bool<DEALLOCATES>;
    type Shrinks = Bool<SHRINKS>;

    type WithMinimumAlignment<const VALUE: usize>
        = BumpSettings<VALUE, UP, GUARANTEED_ALLOCATED, DEALLOCATES, SHRINKS>
    where
        MinimumAlignment<VALUE>: SupportedMinimumAlignment;
    type WithUp<const VALUE: bool> = BumpSettings<MIN_ALIGN, VALUE, GUARANTEED_ALLOCATED, DEALLOCATES, SHRINKS>;
    type WithGuaranteedAllocated<const VALUE: bool> = BumpSettings<MIN_ALIGN, UP, VALUE, DEALLOCATES, SHRINKS>;
    type WithDeallocates<const VALUE: bool> = BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, VALUE, SHRINKS>;
    type WithShrinks<const VALUE: bool> = BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, DEALLOCATES, VALUE>;
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
#[derive(Clone, Copy)]
pub struct MinimumAlignment<const ALIGNMENT: usize>;

mod supported_minimum_alignment {
    use crate::ArrayLayout;

    pub trait Sealed {
        /// We'd be fine with just an [`core::ptr::Alignment`], but that's not stable.
        #[doc(hidden)]
        const LAYOUT: ArrayLayout;
    }
}

/// Statically guarantees that a minimum alignment is supported.
///
/// This trait is *sealed*: the list of implementors below is total. Users do not have the ability to mark additional
/// `MinimumAlignment<N>` values as supported. Only bump allocators with the supported minimum alignments are constructable.
pub trait SupportedMinimumAlignment: supported_minimum_alignment::Sealed + Copy {
    /// The minimum alignment in bytes.
    const VALUE: usize;
}

macro_rules! supported_alignments {
    ($($i:literal)*) => {
        $(
            impl supported_minimum_alignment::Sealed for MinimumAlignment<$i> {
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
