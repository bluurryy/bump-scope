use alloc::alloc::Global;

#[cfg(feature = "std")]
use std::alloc::System;

/// Temporary solution for [`Bump::const_new`] until we have `const Default`.
///
/// We could make this a public `ConstDefault` trait.
/// Currently it's just named `GlobalOrSystem` to give context to this private bound.
pub(crate) trait GlobalOrSystem {
    const CONST_DEFAULT: Self;
}

impl GlobalOrSystem for Global {
    const CONST_DEFAULT: Self = Self;
}

#[cfg(feature = "std")]
impl GlobalOrSystem for System {
    const CONST_DEFAULT: Self = Self;
}
