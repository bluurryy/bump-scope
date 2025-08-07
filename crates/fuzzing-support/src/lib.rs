#![feature(pointer_is_aligned_to, allocator_api, alloc_layout_extra)]
#![allow(clippy::cargo_common_metadata)]

use std::{alloc::Layout, cell::Cell, mem::swap, ops::Deref, ptr::NonNull, rc::Rc};

use arbitrary::{Arbitrary, Unstructured};
use bump_scope::alloc::{AllocError, Allocator};

pub use arbitrary;
pub use bump_scope;
pub mod alloc_static_layout;
pub mod allocator_api;
pub mod bump_down;
pub mod bump_prepare_down;
pub mod bump_prepare_up;
pub mod bump_up;
pub mod bump_vec;
pub mod chunk_size;
mod from_bump_scope;

#[derive(Debug, Clone)]
struct RcAllocator<A> {
    inner: Rc<A>,
}

impl<A> RcAllocator<A> {
    pub fn new(inner: Rc<A>) -> Self {
        Self { inner }
    }
}

impl<A> Deref for RcAllocator<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

unsafe impl<A> Allocator for RcAllocator<A>
where
    A: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.inner.deallocate(ptr, layout) };
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate_zeroed(layout)
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.inner.grow(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.inner.grow_zeroed(ptr, old_layout, new_layout) }
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.inner.shrink(ptr, old_layout, new_layout) }
    }
}

#[derive(Debug, Clone)]
struct MaybeFailingAllocator<A> {
    pub inner: A,
    pub fails: Cell<bool>,
}

impl<A> MaybeFailingAllocator<A> {
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            fails: Cell::new(false),
        }
    }
}

unsafe impl<A> Allocator for MaybeFailingAllocator<A>
where
    A: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        self.inner.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.inner.deallocate(ptr, layout) };
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        self.inner.allocate_zeroed(layout)
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        unsafe { self.inner.grow(ptr, old_layout, new_layout) }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        unsafe { self.inner.grow_zeroed(ptr, old_layout, new_layout) }
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        unsafe { self.inner.shrink(ptr, old_layout, new_layout) }
    }
}

// modified from std
macro_rules! debug_dbg {
    // NOTE: We cannot use `concat!` to make a static string as a format argument
    // of `eprintln!` because `file!` could contain a `{` or
    // `$val` expression could be a block (`{ .. }`), in which case the `eprintln!`
    // will be malformed.
    () => {
        ::log::debug!("[{}:{}:{}]", file!(), line!(), column!())
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                ::log::debug!("[{}:{}:{}] {} = {:#?}",
                    file!(), line!(), column!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}

pub(crate) use debug_dbg;

#[derive(Debug, Clone, Copy)]
struct FuzzBumpPropsRange {
    start: usize,
    end: usize,
}

impl<'a> Arbitrary<'a> for FuzzBumpPropsRange {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let [mut start, mut end] = u.arbitrary()?;

        if end < start {
            swap(&mut start, &mut end);
        }

        start = align(start);
        end = align(end);

        Ok(Self { start, end })
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <[usize; 2]>::size_hint(depth)
    }
}

#[derive(Debug, Clone, Copy)]
struct FuzzBumpPropsLayout(Layout);

impl<'a> Arbitrary<'a> for FuzzBumpPropsLayout {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let size: usize = u.arbitrary()?;
        let align_pow2: usize = u.int_in_range::<u8>(0..=10)?.into();

        let align = 1 << align_pow2;

        Layout::from_size_align(size, align)
            .map(Self)
            .map_err(|_| arbitrary::Error::IncorrectFormat)
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <(usize, u8)>::size_hint(depth)
    }
}

#[derive(Arbitrary, Debug, Clone, Copy)]
pub(crate) struct FuzzBumpProps {
    pub(crate) range: FuzzBumpPropsRange,
    pub(crate) layout: FuzzBumpPropsLayout,
    pub(crate) min_align: MinAlign,
    pub(crate) align_is_const: bool,
    pub(crate) size_is_const: bool,
}

impl FuzzBumpProps {
    fn for_up(mut self) -> Self {
        self.range.start = down_align(self.range.start, self.min_align as usize);
        self
    }

    fn for_down(mut self) -> Self {
        self.range.end = down_align(self.range.end, self.min_align as usize);
        self
    }

    fn for_prepare(self) -> Self {
        self
    }
}

impl FuzzBumpProps {
    fn to(self) -> from_bump_scope::bumping::BumpProps {
        let Self {
            range: FuzzBumpPropsRange { start, end },
            layout: FuzzBumpPropsLayout(layout),
            min_align,
            align_is_const,
            size_is_const,
        } = self;

        from_bump_scope::bumping::BumpProps {
            start,
            end,
            layout,
            min_align: min_align as usize,
            align_is_const,
            size_is_const,
            size_is_multiple_of_align: layout.size().is_multiple_of(layout.align()),
        }
    }
}

#[inline(always)]
fn down_align(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}

#[inline(always)]
fn align(addr: usize) -> usize {
    let addr = down_align(addr, 16);

    if addr == 0 { 16 } else { addr }
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum MinAlign {
    Shl0 = 1 << 0,
    Shl1 = 1 << 1,
    Shl2 = 1 << 2,
    Shl3 = 1 << 3,
    Shl4 = 1 << 4,
}

#[derive(Debug, Clone, Copy)]
struct UpTo<const MAX: usize>(usize);

impl<'a, const MAX: usize> Arbitrary<'a> for UpTo<MAX> {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(u.int_in_range(0..=MAX)?))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <usize as Arbitrary>::size_hint(depth)
    }
}
