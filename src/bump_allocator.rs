use core::{alloc::Layout, ptr::NonNull};
use std::alloc::handle_alloc_error;

use allocator_api2::alloc::{AllocError, Allocator};

use crate::{BaseAllocator, Bump, BumpScope, MinimumAlignment, SupportedMinimumAlignment};

/// An allocator that allows `grow(_zeroed)`, `shrink` and `deallocate` calls with pointers that were not allocated by this allocator.
///
/// This trait is used for [`BumpBox::into_box`][into_box] to allow safely converting a `BumpBox` into a `Box`.
///
/// # Safety
///
/// This trait must only be implemented when
/// - `grow(_zeroed)`, `shrink` and `deallocate` can be called with a pointer that was not allocated by this Allocator
/// - `deallocate` can be called with any pointer or alignment when the size is `0`
///
/// [into_box]: crate::BumpBox::into_box
#[allow(clippy::missing_errors_doc)]
pub unsafe trait BumpAllocator: Allocator {
    /// A specialized version of `allocate`.
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        match self.allocate(layout) {
            Ok(ptr) => ptr.cast(),
            Err(AllocError) => handle_alloc_error(layout),
        }
    }

    /// A specialized version of `allocate`.
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        match self.allocate(layout) {
            Ok(ptr) => Ok(ptr.cast()),
            Err(err) => Err(err),
        }
    }

    /// A specialized version of `allocate`.
    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        let layout = Layout::new::<T>();

        match self.allocate(layout) {
            Ok(ptr) => ptr.cast(),
            Err(AllocError) => handle_alloc_error(Layout::new::<T>()),
        }
    }

    /// A specialized version of `allocate`.
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        match self.allocate(Layout::new::<T>()) {
            Ok(ptr) => Ok(ptr.cast()),
            Err(err) => Err(err),
        }
    }

    /// A specialized version of `allocate`.
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        let layout = match Layout::array::<T>(len) {
            Ok(layout) => layout,
            Err(_) => invalid_slice_layout(),
        };

        match self.allocate(layout) {
            Ok(ptr) => ptr.cast(),
            Err(AllocError) => handle_alloc_error(layout),
        }
    }

    /// A specialized version of `allocate`.
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        let layout = match Layout::array::<T>(len) {
            Ok(layout) => layout,
            Err(_) => return Err(AllocError),
        };

        match self.allocate(layout) {
            Ok(ptr) => Ok(ptr.cast()),
            Err(err) => Err(err),
        }
    }
}

unsafe impl<A: BumpAllocator> BumpAllocator for &A {}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

/// An allocator that makes allocations with a lifetime of `'a`.
///
/// # Safety
///
/// This trait must only be implemented when allocations live for `'a`.
/// In other words this function must be sound:
///
/// ```
/// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
/// # #![allow(dead_code)]
/// use bump_scope::BumpAllocatorScope;
/// use core::alloc::Layout;
///
/// fn allocate_zeroed_bytes<'a>(allocator: impl BumpAllocatorScope<'a>, len: usize) -> &'a [u8] {
///     let layout = Layout::array::<u8>(len).unwrap();
///     let ptr = allocator.allocate_zeroed(layout).unwrap();
///     unsafe { ptr.as_ref() }
/// }
/// ```
pub unsafe trait BumpAllocatorScope<'a>: BumpAllocator {}

unsafe impl<'a, A: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for &A {}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for &'a Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

#[cold]
#[inline(never)]
#[cfg(not(no_global_oom_handling))]
pub const fn invalid_slice_layout() -> ! {
    panic!("invalid slice layout");
}
