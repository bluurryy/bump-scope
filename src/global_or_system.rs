use alloc::alloc::Global;

#[cfg(feature = "std")]
use std::alloc::System;

/// Temporary solution for [`Bump::const_new`] until we have `const Default`.
#[diagnostic::on_unimplemented(
    message = "`const_new` can only be called with a `Global` or `System` base allocator",
    label = "allocator type not supported",
    note = "You can still const-construct a `Bump` using `const_new_in`"
)]
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
