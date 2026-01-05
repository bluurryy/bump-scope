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

    /// The minimum alignment.
    type MinimumAlignment: SupportedMinimumAlignment;

    /// The bump direction.
    type Up: Boolean;

    /// Whether the allocator is guaranteed to have a chunk allocated and thus is allowed to create scopes.
    type GuaranteedAllocated: Boolean;

    /// Whether the allocator tries to free allocations.
    type Deallocates: Boolean;

    /// Changes the minimum alignment.
    type WithMinimumAlignment<const NEW_MIN_ALIGN: usize>: BumpAllocatorSettings
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment;

    /// Changes the bump direction.
    type WithUp<const VALUE: bool>: BumpAllocatorSettings<Up = Bool<VALUE>>;

    /// Changes whether the allocator is guaranteed to have a chunk allocated and thus is allowed to create scopes.
    type WithGuaranteedAllocated<const VALUE: bool>: BumpAllocatorSettings<GuaranteedAllocated = Bool<VALUE>>;

    /// Changes whether the allocator tries to free allocations.
    type WithDeallocates<const VALUE: bool>: BumpAllocatorSettings<Deallocates = Bool<VALUE>>;
}

/// Implementor of [`BumpAllocatorSettings`].
///
/// - **`MIN_ALIGN`** — the alignment maintained for the bump pointer, see [What is minimum alignment?](crate#what-is-minimum-alignment)
/// - **`UP`** — the bump direction, see [Bumping upwards or downwards?](crate#bumping-upwards-or-downwards)
/// - **`GUARANTEED_ALLOCATED`** — see [What does *guaranteed allocated* mean?](crate#what-does-guaranteed-allocated-mean)
/// - **`DEALLOCATES`** — toggles deallocation and shrinking for collections,
///   [`alloc_iter`](crate::Bump::alloc_iter) and
///   <code>[alloc_](crate::Bump::alloc_fmt)([cstr_](crate::Bump::alloc_cstr_fmt))[fmt](crate::Bump::alloc_fmt)</code>
pub struct BumpSettings<
    const MIN_ALIGN: usize = 1,
    const UP: bool = true,
    const GUARANTEED_ALLOCATED: bool = true,
    const DEALLOCATES: bool = true,
>;

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, const DEALLOCATES: bool> Sealed
    for BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, DEALLOCATES>
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, const DEALLOCATES: bool> BumpAllocatorSettings
    for BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    type MinimumAlignment = MinimumAlignment<MIN_ALIGN>;
    type Up = Bool<UP>;
    type GuaranteedAllocated = Bool<GUARANTEED_ALLOCATED>;
    type Deallocates = Bool<DEALLOCATES>;

    type WithMinimumAlignment<const VALUE: usize>
        = BumpSettings<VALUE, UP, GUARANTEED_ALLOCATED, DEALLOCATES>
    where
        MinimumAlignment<VALUE>: SupportedMinimumAlignment;
    type WithUp<const VALUE: bool> = BumpSettings<MIN_ALIGN, VALUE, GUARANTEED_ALLOCATED, DEALLOCATES>;
    type WithGuaranteedAllocated<const VALUE: bool> = BumpSettings<MIN_ALIGN, UP, VALUE, DEALLOCATES>;
    type WithDeallocates<const VALUE: bool> = BumpSettings<MIN_ALIGN, UP, GUARANTEED_ALLOCATED, VALUE>;
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
