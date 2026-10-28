//! Configure bump allocator parameters.
//!
//! # Example
//!
//! You can configure the bump allocator config with a [`BumpConfigBuilder`]:
//! ```
//! use std::alloc::Global;
//! use bump_scope::{Bump, BumpAllocator, config::{BumpConfig, BumpConfigBuilder}};
//!
//! const MY_CONFIG: BumpConfig = BumpConfigBuilder::new()
//!     .min_align(8)
//!     .upwards(false)
//!     .const_constructible(true)
//!     .claimable(false)
//!     .deallocates(false)
//!     .shrinks(false)
//!     .min_chunk_size(4096)
//!     .build();
//!
//! type MyBump = Bump<Global, MY_CONFIG>;
//!
//! let bump = MyBump::const_new();
//! assert_eq!(bump.stats().size(), 0);
//!
//! # let str =
//! bump.alloc_str("Hello, world!").into_ref();
//! # assert_eq!(str, "Hello, world!");
//! assert_eq!(bump.stats().size(), 4096 - size_of::<[usize; 2]>());
//! ```
#![deny(clippy::cast_possible_truncation)]

#[allow(unused_imports, reason = "for doc")]
use core::alloc::Allocator;
use core::fmt;

/// The const generic type used for configuring the bump allocator.
///
/// Currently all parameters are packed into an `u128` integer.
/// This is an implementation detail and must not be relied upon.
/// You also mustn't rely on the raw integer value, changes to how
/// the parameters map to the integer are not considered semver-breaking.
pub type BumpConfig = u128;

/// A type for creating [`BumpConfig`]s.
///
/// You can either start with the default config with `BumpConfigBuilder::new()`,
/// or use an existing config as the base with `BumpConfigBuilder(SOME_CONFIG)`.
pub struct BumpConfigBuilder(pub BumpConfig);

impl BumpConfigBuilder {
    /// Creates a builder based on the [`DEFAULT_BUMP_CONFIG`].
    #[must_use]
    #[expect(clippy::new_without_default, reason = "type is only meant to be used at compile time")]
    pub const fn new() -> Self {
        Self(DEFAULT_BUMP_CONFIG)
    }

    /// Returns the value usable as a const generic for [`Bump`].
    ///
    /// [`Bump`]: crate::Bump
    #[must_use]
    pub const fn build(self) -> BumpConfig {
        self.0
    }
}

macro_rules! if_exists {
    (
        $whatever:ident {
            $($then:tt)*
        } else {
            $($else:tt)*
        }
    ) => {
        $($then)*
    };
    (
        {
            $($then:tt)*
        } else {
            $($else:tt)*
        }
    ) => {
        $($else)*
    };
}

mod safe_cast {
    #![deny(clippy::cast_possible_truncation)]

    macro_rules! safe_cast {
        ($name:ident: $from:ident -> $to:ident) => {
            pub(super) const fn $name(v: $from) -> $to {
                if $to::BITS < $from::BITS {
                    assert!(v < ($to::MAX as $from));
                }

                v as $to
            }
        };
    }

    safe_cast!(u128_as_usize: u128 -> usize);
}

/// If the `BumpConfig` contains bits that are not used by any bitfield, then something is probably going wrong.
/// For instance, when changing an `.aligned::<4>(...)` from v2 to a `with_config` from v3, you may forget
/// changing the const generic too. Annoyingly `4` is valid for `BumpConfig`, since it's just a u128.
///
/// To make such an error explicit, we moved all fields to the high bits. If any low bits are set, we error.
const fn assert_valid_config(config: BumpConfig) {
    assert!(!fields::unused_bits_are_set(config), "invalid bump config");
}

macro_rules! config {
    ($config:path, $name:ident) => {
        const { $crate::config::read::$name($config) }
    };
    ($name:ident) => {
        const { $crate::config::read::$name(C) }
    };
}

pub(crate) use config;

macro_rules! bit_fields {
    (
        $(#[$mod_fields_attr:meta])* $mod_fields_vis:vis mod $mod_fields:ident;
        $(#[$mod_read_attr:meta])* $mod_read_vis:vis mod $mod_read:ident;
        $(#[$const_default_attr:meta])* $const_default_vis:vis const $const_default:ident;

        $(
            $(#[$attr:meta])*
            $name:ident: $ty:ty {
                default: $default:expr,
                bits: $bits:literal,
                encode: |$en_val:ident| $en_expr:expr,
                decode: |$de_val:ident| $de_expr:expr,
                $(validate: |$vl_val:ident| $vl_expr:expr,)?
            }
        )*
    ) => {
        $(#[$mod_fields_attr])*
        $mod_fields_vis mod $mod_fields {
            const USED_BITS: u32 = 0 $(+$bits)*;
            const UNUSED_BITS: u32 = 128 - USED_BITS;
            const UNUSED_MASK: u128 = (1 << UNUSED_BITS) - 1;

            pub(crate) const fn unused_bits_are_set(bits: u128) -> bool {
                bits & UNUSED_MASK != 0
            }

            bit_fields!(
                @iter 0;

                $($name: $ty {
                    default: $default,
                    bits: $bits,
                    encode: |$en_val| $en_expr,
                    decode: |$de_val| $de_expr,
                    $(validate: |$vl_val| $vl_expr,)?
                })*
            );
        }

        $(#[$mod_read_attr])*
        $mod_read_vis mod $mod_read {
            use super::{BumpConfig, assert_valid_config, $mod_fields};

            $(
                #[doc = concat!(
                    "Reads the [`", stringify!($name), "`] parameter.\n",
                    "\n",
                    "[`", stringify!($name), "`]: crate::config::BumpConfigBuilder::", stringify!($name)
                )]
                #[must_use]
                pub const fn $name(config: BumpConfig) -> $ty {
                    assert_valid_config(config);
                    $mod_fields::$name::decode(config)
                }
            )*
        }

        $(#[$const_default_attr])*
        $const_default_vis const $const_default: BumpConfig = BumpConfigBuilder(0)
            $(.$name($default))*
            .build();

        impl BumpConfigBuilder {
            $(
                #[doc = concat!("*default: **", stringify!($default), "***\n\n")]
                $(#[$attr])*
                #[must_use]
                pub const fn $name(mut self, value: $ty) -> Self {
                    self.0 = $mod_fields::$name::encode(self.0, value);
                    self
                }
            )*
        }

        impl fmt::Debug for BumpConfigBuilder {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("BumpConfigBuilder")
                    $(.field(stringify!($name), &$mod_read::$name(self.0)))*
                    .finish()
            }
        }
    };
    (@iter $offset:expr;) => {};
    (@iter
        $offset:expr;
        $name:ident: $ty:ty {
            default: $default:expr,
            bits: $bits:literal,
            encode: |$en_val:ident| $en_expr:expr,
            decode: |$de_val:ident| $de_expr:expr,
            $(validate: |$vl_val:ident| $vl_expr:expr,)?
        }
        $($rest:tt)*
    ) => {
        pub(crate) mod $name {
            #[allow(unused_imports)]
            use super::super::safe_cast::*;

            pub(super) const BITS: u32 = $bits;
            pub(super) const OFFSET: u32 = $offset;
            const SHIFT: u32 = 128 - OFFSET - BITS;
            const MASK: u128 = ((1 << BITS) - 1) << SHIFT;

            const fn write(bits: u128, value: u128) -> u128 {
                // the value must only set bits within the mask
                assert!(value & !((1 << BITS) - 1) == 0, "too many bits used for value");

                let inherited = bits & !MASK;
                let overwrite = value << SHIFT;
                inherited | overwrite
            }

            const fn read(bits: u128) -> u128 {
                (bits & MASK) >> SHIFT
            }

            pub(crate) const fn encode(bits: u128, $en_val: $ty) -> u128 {
                if_exists!($($vl_val)? {
                    validate($en_val);
                } else {});

                let value_bits = $en_expr;
                write(bits, value_bits)
            }

            pub(crate) const fn decode(bits: u128) -> $ty {
                let $de_val = read(bits);

                if_exists!($($vl_val)? {{
                    let result = $de_expr;
                    validate(result);
                    result
                }} else {
                    $de_expr
                })
            }

            $(
                const fn validate($vl_val: $ty) {
                    $vl_expr;
                }
            )?
        }

        bit_fields!(
            @iter super::$name::OFFSET + super::$name::BITS;
            $($rest)*
        );
    };
}

bit_fields! {
    mod fields;

    /// Read configuration parameters.
    pub mod read;

    /// The default bump allocator configuration.
    ///
    /// The configuration values are:
    /// - [`min_align`]\: 1
    /// - [`upwards`]\: true
    /// - [`const_constructible`]\: false
    /// - [`claimable`]\: true
    /// - [`deallocates`]\: true
    /// - [`shrinks`]\: true
    /// - [`min_chunk_size`]\: 512
    ///
    /// [`min_align`]: BumpConfigBuilder::min_align
    /// [`upwards`]: BumpConfigBuilder::upwards
    /// [`const_constructible`]: BumpConfigBuilder::const_constructible
    /// [`claimable`]: BumpConfigBuilder::claimable
    /// [`deallocates`]: BumpConfigBuilder::deallocates
    /// [`shrinks`]: BumpConfigBuilder::shrinks
    /// [`min_chunk_size`]: BumpConfigBuilder::min_chunk_size
    pub const DEFAULT_BUMP_CONFIG;

    /// Sets the alignment the bump pointer maintains when doing allocations.
    ///
    /// Allocations are slightly faster when `align <= min_align`.
    ///
    /// Allocation sizes are rounded up to the next multiple of `min_align`,
    /// so allocations can take up more space with a higher `min_align`.
    ///
    /// For the performance impact see [crates/callgrind-benches][benches].
    ///
    /// [benches]: https://github.com/bluurryy/bump-scope/tree/main/crates/callgrind-benches
    min_align: usize {
        default: 1,
        bits: 3,
        encode: |v| v.highest_one().expect("due to validation, min_align is not zero") as u128,
        decode: |v| u128_as_usize(1 << v),
        validate: |v| {
            assert!(v.is_power_of_two(), "min_align must be a power of two");
            assert!(v <= 16, "min_align must be <= 16");
        },
    }

    /// Sets the bump direction.
    ///
    /// Bumping upwards has the advantage that the most recent allocation can be grown and shrunk in place.
    /// This benefits growable collections as well as <code>[alloc_iter][]([_mut][alloc_iter_mut])</code> and <code>[alloc_fmt][]([_mut][alloc_fmt_mut])</code>.
    ///
    /// Bumping downwards can be done in less instructions.
    ///
    /// For the performance impact see [crates/callgrind-benches][benches].
    ///
    /// [alloc_iter]: crate::traits::BumpAllocator::alloc_iter
    /// [alloc_iter_mut]: crate::traits::MutBumpAllocator::alloc_iter_mut
    /// [alloc_fmt]: crate::traits::BumpAllocator::alloc_fmt
    /// [alloc_fmt_mut]: crate::traits::MutBumpAllocator::alloc_fmt_mut
    /// [benches]: https://github.com/bluurryy/bump-scope/tree/main/crates/callgrind-benches
    upwards: bool {
        default: true,
        bits: 1,
        encode: |v| if v { 1 } else { 0 },
        decode: |v| v != 0,
    }

    /// Enables the [`const_new`] constructor.
    ///
    /// Setting `const_constructible` to `true` adds additional checks and code paths for handling
    /// the no-chunk-allocated state when exiting scopes, calling [`reset_to`] or calling [`by_value`].
    ///
    /// [`const_new`]: crate::Bump::const_new
    /// [new]: crate::Bump::new
    /// [new_in]: crate::Bump::new_in
    /// [default]: crate::Bump::default
    /// [with_size]: crate::Bump::with_size
    /// [`with_size`]: crate::Bump::with_size
    /// [with_size_in]: crate::Bump::with_size_in
    /// [with_capacity]: crate::Bump::with_capacity
    /// [with_capacity_in]: crate::Bump::with_capacity_in
    /// [`reset_to`]: crate::traits::BumpAllocator::reset_to
    /// [`by_value`]: crate::BumpScope::by_value
    const_constructible: bool {
        default: false,
        bits: 1,
        encode: |v| if v { 1 } else { 0 },
        decode: |v| v != 0,
    }

    /// Enables the [`claim`] api.
    ///
    /// When this is `false`, calling `claim` will fail to compile.
    ///
    /// [`claim`]: crate::BumpScope::claim
    claimable: bool {
        default: true,
        bits: 1,
        encode: |v| if v { 1 } else { 0 },
        decode: |v| v != 0,
    }

    /// Enables deallocation.
    ///
    /// When this is `false`, [`Allocator::deallocate`] does not free memory.
    deallocates: bool {
        default: true,
        bits: 1,
        encode: |v| if v { 1 } else { 0 },
        decode: |v| v != 0,
    }

    /// Enables shrinking.
    ///
    /// When this is `false`, [`Allocator::shrink`] does not free memory.
    /// Calling `shrink` with a new layout of a greater alignment may still reallocate.
    ///
    /// This also affects the temporary collections used in [`alloc_iter`], [`alloc_fmt`], etc.
    ///
    /// [`alloc_iter`]: crate::traits::BumpAllocator::alloc_iter
    /// [`alloc_fmt`]: crate::traits::BumpAllocator::alloc_fmt
    shrinks: bool {
        default: true,
        bits: 1,
        encode: |v| if v { 1 } else { 0 },
        decode: |v| v != 0,
    }

    /// Sets the minimum chunk size.
    ///
    /// The final chunk size is calculated like described in [`with_size`],
    /// thus it can be slightly smaller than this value.
    ///
    /// [`with_size`]: crate::Bump::with_size
    min_chunk_size: usize {
        default: 512,
        bits: 6,
        encode: |v| v.highest_one().expect("due to validation, min_chunk_size is not zero") as u128,
        decode: |v| u128_as_usize(1 << v),
        validate: |v| {
            assert!(v.is_power_of_two(), "min_chunk_size must be a power of two");
        },
    }
}

#[test]
#[rustfmt::skip]
fn test_roundtrip() {
    for min_align in [1, 2, 4, 8, 16] {
    for upwards in [false, true] {
    for const_constructible in [false, true] {
    for claimable in [false, true] {
    for deallocates in [false, true] {
    for shrinks in [false, true] {
    for min_chunk_size in (0..(1 << 6)).map(|i| 1 << i) {

        let config = BumpConfigBuilder::new()
            .min_align(min_align)
            .upwards(upwards)
            .const_constructible(const_constructible)
            .claimable(claimable)
            .deallocates(deallocates)
            .shrinks(shrinks)
            .min_chunk_size(min_chunk_size)
            .build();

        // std::println!("{config:0128b}");
        // std::println!("{:?}", BumpConfigBuilder(config));

        assert_eq!(min_align, read::min_align(config));
        assert_eq!(upwards, read::upwards(config));
        assert_eq!(const_constructible, read::const_constructible(config));
        assert_eq!(claimable, read::claimable(config));
        assert_eq!(deallocates, read::deallocates(config));
        assert_eq!(shrinks, read::shrinks(config));
        assert_eq!(min_chunk_size, read::min_chunk_size(config));

    }}}}}}}
}
