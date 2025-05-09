#[cfg(all(feature = "nightly-allocator-api", feature = "alloc"))]
mod inner {
    pub use ::alloc::{alloc, boxed, collections, vec};
}

#[cfg(all(feature = "nightly-allocator-api", not(feature = "alloc")))]
mod inner {
    pub use ::core::alloc;
}

#[cfg(not(feature = "nightly-allocator-api"))]
mod inner {
    pub use ::allocator_api2::alloc;

    #[cfg(feature = "alloc")]
    pub use ::allocator_api2::{boxed, collections, vec};
}

pub use inner::*;
