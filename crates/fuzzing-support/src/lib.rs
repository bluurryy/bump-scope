#![feature(pointer_is_aligned_to, strict_provenance, allocator_api)]
#![allow(clippy::cargo_common_metadata)]

use std::{alloc::Layout, cell::Cell, mem::swap, ops::Deref, ptr::NonNull, rc::Rc};

use arbitrary::Arbitrary;
use bump_scope::allocator_api2::alloc::{AllocError, Allocator};

pub use arbitrary;

pub mod allocator_api;
pub mod bump_down;
pub mod bump_greedy_down;
pub mod bump_greedy_up;
pub mod bump_up;
pub mod bumping;
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
        self.inner.deallocate(ptr, layout)
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate_zeroed(layout)
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.grow(ptr, old_layout, new_layout)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.grow_zeroed(ptr, old_layout, new_layout)
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.shrink(ptr, old_layout, new_layout)
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
        self.inner.deallocate(ptr, layout)
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

        self.inner.grow(ptr, old_layout, new_layout)
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

        self.inner.grow_zeroed(ptr, old_layout, new_layout)
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.fails.get() {
            return Err(AllocError);
        }

        self.inner.shrink(ptr, old_layout, new_layout)
    }
}

#[cfg(fuzzing_repro)]
pub use std::{dbg, eprint, eprintln};

#[cfg(not(fuzzing_repro))]
#[macro_export]
macro_rules! dbg {
    ($($tt:tt)*) => {};
}

#[cfg(not(fuzzing_repro))]
#[macro_export]
macro_rules! eprint {
    ($($tt:tt)*) => {};
}

#[cfg(not(fuzzing_repro))]
#[macro_export]
macro_rules! eprintln {
    ($($tt:tt)*) => {};
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FuzzBumpProps {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) layout: Layout,
    pub(crate) min_align: usize,
    pub(crate) align_is_const: bool,
    pub(crate) size_is_const: bool,
}

impl<'a> Arbitrary<'a> for FuzzBumpProps {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let (mut start, mut end) = u.arbitrary()?;

        if end < start {
            swap(&mut start, &mut end);
        }

        start = align(start);
        end = align(end);

        let layout = {
            let size = u.arbitrary()?;
            let align_pow2 = u.int_in_range(0..=10)?;
            let align = 1 << align_pow2;
            Layout::from_size_align(size, align).map_err(|_| arbitrary::Error::IncorrectFormat)?
        };

        let min_align = *u.choose(&[1, 2, 4, 8, 16])?;

        Ok(Self {
            start,
            end,
            layout,
            min_align,
            align_is_const: u.arbitrary()?,
            size_is_const: u.arbitrary()?,
        })
    }
}

impl FuzzBumpProps {
    fn for_up(mut self) -> Self {
        self.start = down_align(self.start, self.min_align);
        self
    }

    fn for_down(mut self) -> Self {
        self.end = down_align(self.end, self.min_align);
        self
    }
}

impl FuzzBumpProps {
    fn to(self) -> from_bump_scope::bumping::BumpProps {
        let Self {
            start,
            end,
            layout,
            min_align,
            align_is_const,
            size_is_const,
        } = self;

        from_bump_scope::bumping::BumpProps {
            start,
            end,
            layout,
            min_align,
            align_is_const,
            size_is_const,
            size_is_multiple_of_align: layout.size() % layout.align() == 0,
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

    if addr == 0 {
        16
    } else {
        addr
    }
}
