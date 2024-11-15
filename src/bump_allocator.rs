use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

use allocator_api2::alloc::{AllocError, Allocator};

use crate::{
    bump_down, polyfill::nonnull, up_align_usize_unchecked, BaseAllocator, Bump, BumpScope, MinimumAlignment,
    SizedTypeProperties, Stats, SupportedMinimumAlignment,
};

#[cfg(not(no_global_oom_handling))]
use crate::{handle_alloc_error, infallible};

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
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    fn stats(&self) -> Stats<'_> {
        Stats { current: None }
    }

    /// A specialized version of `allocate`.
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        #[cfg(not(no_global_oom_handling))]
        {
            match self.allocate(layout) {
                Ok(ptr) => ptr.cast(),
                Err(AllocError) => handle_alloc_error(layout),
            }
        }

        #[cfg(no_global_oom_handling)]
        {
            _ = layout;
            unreachable!()
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
        #[cfg(not(no_global_oom_handling))]
        {
            let layout = Layout::new::<T>();

            match self.allocate(layout) {
                Ok(ptr) => ptr.cast(),
                Err(AllocError) => handle_alloc_error(Layout::new::<T>()),
            }
        }

        #[cfg(no_global_oom_handling)]
        {
            unreachable!()
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
        #[cfg(not(no_global_oom_handling))]
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

        #[cfg(no_global_oom_handling)]
        {
            _ = len;
            unreachable!()
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

    /// A specialized version of `shrink`.
    ///
    /// Returns `Some` if a shrink was performed, `None` if not.
    ///
    /// # Safety
    ///
    /// `new_len` must be less than `old_len`
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>>
    where
        Self: Sized,
    {
        _ = (ptr, old_len, new_len);
        None
    }
}

unsafe impl<A: BumpAllocator> BumpAllocator for &A {
    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        A::stats(self)
    }

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
    #[cfg(not(no_global_oom_handling))]
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
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        A::shrink_slice(self, ptr, old_len, new_len)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        BumpScope::stats(self)
    }

    #[inline(always)]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        #[cfg(not(no_global_oom_handling))]
        {
            BumpScope::alloc_layout(self, layout)
        }

        #[cfg(no_global_oom_handling)]
        {
            _ = layout;
            unreachable!()
        }
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
        #[cfg(not(no_global_oom_handling))]
        {
            infallible(self.do_alloc_sized())
        }

        #[cfg(no_global_oom_handling)]
        {
            unreachable!()
        }
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
        #[cfg(not(no_global_oom_handling))]
        {
            infallible(self.do_alloc_slice(len))
        }

        #[cfg(no_global_oom_handling)]
        {
            _ = len;
            unreachable!()
        }
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        self.do_alloc_slice(len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        let old_ptr = ptr.cast::<u8>();
        let old_size = old_len * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_len * T::SIZE; // its less than the capacity so this can't overflow

        // Adapted from `Allocator::shrink`.
        unsafe {
            let is_last = if UP {
                old_ptr.as_ptr().add(old_size) == self.chunk.get().pos().as_ptr()
            } else {
                old_ptr == self.chunk.get().pos()
            };

            // if that's not the last allocation, there is nothing we can do
            if !is_last {
                return None;
            }

            if UP {
                let end = nonnull::addr(old_ptr).get() + new_size;

                // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(end, MIN_ALIGN);

                self.chunk.get().set_pos_addr(new_pos);
                Some(old_ptr.cast())
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
                Some(new_ptr.cast())
            }
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
    fn stats(&self) -> Stats<'_> {
        Bump::stats(self)
    }

    #[inline(always)]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        self.as_scope().allocate_layout(layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        self.as_scope().try_allocate_layout(layout)
    }

    #[inline(always)]
    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        self.as_scope().allocate_sized()
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        self.as_scope().try_allocate_sized()
    }

    #[inline(always)]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        self.as_scope().allocate_slice(len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        self.as_scope().try_allocate_slice(len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>>
    where
        Self: Sized,
    {
        self.as_scope().shrink_slice(ptr, old_len, new_len)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for &mut BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        BumpScope::stats(self)
    }

    #[inline(always)]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        BumpScope::allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        BumpScope::try_allocate_layout(self, layout)
    }

    #[inline(always)]
    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        BumpScope::allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        BumpScope::try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(not(no_global_oom_handling))]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        BumpScope::allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        BumpScope::try_allocate_slice(self, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        BumpScope::shrink_slice(self, ptr, old_len, new_len)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for &mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        Bump::stats(self)
    }

    #[inline(always)]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        Bump::allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        Bump::try_allocate_layout(self, layout)
    }

    #[inline(always)]
    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        Bump::allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        Bump::try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(not(no_global_oom_handling))]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        Bump::allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        Bump::try_allocate_slice(self, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        Bump::shrink_slice(self, ptr, old_len, new_len)
    }
}

/// A [`BumpAllocator`] who has exclusive access to allocation.
///
/// # Safety
///
/// TODO
///
/// [into_box]: crate::BumpBox::into_box
/// [prepare_slice_allocation]: Self::prepare_slice_allocation
/// [try_prepare_slice_allocation]: Self::try_prepare_slice_allocation
pub unsafe trait BumpAllocatorMut: BumpAllocator {
    /// Does not allocate, just returns a slice of `T` that are currently available.
    ///
    /// # Panics
    ///
    /// Panics when the allocation fails.
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized;

    /// Does not allocate, just returns a slice of `T` that are currently available.
    ///
    /// # Errors
    ///
    /// Errors when the allocation fails.
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized;

    /// Allocate part of a valid free slice returned by `(try_)prepare_slice_allocation`.
    ///
    /// # Safety
    ///
    /// - `ptr + cap` must be a slice returned by `(try_)prepare_slice_allocation`. No allocation,
    ///   grow, shrink or deallocate must have been called since then.
    /// - `len` must be less than or equal to `cap`
    unsafe fn use_reserved_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized;
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorMut
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        #[cfg(not(no_global_oom_handling))]
        {
            let (ptr, len) = infallible(BumpScope::alloc_greedy(self, len));
            nonnull::slice_from_raw_parts(ptr, len)
        }

        #[cfg(no_global_oom_handling)]
        {
            _ = len;
            unreachable!()
        }
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        let (ptr, len) = BumpScope::alloc_greedy(self, len)?;
        Ok(nonnull::slice_from_raw_parts(ptr, len))
    }

    #[inline(always)]
    unsafe fn use_reserved_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        BumpScope::consolidate_greed(self, ptr, len, cap)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorMut
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        self.as_mut_scope().prepare_slice_allocation(len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        self.as_mut_scope().try_prepare_slice_allocation(len)
    }

    #[inline(always)]
    unsafe fn use_reserved_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        self.as_mut_scope().use_reserved_allocation(ptr, len, cap)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorMut
    for &mut BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        BumpScope::prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        BumpScope::try_prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    unsafe fn use_reserved_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        BumpScope::use_reserved_allocation(self, ptr, len, cap)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorMut
    for &mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn prepare_slice_allocation<T>(&mut self, len: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        Bump::prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    fn try_prepare_slice_allocation<T>(&mut self, len: usize) -> Result<NonNull<[T]>, AllocError>
    where
        Self: Sized,
    {
        Bump::try_prepare_slice_allocation(self, len)
    }

    #[inline(always)]
    unsafe fn use_reserved_allocation<T>(&mut self, ptr: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>
    where
        Self: Sized,
    {
        Bump::use_reserved_allocation(self, ptr, len, cap)
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
pub unsafe trait BumpAllocatorScope<'a>: BumpAllocator {
    // TODO: implement `stats` that live for `'a`?
}

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

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for &mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

unsafe impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorScope<'a>
    for &'a mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

/// Shorthand for <code>[BumpAllocatorScope]<'a> + [BumpAllocatorMut]</code>
pub trait BumpAllocatorScopeMut<'a>: BumpAllocatorScope<'a> + BumpAllocatorMut {}
impl<'a, T: BumpAllocatorScope<'a> + BumpAllocatorMut> BumpAllocatorScopeMut<'a> for T {}

#[cold]
#[inline(never)]
#[cfg(not(no_global_oom_handling))]
pub const fn invalid_slice_layout() -> ! {
    panic!("invalid slice layout");
}
