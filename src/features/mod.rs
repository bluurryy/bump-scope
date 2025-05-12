#[cfg(feature = "serde")]
mod serde;

#[cfg(all(feature = "alloc", feature = "nightly-allocator-api"))]
pub mod alloc;

#[cfg(feature = "allocator-api2-02")]
pub mod allocator_api2_02;

#[cfg(feature = "allocator-api2-03")]
pub mod allocator_api2_03;

#[cfg(feature = "zerocopy")]
mod zerocopy;
