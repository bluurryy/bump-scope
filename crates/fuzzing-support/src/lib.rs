#![feature(pointer_is_aligned_to, allocator_api, alloc_layout_extra)]
#![expect(clippy::cargo_common_metadata)]

use std::{alloc::Layout, cell::Cell, ops::Deref, ptr::NonNull, rc::Rc};

use arbitrary::{Arbitrary, Unstructured};
use bump_scope::{
    alloc::{AllocError, Allocator, Global},
    settings::BumpSettings,
};

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

pub type Bump<A = Global, const MIN_ALIGN: usize = 1, const UP: bool = true> =
    bump_scope::Bump<A, BumpSettings<MIN_ALIGN, UP>>;

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

use crate::from_bump_scope::bumping::{BumpProps, MIN_CHUNK_ALIGN};

#[derive(Debug, Clone, Copy)]
struct FuzzBumpPropsRange {
    start: usize,
    end: usize,
    is_dummy_chunk: bool,
}

impl<'a> Arbitrary<'a> for FuzzBumpPropsRange {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let is_dummy_chunk = u.arbitrary::<bool>()?;

        let start: usize;
        let end: usize;

        if is_dummy_chunk {
            end = down_align(u.int_in_range(MIN_CHUNK_ALIGN..=(usize::MAX - 16))?, MIN_CHUNK_ALIGN);
            start = end + 16;
        } else {
            start = u.int_in_range(MIN_CHUNK_ALIGN..=usize::MAX)?;

            // an allocation can't be greater than `isize::MAX`
            let max = start.saturating_add(isize::MAX as usize);

            end = u.int_in_range(start..=max)?;

            assert!(is_valid_size(start, end));
        }

        Ok(Self {
            start,
            end,
            is_dummy_chunk,
        })
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        arbitrary::size_hint::or(<(bool, usize)>::size_hint(depth), <(bool, [usize; 2])>::size_hint(depth))
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

#[derive(Debug)]
struct FuzzBumpPropsUp(BumpProps);

impl<'a> Arbitrary<'a> for FuzzBumpPropsUp {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(u.arbitrary::<FuzzBumpProps>()?.for_up().to()?))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        FuzzBumpProps::size_hint(depth)
    }
}

#[derive(Debug)]
struct FuzzBumpPropsDown(BumpProps);

impl<'a> Arbitrary<'a> for FuzzBumpPropsDown {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(u.arbitrary::<FuzzBumpProps>()?.for_down().to()?))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        FuzzBumpProps::size_hint(depth)
    }
}

#[derive(Debug)]
struct FuzzBumpPropsPrepareUp(BumpProps);

impl<'a> Arbitrary<'a> for FuzzBumpPropsPrepareUp {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(u.arbitrary::<FuzzBumpProps>()?.for_prepare().for_up().to()?))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        FuzzBumpProps::size_hint(depth)
    }
}

#[derive(Debug)]
struct FuzzBumpPropsPrepareDown(BumpProps);

impl<'a> Arbitrary<'a> for FuzzBumpPropsPrepareDown {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(u.arbitrary::<FuzzBumpProps>()?.for_prepare().for_down().to()?))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        FuzzBumpProps::size_hint(depth)
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
        if !self.range.is_dummy_chunk {
            self.range.start = down_align(self.range.start, self.min_align as usize);
            self.range.end = down_align(self.range.end, MIN_CHUNK_ALIGN);
        }

        self
    }

    fn for_down(mut self) -> Self {
        if !self.range.is_dummy_chunk {
            self.range.start = down_align(self.range.start, MIN_CHUNK_ALIGN);
            self.range.end = down_align(self.range.end, self.min_align as usize);
        }

        self
    }

    fn for_prepare(self) -> Self {
        self
    }
}

impl FuzzBumpProps {
    fn to(self) -> arbitrary::Result<from_bump_scope::bumping::BumpProps> {
        let Self {
            range:
                FuzzBumpPropsRange {
                    start,
                    end,
                    is_dummy_chunk,
                },
            layout: FuzzBumpPropsLayout(layout),
            min_align,
            align_is_const,
            size_is_const,
        } = self;

        if is_dummy_chunk {
            assert_eq!(start, end + 16);
        } else {
            // the aligning in `for_up` and `for_down` may have caused the
            // start and end to no longer represent a valid size
            if !is_valid_size(start, end) {
                return Err(arbitrary::Error::IncorrectFormat);
            }
        }

        Ok(from_bump_scope::bumping::BumpProps {
            start,
            end,
            layout,
            min_align: min_align as usize,
            align_is_const,
            size_is_const,
            size_is_multiple_of_align: layout.size().is_multiple_of(layout.align()),
        })
    }
}

fn is_valid_size(start: usize, end: usize) -> bool {
    const MAX: i128 = isize::MAX as i128;
    let size = end as i128 - start as i128;
    (0..=MAX).contains(&size)
}

#[inline(always)]
fn down_align(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
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
