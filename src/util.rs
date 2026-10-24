#[inline(always)]
pub(crate) unsafe fn ptr_result<T, E>(mut ptr: *mut Result<T, E>) -> Result<*mut T, *mut E> {
    unsafe {
        match &mut *ptr {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err),
        }
    }
}
