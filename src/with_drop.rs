//! When wrapped in [`WithDrop`], the `alloc*` methods return `&mut T` instead. In this case `T` will be dropped once `WithDrop` drops.
//!
//! This is a stub.
//!
//! It does work, but requires all allocated values to be `'static`. That's because
//! otherwise you could potentially access references to already dropped values:
//!
//! ```no_run
//! #[derive(Debug)]
//! struct Foo<'a>(Option<&'a String>);
//!
//! impl Drop for Foo<'_> {
//!     fn drop(&mut self) {
//!         dbg!(self.0);
//!     }
//! }
//!
//! let bump: Bump = Bump::new().with_drop();
//!
//! let foo = bump.alloc(Foo(None));
//! let value = bump.alloc(String::from("Oh no!"));
//!
//! foo.0 = Some(value);
//!
//! drop(bump);
//! ```
//!
//! Is this still useful even with the `'static` bound?

#![allow(dead_code)]

use core::{
    alloc::Layout,
    cell::Cell,
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
    slice,
};

#[cfg(feature = "alloc")]
use core::fmt;
use std::ops::Deref;

use allocator_api2::alloc::Allocator;

use crate::{
    allocation_behavior::LayoutProps,
    polyfill::{layout, nonnull},
    AllocError, AnyBump, BumpAllocator, BumpBox, ErrorBehavior, SizedTypeProperties,
};

/// Wraps a bump allocator, makes all of the `alloc*` functions return `&mut T` and drops those `T` when it drops itself.
///
/// This type is returned from [`Bump(Scope)::with_drop`](crate::Bump::with_drop)([`_ref`](crate::Bump::with_drop_ref)/[`_mut`](crate::Bump::with_drop_mut)).
pub struct WithDrop<Bump> {
    inner: ManuallyDrop<Bump>,
    drop_list: DropList,
}

impl<Bump> Drop for WithDrop<Bump> {
    fn drop(&mut self) {
        unsafe {
            self.drop_list.drop();
            ManuallyDrop::drop(&mut self.inner);
        }
    }
}

impl<Bump> WithDrop<Bump> {
    /// Create a new `WithDrop` by wrapping `bump`.
    ///
    /// `bump` may be a `Bump`, `BumpScope` as well as shared or mutable references thereof.
    pub fn new(bump: Bump) -> Self {
        Self {
            inner: ManuallyDrop::new(bump),
            drop_list: DropList::new(),
        }
    }

    /// Returns a reference to the inner bump allocator.
    pub fn as_inner(&self) -> &Bump {
        &self.inner
    }

    /// Converts this `WithDrop` into its inner bump allocator.
    ///
    /// This drops all values allocated with this `WithDrop`.
    pub fn into_inner(self) -> Bump {
        let mut this = ManuallyDrop::new(self);
        unsafe {
            this.drop_list.drop();
            ManuallyDrop::take(&mut this.inner)
        }
    }
}

impl<Bump: AnyBump> WithDrop<Bump> {
    #[inline(always)]
    pub(crate) fn generic_alloc<B: ErrorBehavior, T: 'static>(&self, value: T) -> Result<&mut T, B> {
        self.generic_alloc_with(|| value)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_with<B: ErrorBehavior, T: 'static>(&self, f: impl FnOnce() -> T) -> Result<&mut T, B> {
        if !T::NEEDS_DROP {
            return if T::IS_ZST {
                Ok(zst())
            } else {
                let boxed = self.inner.alloc_with(f)?;
                Ok(BumpBox::leak(boxed))
            };
        }

        let Allocation { header, uninit } = self.boxed_alloc::<B, T>()?;
        let init = uninit.init(f());
        Ok(self.drop_list.append(header, init))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_default<B: ErrorBehavior, T: 'static + Default>(&self) -> Result<&mut T, B> {
        self.generic_alloc_with(Default::default)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_copy<B: ErrorBehavior, T: 'static + Copy>(&self, slice: &[T]) -> Result<&mut [T], B> {
        if !T::NEEDS_DROP {
            return if T::IS_ZST {
                Ok(zst_slice(slice.len()))
            } else {
                let boxed = self.inner.alloc_slice_copy(slice)?;
                Ok(BumpBox::leak(boxed))
            };
        }

        let Allocation { header, uninit } = self.boxed_alloc_slice_for(slice)?;
        let init = uninit.init_copy(slice);
        Ok(self.drop_list.append(header, init))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_clone<B: ErrorBehavior, T: 'static + Clone>(
        &self,
        slice: &[T],
    ) -> Result<&mut [T], B> {
        if !T::NEEDS_DROP {
            return if T::IS_ZST {
                Ok(zst_slice(slice.len()))
            } else {
                let boxed = self.inner.alloc_slice_clone(slice)?;
                Ok(BumpBox::leak(boxed))
            };
        }

        let Allocation { header, uninit } = self.boxed_alloc_slice_for(slice)?;
        let init = uninit.init_clone(slice);
        Ok(self.drop_list.append(header, init))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_fill<B: ErrorBehavior, T: 'static + Clone>(
        &self,
        len: usize,
        value: T,
    ) -> Result<&mut [T], B> {
        if !T::NEEDS_DROP {
            return if T::IS_ZST {
                Ok(zst_slice(len))
            } else {
                let boxed = self.inner.alloc_slice_fill(len, value)?;
                Ok(BumpBox::leak(boxed))
            };
        }

        let Allocation { header, uninit } = self.boxed_alloc_slice(len)?;
        let init = uninit.init_fill(value);
        Ok(self.drop_list.append(header, init))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_fill_with<B: ErrorBehavior, T: 'static>(
        &self,
        len: usize,
        f: impl FnMut() -> T,
    ) -> Result<&mut [T], B> {
        if !T::NEEDS_DROP {
            return if T::IS_ZST {
                Ok(zst_slice(len))
            } else {
                let boxed = self.inner.alloc_slice_fill_with(len, f)?;
                Ok(BumpBox::leak(boxed))
            };
        }

        let Allocation { header, uninit } = self.boxed_alloc_slice(len)?;
        let init = uninit.init_fill_with(f);
        Ok(self.drop_list.append(header, init))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_str<B: ErrorBehavior>(&self, src: &str) -> Result<&mut str, B> {
        let boxed = self.inner.alloc_str(src)?;
        Ok(unsafe { boxed.into_raw().as_mut() })
    }

    #[inline(always)]
    #[cfg(feature = "alloc")]
    pub(crate) fn generic_alloc_fmt<B: ErrorBehavior>(&self, args: fmt::Arguments) -> Result<&mut str, B> {
        let boxed = self.inner.alloc_fmt(args)?;
        Ok(BumpBox::leak(boxed))
    }

    #[inline(always)]
    pub(crate) fn generic_reserve_bytes<B: ErrorBehavior>(&self, additional: usize) -> Result<(), B> {
        self.inner.reserve_bytes(additional)
    }

    #[inline(always)]
    fn boxed_alloc<B: ErrorBehavior, T: 'static>(&self) -> Result<Allocation<MaybeUninit<T>>, B> {
        assert!(T::NEEDS_DROP);

        let uninit = self.inner.alloc_uninit::<B, WithHeader<T>>()?;

        let header_ptr = uninit.into_raw().cast::<Header>();
        let header = self.drop_list.header_for_sized::<T>();

        unsafe {
            header_ptr.as_ptr().write(header);

            let value_ptr = nonnull::byte_add(header_ptr, T::OFFSET_FROM_HEADER).cast::<MaybeUninit<T>>();

            Ok(Allocation {
                header: header_ptr,
                uninit: BumpBox::from_raw(value_ptr),
            })
        }
    }

    #[inline(always)]
    fn boxed_alloc_slice<B: ErrorBehavior, T: 'static>(&self, len: usize) -> Result<Allocation<[MaybeUninit<T>]>, B> {
        assert!(T::NEEDS_DROP);

        let slice_layout = match Layout::array::<T>(len) {
            Ok(layout) => layout,
            Err(_) => return Err(B::capacity_overflow()),
        };

        let full_layout = match Layout::new::<SliceHeader>().extend(slice_layout) {
            Ok(extended) => extended.0,
            Err(_) => return Err(B::capacity_overflow()),
        };

        unsafe { self.boxed_alloc_slice_inner(len, full_layout) }
    }

    #[inline(always)]
    fn boxed_alloc_slice_for<B: ErrorBehavior, T: 'static>(&self, slice: &[T]) -> Result<Allocation<[MaybeUninit<T>]>, B> {
        assert!(T::NEEDS_DROP);

        let slice_layout = Layout::for_value(slice);

        let full_layout = match Layout::new::<SliceHeader>().extend(slice_layout) {
            Ok(extended) => extended.0,
            Err(_) => return Err(B::capacity_overflow()),
        };

        unsafe { self.boxed_alloc_slice_inner(slice.len(), full_layout) }
    }

    /// This function exists because you may get a `layout` from
    /// `Layout::array::<T>(len)` or `Layout::for_value(slice)`.
    ///
    /// # Safety
    ///
    /// `layout` must be equal to `SliceWithHeader::<T>::layout(len)`.
    #[inline(always)]
    unsafe fn boxed_alloc_slice_inner<B: ErrorBehavior, T: 'static>(
        &self,
        len: usize,
        layout: Layout,
    ) -> Result<Allocation<[MaybeUninit<T>]>, B> {
        let ptr = match self.inner.alloc_in_current_chunk(CustomLayoutConstAlign(layout)) {
            Some(ptr) => ptr,
            None => self.inner.alloc_in_another_chunk::<B>(layout)?,
        };

        let header_ptr = ptr.cast::<SliceHeader>();
        let header = self.drop_list.header_for_slice::<T>(len);

        unsafe {
            header_ptr.as_ptr().write(header);

            let value_ptr = nonnull::byte_add(header_ptr, <[T]>::OFFSET_FROM_HEADER).cast::<MaybeUninit<T>>();
            let slice_ptr = nonnull::slice_from_raw_parts(value_ptr, len);

            Ok(Allocation {
                header: header_ptr.cast(),
                uninit: BumpBox::from_raw(slice_ptr),
            })
        }
    }
}

fn zst<'a, T>() -> &'a mut T {
    assert!(T::IS_ZST);

    unsafe { NonNull::dangling().as_mut() }
}

fn zst_slice<'a, T>(len: usize) -> &'a mut [T] {
    assert!(T::IS_ZST);

    unsafe { slice::from_raw_parts_mut(NonNull::dangling().as_ptr(), len) }
}

struct Allocation<'a, T: ?Sized + 'static> {
    header: NonNull<Header>,
    uninit: BumpBox<'a, T>,
}

struct DropList {
    last: Cell<Option<NonNull<Header>>>,
}

impl DropList {
    const fn new() -> Self {
        Self { last: Cell::new(None) }
    }

    fn header_for_sized<T>(&self) -> Header {
        Header {
            drop: drop_sized::<T>,
            prev: self.last.get(),
        }
    }

    fn header_for_slice<T>(&self, len: usize) -> SliceHeader {
        SliceHeader {
            header: Header {
                drop: drop_slice::<T>,
                prev: self.last.get(),
            },
            len,
        }
    }

    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    fn append<'a, T: ?Sized + 'static>(&self, header: NonNull<Header>, init: BumpBox<'a, T>) -> &'a mut T {
        self.last.set(Some(header));
        BumpBox::leak(init)
    }

    /// Drops all values in the list.
    ///
    /// # Safety
    ///
    /// All `BumpBox`es that have been appended to the list must still be live.
    unsafe fn drop(&self) {
        let mut iter = self.last.get();

        while let Some(header) = iter {
            let Header { drop, prev } = *header.as_ref();
            drop(header);
            iter = prev;
        }
    }
}

unsafe fn drop_sized<T>(header_ptr: NonNull<Header>) {
    let value_ptr = nonnull::byte_add(header_ptr, T::OFFSET_FROM_HEADER).cast::<T>();
    value_ptr.as_ptr().drop_in_place();
}

unsafe fn drop_slice<T>(header_ptr: NonNull<Header>) {
    let len = nonnull::byte_add(header_ptr, usize::OFFSET_FROM_HEADER)
        .cast::<usize>()
        .as_ptr()
        .read();
    let value_ptr = nonnull::byte_add(header_ptr, <[T]>::OFFSET_FROM_HEADER).cast::<T>();
    let slice_ptr = nonnull::slice_from_raw_parts(value_ptr, len);
    slice_ptr.as_ptr().drop_in_place();
}

trait OffsetFromHeader {
    const EXTEND_WITH_HEADER: (Layout, usize);
    const LAYOUT_WITH_HEADER: Layout = Self::EXTEND_WITH_HEADER.0;
    const OFFSET_FROM_HEADER: usize = Self::EXTEND_WITH_HEADER.1;
}

impl<T> OffsetFromHeader for T {
    const EXTEND_WITH_HEADER: (Layout, usize) = match layout::extend(Layout::new::<Header>(), Layout::new::<T>()) {
        Ok(offset) => offset,
        Err(_) => panic!("can't allocate this type in WithDrop"),
    };
}

impl<T> OffsetFromHeader for [T] {
    const EXTEND_WITH_HEADER: (Layout, usize) = match layout::extend(Layout::new::<SliceHeader>(), Layout::new::<T>()) {
        Ok(offset) => offset,
        Err(_) => panic!("can't allocate a slice of this type in WithDrop"),
    };
}

const fn layout_eq(lhs: Layout, rhs: Layout) -> bool {
    lhs.align() == rhs.align() && lhs.size() == rhs.size()
}

const _: () = assert!(layout_eq(usize::LAYOUT_WITH_HEADER, Layout::new::<SliceHeader>()));

#[repr(C)]
pub(crate) struct Header {
    drop: unsafe fn(NonNull<Header>),
    prev: Option<NonNull<Header>>,
}

#[repr(C)]
pub(crate) struct SliceHeader {
    header: Header,
    len: usize,
}

#[repr(C)]
pub(crate) struct WithHeader<T> {
    header: Header,
    value: MaybeUninit<T>,
}

unsafe impl<Bump: Allocator> Allocator for WithDrop<Bump> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.inner.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.inner.deallocate(ptr, layout);
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

unsafe impl<Bump: BumpAllocator> BumpAllocator for WithDrop<Bump> {}

#[derive(Clone, Copy)]
pub(crate) struct CustomLayoutConstAlign(pub Layout);

impl LayoutProps for CustomLayoutConstAlign {
    const ALIGN_IS_CONST: bool = true;
    const SIZE_IS_CONST: bool = false;
    const SIZE_IS_MULTIPLE_OF_ALIGN: bool = false;
}

impl CustomLayoutConstAlign {
    #[inline(always)]
    pub(crate) const fn new<T>() -> Self {
        Self(Layout::new::<T>())
    }
}

impl Deref for CustomLayoutConstAlign {
    type Target = Layout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
