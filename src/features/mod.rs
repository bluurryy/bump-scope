#[cfg(feature = "serde")]
mod serde;

#[cfg(all(feature = "alloc", feature = "nightly-allocator-api"))]
mod alloc;

mod allocator_api2_03;

#[cfg(feature = "zerocopy")]
mod zerocopy;
