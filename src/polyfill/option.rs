#[inline(always)]
pub(crate) const fn const_unwrap<T: Copy>(option: Option<T>) -> T {
    match option {
        Some(value) => value,
        None => panic!("expected Some"),
    }
}
