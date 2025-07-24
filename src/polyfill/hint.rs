/// Not part of std.
#[cold]
#[inline(always)]
#[allow(dead_code)]
pub(crate) fn cold() {}

/// Not part of std.
#[inline(always)]
#[allow(dead_code)]
pub(crate) fn likely(condition: bool) -> bool {
    if condition {
        // ...
    } else {
        cold();
    }

    condition
}

/// Not part of std.
#[inline(always)]
#[allow(dead_code)]
pub(crate) fn unlikely(condition: bool) -> bool {
    if condition {
        cold();
    } else {
        // ...
    }

    condition
}
