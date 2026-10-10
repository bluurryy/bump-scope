#[cfg(feature = "serde")]
mod serde;

#[cfg(any(feature = "bytemuck", feature = "zerocopy-08"))]
mod bytemuck_or_zerocopy;

#[cfg(any(feature = "bytemuck", feature = "zerocopy-08"))]
pub(crate) use bytemuck_or_zerocopy::bytemuck_or_zerocopy;
