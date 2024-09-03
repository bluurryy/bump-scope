use core::{
    alloc::{Layout, LayoutError},
    fmt,
    ops::Deref,
};

pub(crate) trait LayoutProps: Deref<Target = Layout> + Copy {
    const ALIGN_IS_CONST: bool;
    const SIZE_IS_CONST: bool;
    const SIZE_IS_MULTIPLE_OF_ALIGN: bool;
}

#[derive(Clone, Copy)]
pub(crate) struct SizedLayout(Layout);

impl LayoutProps for SizedLayout {
    const ALIGN_IS_CONST: bool = true;
    const SIZE_IS_CONST: bool = true;
    const SIZE_IS_MULTIPLE_OF_ALIGN: bool = true;
}

impl SizedLayout {
    #[inline(always)]
    pub(crate) const fn new<T>() -> Self {
        Self(Layout::new::<T>())
    }
}

impl Deref for SizedLayout {
    type Target = Layout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// This must be `pub` because we use it in `supported_minimum_alignment::Sealed` which is pub.
/// The current msrv denies us using `pub(crate)` with the error:
/// ```txt
/// error[E0446]: crate-private type `ArrayLayout` in public interface
/// ```
#[derive(Clone, Copy)]
pub struct ArrayLayout(Layout);

impl LayoutProps for ArrayLayout {
    const ALIGN_IS_CONST: bool = true;
    const SIZE_IS_CONST: bool = false;
    const SIZE_IS_MULTIPLE_OF_ALIGN: bool = true;
}

impl ArrayLayout {
    #[inline(always)]
    pub(crate) fn for_value<T>(value: &[T]) -> Self {
        Self(Layout::for_value(value))
    }

    #[inline(always)]
    pub(crate) fn array<T>(len: usize) -> Result<Self, LayoutError> {
        Ok(Self(Layout::array::<T>(len)?))
    }

    #[inline(always)]
    pub(crate) const fn from_layout(layout: Layout) -> Result<Self, ArrayLayoutError> {
        if layout.size() % layout.align() == 0 {
            Ok(ArrayLayout(layout))
        } else {
            Err(ArrayLayoutError)
        }
    }

    #[inline(always)]
    pub(crate) const fn from_size_align(size: usize, align: usize) -> Result<Self, ArrayLayoutError> {
        match Layout::from_size_align(size, align) {
            Ok(layout) => Self::from_layout(layout),
            Err(_) => Err(ArrayLayoutError),
        }
    }
}

impl Deref for ArrayLayout {
    type Target = Layout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CustomLayout(pub(crate) Layout);

impl LayoutProps for CustomLayout {
    const ALIGN_IS_CONST: bool = false;
    const SIZE_IS_CONST: bool = false;
    const SIZE_IS_MULTIPLE_OF_ALIGN: bool = false;
}

impl Deref for CustomLayout {
    type Target = Layout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ArrayLayoutError;

impl fmt::Display for ArrayLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid parameters to ArrayLayout constructor")
    }
}
