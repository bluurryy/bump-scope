#[inline(always)]
pub(crate) const fn const_unwrap<T: Copy>(option: Option<T>) -> T {
    match option {
        Some(value) => value,
        None => unwrap_failed(),
    }
}

#[cold]
#[inline(never)]
#[track_caller]
const fn unwrap_failed() -> ! {
    panic!("called `Option::unwrap()` on a `None` value")
}
