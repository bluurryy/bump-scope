/// See [`std::hint::cold_path`].
#[cold]
#[inline(always)]
pub(crate) fn cold() {}

/// See [`std::hint::likely`].
#[inline(always)]
pub(crate) fn likely(condition: bool) -> bool {
    if condition {
        // ...
    } else {
        cold();
    }

    condition
}

/// See [`std::hint::unlikely`].
#[inline(always)]
#[expect(dead_code)]
pub(crate) fn unlikely(condition: bool) -> bool {
    if condition {
        cold();
    } else {
        // ...
    }

    condition
}
