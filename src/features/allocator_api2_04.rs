use allocator_api2_04::alloc::{AllocError, Allocator};

#[cfg(feature = "alloc")]
use allocator_api2_04::{alloc::Global, boxed::Box};

use crate::{
    BaseAllocator, Bump, BumpScope, WithoutDealloc, WithoutShrink, alloc::AllocError as CrateAllocError,
    settings::BumpAllocatorSettings, traits::BumpAllocatorCore,
};

#[cfg(feature = "alloc")]
use crate::alloc::{BoxLike, box_like};

use super::allocator_util::{allocator_compat_wrapper, impl_allocator_via_allocator};

allocator_compat_wrapper! {
    /// Wraps an <code>allocator_api2::alloc::[Allocator](allocator_api2_04::alloc::Allocator)</code> to implement
    /// <code>bump_scope::alloc::[Allocator](crate::alloc::Allocator)</code> and vice versa.
    ///
    /// # Example
    ///
    /// ```
    /// # use allocator_api2_04 as allocator_api2;
    /// # use core::{alloc::Layout, ptr::NonNull};
    /// # use allocator_api2::alloc::{AllocError, Global};
    /// use allocator_api2::alloc::Allocator;
    ///
    /// use bump_scope::{Bump, alloc::compat::AllocatorApi2V04Compat};
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
    /// let bump: Bump<_> = Bump::new_in(AllocatorApi2V04Compat(MyAllocatorApi2Allocator));
    /// # _ = bump;
    /// ```
    struct AllocatorApi2V04Compat for allocator_api2_04
}

impl_allocator_via_allocator! {
    self;

    #[cfg(feature = "alloc")]
    use {self} for crate as allocator_api2_04 impl[] Global

    use {self} for allocator_api2_04 as crate impl[A, S] Bump<A, S>
    where [
        A: BaseAllocator<S::GuaranteedAllocated>,
        S: BumpAllocatorSettings,
    ]

    use {self} for allocator_api2_04 as crate impl[A, S] BumpScope<'_, A, S>
    where [
        A: BaseAllocator<S::GuaranteedAllocated>,
        S: BumpAllocatorSettings,
    ]

    use {self} for allocator_api2_04 as crate impl[A: BumpAllocatorCore] WithoutShrink<A>
    use {self} for allocator_api2_04 as crate impl[A: BumpAllocatorCore] WithoutDealloc<A>
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

#[cfg(feature = "alloc")]
impl<T: ?Sized, A: Allocator> box_like::Sealed for Box<T, A> {
    type T = T;
    type A = A;

    #[inline(always)]
    unsafe fn from_raw_in(ptr: *mut Self::T, allocator: Self::A) -> Self {
        unsafe { Box::from_raw_in(ptr, allocator) }
    }
}

#[cfg(feature = "alloc")]
impl<T: ?Sized, A: Allocator> BoxLike for Box<T, A> {}

#[test]
fn test_compat() {
    use core::{alloc::Layout, ptr::NonNull};

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
    is_base_allocator(AllocatorApi2V04Compat(TestAllocator));
    is_base_allocator(AllocatorApi2V04Compat::from_ref(&TestAllocator));
}
