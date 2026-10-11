#[inline]
#[cfg(not(feature = "std"))]
pub fn abort_on_unwind<R>(f: impl FnOnce() -> R) -> R {
    #[inline]
    extern "C" fn inner<R>(f: impl FnOnce() -> R) -> R {
        f()
    }

    inner(f)
}
