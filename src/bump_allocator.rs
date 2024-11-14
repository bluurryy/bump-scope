use alloc::alloc::handle_alloc_error;
use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

use allocator_api2::alloc::{AllocError, Allocator};

use crate::{
    bump_down, infallible, polyfill::nonnull, raw_fixed_bump_vec::RawFixedBumpVec, up_align_usize_unchecked, BaseAllocator,
    Bump, BumpScope, MinimumAlignment, SizedTypeProperties, SupportedMinimumAlignment,
};

/// An allocator that allows `grow(_zeroed)`, `shrink` and `deallocate` calls with pointers that were not allocated by this allocator.
///
/// This trait is used for [`BumpBox::into_box`][into_box] to allow safely converting a `BumpBox` into a `Box`.
///
/// # Safety
///
/// This trait must only be implemented when
/// - `grow(_zeroed)`, `shrink` and `deallocate` can be called with a pointer that was not allocated by this Allocator
/// - `deallocate` can be called with any pointer or alignment when the size is `0`
/// - `shrink` does not error
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

    /// Returns whether this memory block is the last allocation.
    fn is_last_allocation(&self, allocation: NonNull<[u8]>) -> bool {
        _ = allocation;
        false
    }

    /// TODO
    fn shrink_to_fit<T>(&self, vec: &mut RawFixedBumpVec<T>) {
        _ = vec;
        todo!()
    }
}

unsafe impl<A: BumpAllocator> BumpAllocator for &A {
    #[inline(always)]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        A::allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        A::try_allocate_layout(self, layout)
    }

    #[inline(always)]
    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_sized(self)
    }

    #[inline(always)]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_slice(self, len)
    }

    #[inline(always)]
    fn is_last_allocation(&self, allocation: NonNull<[u8]>) -> bool {
        A::is_last_allocation(self, allocation)
    }

    #[inline(always)]
    fn shrink_to_fit<T>(&self, vec: &mut RawFixedBumpVec<T>) {
        A::shrink_to_fit(self, vec);
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        BumpScope::alloc_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        BumpScope::try_alloc_layout(self, layout)
    }

    #[inline(always)]
    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        infallible(self.do_alloc_sized())
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        self.do_alloc_sized()
    }

    #[inline(always)]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        infallible(self.do_alloc_slice(len))
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        self.do_alloc_slice(len)
    }

    #[inline(always)]
    fn is_last_allocation(&self, allocation: NonNull<[u8]>) -> bool {
        unsafe {
            let size = allocation.len();
            let start = allocation.cast::<u8>().as_ptr();
            let end = unsafe { start.add(size) };

            if UP {
                end == self.chunk.get().pos().as_ptr()
            } else {
                start == self.chunk.get().pos().as_ptr()
            }
        }
    }

    fn shrink_to_fit<T>(&self, vec: &mut RawFixedBumpVec<T>) {
        let old_ptr = vec.initialized.ptr.cast::<u8>();
        let old_size = vec.capacity * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = vec.len() * T::SIZE; // its less than the capacity so this can't overflow

        // Adapted from `Allocator::shrink`.
        unsafe {
            let is_last = if UP {
                old_ptr.as_ptr().add(old_size) == self.chunk.get().pos().as_ptr()
            } else {
                old_ptr == self.chunk.get().pos()
            };

            // if that's not the last allocation, there is nothing we can do
            if !is_last {
                return;
            }

            if UP {
                let end = nonnull::addr(old_ptr).get() + new_size;

                // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(end, MIN_ALIGN);

                self.chunk.get().set_pos_addr(new_pos);
            } else {
                let old_addr = nonnull::addr(old_ptr);
                let old_addr_old_end = NonZeroUsize::new_unchecked(old_addr.get() + old_size);

                let new_addr = bump_down(old_addr_old_end, new_size, T::ALIGN.max(MIN_ALIGN));
                let new_addr = NonZeroUsize::new_unchecked(new_addr);
                let old_addr_new_end = NonZeroUsize::new_unchecked(old_addr.get() + new_size);

                let new_ptr = nonnull::with_addr(old_ptr, new_addr);
                let overlaps = old_addr_new_end > new_addr;

                if overlaps {
                    nonnull::copy(old_ptr, new_ptr, new_size);
                } else {
                    nonnull::copy_nonoverlapping(old_ptr, new_ptr, new_size);
                }

                self.chunk.get().set_pos(new_ptr);
                nonnull::set_ptr(&mut vec.initialized.ptr, new_ptr.cast());
            }

            vec.capacity = vec.len();
        }
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn is_last_allocation(&self, allocation: NonNull<[u8]>) -> bool {
        self.as_scope().is_last_allocation(allocation)
    }

    #[inline(always)]
    fn shrink_to_fit<T>(&self, vec: &mut RawFixedBumpVec<T>) {
        self.as_scope().shrink_to_fit(vec);
    }
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
