#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "nightly-allocator-api")]
pub mod nightly_allocator_api;

#[cfg(feature = "allocator-api2-02")]
pub mod allocator_api2_02;

#[cfg(feature = "allocator-api2-03")]
pub mod allocator_api2_03;

#[cfg(feature = "zerocopy-08")]
pub mod zerocopy_08;
