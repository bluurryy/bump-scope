/// This trait marks types that don't need dropping.
///
/// This trait is a best effort for modeling such a constraint.
/// It is not implemented for all types that don't need dropping.
///
/// Specifically `&mut T` types don't implement `NoDrop` but definitely would if it were possible.
///
/// Every `T where T: Copy` and every `[T] where T: NoDrop` automatically implements `NoDrop`.
///
/// It is used as a bound for [`BumpBox`]'s [`into_ref`] and [`into_mut`] so you don't accidentally omit a drop that does matter.
///
/// [`BumpBox`]: crate::BumpBox
/// [`into_ref`]: crate::BumpBox::into_ref
/// [`into_mut`]: crate::BumpBox::into_mut
pub trait NoDrop {}

impl NoDrop for str {}
impl<T: Copy> NoDrop for T {}
impl<T: NoDrop> NoDrop for [T] {}

impl NoDrop for core::ffi::CStr {}

#[cfg(feature = "std")]
mod std_impl {
    use super::NoDrop;

    impl NoDrop for std::ffi::OsStr {}
    impl NoDrop for std::path::Path {}
}
