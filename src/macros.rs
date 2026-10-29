// TODO: contribute to allocator-api2?

/// A replacement for [`vec!`] that accepts an optional allocator parameter.
///
/// # Panics
/// If used without `try`, panics on allocation failure.
///
/// # Errors
/// If used with `try`, errors on allocation failure.
///
/// # Examples
///
/// - Create an empty [`Vec`]:
/// ```
/// use bump_scope::{Bump, vec};
///
/// let bump: Bump = Bump::new();
/// let vec: Vec<i32, _> = vec![in &bump];
/// assert!(vec.is_empty());
/// ```
///
/// - Create a [`Vec`] containing a given list of elements:
///
/// ```
/// use bump_scope::{Bump, vec};
///
/// let bump: Bump = Bump::new();
/// let vec = vec![in &bump; 1, 2, 3];
/// assert_eq!(vec[0], 1);
/// assert_eq!(vec[1], 2);
/// assert_eq!(vec[2], 3);
/// ```
///
/// - Create a [`Vec`] from a given element and size:
///
/// ```
/// use bump_scope::{Bump, vec};
///
/// let bump: Bump = Bump::new();
/// let vec = vec![in &bump; 1; 3];
/// assert_eq!(vec, [1, 1, 1]);
/// ```
///
/// [`vec!`]: alloc::vec!
/// [`Vec`]: alloc::vec::Vec
#[macro_export]
macro_rules! vec {
    [in $bump:expr] => {
        $crate::private::alloc::vec::Vec::new_in($bump)
    };
    [in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::extra::vec_from_owned_slice_in([$($values),*], $bump)
    };
    [in $bump:expr; $value:expr; $count:expr] => {
        $crate::private::alloc::vec::from_elem_in($value, $count, $bump)
    };
    [try in $bump:expr] => {
        Ok::<_, $crate::private::core::alloc::AllocError>($crate::private::alloc::vec::Vec::new_in($bump))
    };
    [try in $bump:expr; $($values:expr),* $(,)?] => {
        $crate::extra::vec_from_owned_slice_in([$($values),*], $bump)
    };
    [try in $bump:expr; $value:expr; $count:expr] => {
        $crate::BumpVec::try_from_elem_in($value, $count, $bump)
    };
    [$($tt:tt)*] => {
        $crate::private::alloc::vec![$($tt)*]
    };
}
