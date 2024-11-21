/// This trait marks types that don't need dropping.
///
/// This trait is a best effort for modeling such a constraint. It is not implemented for all types that don't need dropping.
///
/// Every `T where T: Copy` and every `[T] where T: NoDrop` automatically implements `NoDrop`.
///
/// It is used as a bound for [`BumpBox`]'s [`into_ref`](BumpBox::into_ref) and [`into_mut`](BumpBox::into_mut) so you don't accidentally omit a drop that does matter.
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
