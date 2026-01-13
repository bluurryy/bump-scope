use core::alloc::{AllocError, Allocator};

#[cfg(feature = "alloc")]
use alloc_crate::{alloc::Global, boxed::Box};

use crate::{
    BaseAllocator, Bump, BumpScope, WithoutDealloc, WithoutShrink,
    alloc::{AllocError as CrateAllocError, BoxLike, box_like},
    settings::BumpAllocatorSettings,
    traits::BumpAllocatorCore,
};

use super::allocator_util::{allocator_compat_wrapper, impl_allocator_via_allocator};

allocator_compat_wrapper! {
    /// Wraps an <code>alloc::alloc::[Allocator](core::alloc::Allocator)</code> to implement
    /// <code>bump_scope::alloc::[Allocator](crate::alloc::Allocator)</code> and vice versa.
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
    struct AllocatorNightlyCompat for core
}

impl_allocator_via_allocator! {
    self;

    #[cfg(feature = "alloc")]
    use {self} for crate as core impl[] Global

    use {self} for core as crate impl[A, S] Bump<A, S>
    where [
        A: BaseAllocator<S::GuaranteedAllocated>,
        S: BumpAllocatorSettings,
    ]

    use {self} for core as crate impl[A, S] BumpScope<'_, A, S>
    where [
        A: BaseAllocator<S::GuaranteedAllocated>,
        S: BumpAllocatorSettings,
    ]

    use {self} for core as crate impl[A: BumpAllocatorCore] WithoutShrink<A>
    use {self} for core as crate impl[A: BumpAllocatorCore] WithoutDealloc<A>
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
    is_base_allocator(AllocatorNightlyCompat(TestAllocator));
    is_base_allocator(AllocatorNightlyCompat::from_ref(&TestAllocator));
}
