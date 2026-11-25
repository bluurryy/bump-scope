use core::hint::cold_path;

/// See [`std::hint::likely`].
#[inline(always)]
#[expect(unused)]
pub(crate) fn likely(condition: bool) -> bool {
    if condition {
        // ...
    } else {
        cold_path();
    }

    condition
}

/// See [`std::hint::unlikely`].
#[inline(always)]
#[expect(unused)]
pub(crate) fn unlikely(condition: bool) -> bool {
    if condition {
        cold_path();
    } else {
        // ...
    }

    condition
}
