use core::{
    alloc::Layout,
    ptr::{self, NonNull},
};

use allocator_api2_02::alloc::{AllocError, Allocator};

#[cfg(feature = "alloc")]
#[cfg(not(feature = "nightly-allocator-api"))]
use allocator_api2_02::{alloc::Global, boxed::Box};

use crate::alloc::{AllocError as CrateAllocError, Allocator as CrateAllocator};

#[cfg(not(feature = "nightly-allocator-api"))]
use crate::{Bump, BumpAllocator, BumpScope, MinimumAlignment, SupportedMinimumAlignment, WithoutDealloc, WithoutShrink};

#[cfg(feature = "alloc")]
#[cfg(not(feature = "nightly-allocator-api"))]
use crate::alloc::{BoxLike, box_like};

#[cfg(any(test, not(feature = "nightly-allocator-api")))]
use crate::BaseAllocator;

#[cfg(feature = "alloc")]
#[cfg(not(feature = "nightly-allocator-api"))]
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

/// Wraps an <code>allocator_api2::alloc::[Allocator](Allocator)</code> to implement
/// <code>bump_scope::alloc::[Allocator](CrateAllocator)</code> and vice versa.
///
/// # Example
///
/// ```
/// # use allocator_api2_02 as allocator_api2;
/// # use core::{alloc::Layout, ptr::NonNull};
/// # use allocator_api2::alloc::{AllocError, Global};
/// use allocator_api2::alloc::Allocator;
///
/// use bump_scope::{Bump, alloc::compat::AllocatorApi2V02Compat};
///
/// #[derive(Clone)]
/// struct MyAllocatorApi2Allocator;
///
/// unsafe impl Allocator for MyAllocatorApi2Allocator {
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
/// let bump: Bump<_> = Bump::new_in(AllocatorApi2V02Compat(MyAllocatorApi2Allocator));
/// # _ = bump;
/// ```
#[repr(transparent)]
#[derive(Debug, Default, Clone)]
pub struct AllocatorApi2V02Compat<A: ?Sized>(pub A);

impl<A: ?Sized> AllocatorApi2V02Compat<A> {
    #[inline(always)]
    #[allow(missing_docs)]
    pub fn from_ref(allocator: &A) -> &Self {
        unsafe { &*(ptr::from_ref(allocator) as *const Self) }
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn from_mut(allocator: &mut A) -> &mut Self {
        unsafe { &mut *(ptr::from_mut(allocator) as *mut Self) }
    }
}

unsafe impl<A: ?Sized + Allocator> CrateAllocator for AllocatorApi2V02Compat<A> {
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

unsafe impl<A: ?Sized + CrateAllocator> Allocator for AllocatorApi2V02Compat<A> {
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

#[cfg(not(feature = "nightly-allocator-api"))]
impl From<AllocError> for CrateAllocError {
    #[inline(always)]
    fn from(_: AllocError) -> Self {
        CrateAllocError
    }
}

#[cfg(not(feature = "nightly-allocator-api"))]
impl From<CrateAllocError> for AllocError {
    #[inline(always)]
    fn from(_: CrateAllocError) -> Self {
        AllocError
    }
}

#[cfg(not(feature = "nightly-allocator-api"))]
unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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

#[cfg(not(feature = "nightly-allocator-api"))]
unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for &mut BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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

#[cfg(not(feature = "nightly-allocator-api"))]
unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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

#[cfg(not(feature = "nightly-allocator-api"))]
unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for &mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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

#[cfg(not(feature = "nightly-allocator-api"))]
unsafe impl<A: BumpAllocator> Allocator for WithoutShrink<A> {
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

#[cfg(not(feature = "nightly-allocator-api"))]
unsafe impl<A: BumpAllocator> Allocator for WithoutDealloc<A> {
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

#[cfg(feature = "alloc")]
#[cfg(not(feature = "nightly-allocator-api"))]
impl<T: ?Sized, A: Allocator> box_like::Sealed for Box<T, A> {
    type T = T;
    type A = A;

    unsafe fn from_raw_in(ptr: *mut Self::T, allocator: Self::A) -> Self {
        unsafe { Box::from_raw_in(ptr, allocator) }
    }
}

#[cfg(feature = "alloc")]
#[cfg(not(feature = "nightly-allocator-api"))]
impl<T: ?Sized, A: Allocator> BoxLike for Box<T, A> {}

#[test]
fn test_compat() {
    fn is_base_allocator<T: BaseAllocator>(_: T) {}

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
    #[cfg(not(feature = "nightly-allocator-api"))]
    is_base_allocator(Global);
    is_base_allocator(AllocatorApi2V02Compat(TestAllocator));
    is_base_allocator(AllocatorApi2V02Compat::from_ref(&TestAllocator));
}
