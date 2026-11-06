/// See [`core::panic::abort_on_unwind`](https://doc.rust-lang.org/nightly/core/panic/fn.abort_on_unwind.html).
#[inline]
#[cfg(not(feature = "std"))]
pub fn abort_on_unwind<R>(f: impl FnOnce() -> R) -> R {
    #[inline]
    extern "C" fn inner<R>(f: impl FnOnce() -> R) -> R {
        f()
    }

    inner(f)
}
