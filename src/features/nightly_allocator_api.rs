use core::{alloc::Layout, ptr, ptr::NonNull};

use core::alloc::{AllocError, Allocator};

#[cfg(feature = "alloc")]
use alloc_crate::{alloc::Global, boxed::Box};

use crate::{
    BaseAllocator, Bump, BumpAllocatorExt, BumpScope, WithoutDealloc, WithoutShrink,
    alloc::{AllocError as CrateAllocError, Allocator as CrateAllocator, BoxLike, box_like},
    settings::BumpAllocatorSettings,
};

#[cfg(feature = "alloc")]
unsafe impl CrateAllocator for Global {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, CrateAllocError> {
        <Self as Allocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { <Self as Allocator>::deallocate(self, ptr, layout) };
    }

    #[inline(always)]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, CrateAllocError> {
        <Self as Allocator>::allocate_zeroed(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, CrateAllocError> {
        unsafe { <Self as Allocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, CrateAllocError> {
        unsafe { <Self as Allocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, CrateAllocError> {
        unsafe { <Self as Allocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    fn by_ref(&self) -> &Self
    where
        Self: Sized,
    {
        self
    }
}

/// Wraps an <code>alloc::alloc::[Allocator](Allocator)</code> to implement
/// <code>bump_scope::alloc::[Allocator](CrateAllocator)</code> and vice versa.
///
/// # Example
///
/// ```
/// # extern crate alloc;
/// # use core::{alloc::Layout, ptr::NonNull};
/// # use alloc::alloc::{AllocError, Global};
/// use alloc::alloc::Allocator;
///
/// use bump_scope::{Bump, alloc::compat::AllocatorNightlyCompat};
///
/// #[derive(Clone)]
/// struct MyNightlyAllocator;
///
/// unsafe impl Allocator for MyNightlyAllocator {
/// # /*
///     ...
/// # */
/// #   fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
/// #       <Global as Allocator>::allocate(&Global, layout)
/// #   }
/// #       
/// #   unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
/// #       <Global as Allocator>::deallocate(&Global, ptr, layout)
/// #   }
/// }
///
/// let bump: Bump<_> = Bump::new_in(AllocatorNightlyCompat(MyNightlyAllocator));
/// # _ = bump;
/// ```
#[repr(transparent)]
#[derive(Debug, Default, Clone)]
pub struct AllocatorNightlyCompat<A: ?Sized>(pub A);

impl<A: ?Sized> AllocatorNightlyCompat<A> {
    #[inline(always)]
    #[expect(missing_docs)]
    pub fn from_ref(allocator: &A) -> &Self {
        unsafe { &*(ptr::from_ref(allocator) as *const Self) }
    }

    #[inline(always)]
    #[expect(missing_docs)]
    pub fn from_mut(allocator: &mut A) -> &mut Self {
        unsafe { &mut *(ptr::from_mut(allocator) as *mut Self) }
    }
}

unsafe impl<A: ?Sized + Allocator> CrateAllocator for AllocatorNightlyCompat<A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, CrateAllocError> {
        <A as Allocator>::allocate(&self.0, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { <A as Allocator>::deallocate(&self.0, ptr, layout) };
    }

    #[inline(always)]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, CrateAllocError> {
        <A as Allocator>::allocate_zeroed(&self.0, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, CrateAllocError> {
        unsafe { <A as Allocator>::grow(&self.0, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, CrateAllocError> {
        unsafe { <A as Allocator>::grow_zeroed(&self.0, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, CrateAllocError> {
        unsafe { <A as Allocator>::shrink(&self.0, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    fn by_ref(&self) -> &Self
    where
        Self: Sized,
    {
        self
    }
}

unsafe impl<A: ?Sized + CrateAllocator> Allocator for AllocatorNightlyCompat<A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <A as CrateAllocator>::allocate(&self.0, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { <A as CrateAllocator>::deallocate(&self.0, ptr, layout) };
    }

    #[inline(always)]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <A as CrateAllocator>::allocate_zeroed(&self.0, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <A as CrateAllocator>::grow(&self.0, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <A as CrateAllocator>::grow_zeroed(&self.0, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <A as CrateAllocator>::shrink(&self.0, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    fn by_ref(&self) -> &Self
    where
        Self: Sized,
    {
        self
    }
}

impl From<AllocError> for CrateAllocError {
    #[inline(always)]
    fn from(_: AllocError) -> Self {
        CrateAllocError
    }
}

impl From<CrateAllocError> for AllocError {
    #[inline(always)]
    fn from(_: CrateAllocError) -> Self {
        AllocError
    }
}

unsafe impl<A, S> Allocator for BumpScope<'_, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { <Self as CrateAllocator>::deallocate(self, ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }
}

unsafe impl<A, S> Allocator for Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { <Self as CrateAllocator>::deallocate(self, ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }
}

unsafe impl<A: BumpAllocatorExt> Allocator for WithoutShrink<A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { <Self as CrateAllocator>::deallocate(self, ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }
}

unsafe impl<A: BumpAllocatorExt> Allocator for WithoutDealloc<A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as CrateAllocator>::allocate(self, layout).map_err(Into::into)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { <Self as CrateAllocator>::deallocate(self, ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::grow(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::grow_zeroed(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { <Self as CrateAllocator>::shrink(self, ptr, old_layout, new_layout).map_err(Into::into) }
    }
}

impl<T: ?Sized, A: Allocator> box_like::Sealed for Box<T, A> {
    type T = T;
    type A = A;

    #[inline(always)]
    unsafe fn from_raw_in(ptr: *mut Self::T, allocator: Self::A) -> Self {
        unsafe { Box::from_raw_in(ptr, allocator) }
    }
}

impl<T: ?Sized, A: Allocator> BoxLike for Box<T, A> {}

#[test]
fn test_compat() {
    use crate::settings::True;

    fn is_base_allocator<T: BaseAllocator<True>>(_: T) {}

    #[derive(Clone)]
    struct TestAllocator;

    unsafe impl Allocator for TestAllocator {
        fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> {
            unimplemented!()
        }

        unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {
            unimplemented!()
        }
    }

    #[cfg(feature = "alloc")]
    is_base_allocator(Global);
    is_base_allocator(AllocatorNightlyCompat(TestAllocator));
    is_base_allocator(AllocatorNightlyCompat::from_ref(&TestAllocator));
}
