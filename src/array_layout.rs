//! Adding additional invariants to [`Layout`].

use core::{
    alloc::{Layout, LayoutError},
    fmt,
};

/// # Safety
///
/// `IS_ARRAY_LAYOUT` must only be `true` iff `size % align == 0`.
pub unsafe trait LayoutTrait: Copy {
    const IS_ARRAY_LAYOUT: bool = false;

    fn layout(&self) -> Layout;

    fn size(&self) -> usize {
        self.layout().size()
    }

    fn align(&self) -> usize {
        self.layout().align()
    }
}

unsafe impl LayoutTrait for Layout {
    #[inline(always)]
    fn layout(&self) -> Layout {
        *self
    }
}

/// This is a wrapper around [`Layout`] that only allows layouts where `size % align == 0`
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ArrayLayout(Layout);

impl ArrayLayout {
    #[inline(always)]
    pub const fn new<T>() -> Self {
        Self(Layout::new::<T>())
    }

    #[inline(always)]
    pub fn for_value<T>(value: &[T]) -> Self {
        Self(Layout::for_value(value))
    }

    #[inline(always)]
    pub fn array<T>(len: usize) -> Result<Self, LayoutError> {
        Ok(Self(Layout::array::<T>(len)?))
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.0.size()
    }

    #[inline(always)]
    pub fn align(&self) -> usize {
        self.0.align()
    }

    #[inline(always)]
    pub const fn into_inner(self) -> Layout {
        self.0
    }

    #[inline(always)]
    pub const fn from_layout(layout: Layout) -> Result<Self, ArrayLayoutError> {
        if layout.size() % layout.align() == 0 {
            Ok(ArrayLayout(layout))
        } else {
            Err(ArrayLayoutError)
        }
    }

    #[inline(always)]
    pub const fn from_size_align(size: usize, align: usize) -> Result<Self, ArrayLayoutError> {
        match Layout::from_size_align(size, align) {
            Ok(layout) => Self::from_layout(layout),
            Err(_) => Err(ArrayLayoutError),
        }
    }
}

impl From<ArrayLayout> for Layout {
    fn from(value: ArrayLayout) -> Self {
        value.0
    }
}

unsafe impl LayoutTrait for ArrayLayout {
    const IS_ARRAY_LAYOUT: bool = true;

    #[inline(always)]
    fn layout(&self) -> Layout {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ArrayLayoutError;

impl fmt::Display for ArrayLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid parameters to ArrayLayout constructor")
    }
}
