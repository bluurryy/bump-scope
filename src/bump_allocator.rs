use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

use allocator_api2::alloc::{AllocError, Allocator};

use crate::{
    bump_down, polyfill::nonnull, up_align_usize_unchecked, BaseAllocator, Bump, BumpScope, MinimumAlignment,
    SizedTypeProperties, Stats, SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// A bump allocator.
///
/// This trait guarantees that it can be used as the allocator for an allocation that wasn't made by itself.
///
/// When handling allocations from a foreign allocator:
/// - `grow` will always allocate a new memory block
/// - `deallocate` and `shrink` will do nothing (`shrink` may allocate iff the alignment increases)
///
/// This makes functions such as [`BumpVec::from_parts`][from_parts] and [`BumpBox::into_box`][into_box] possible.
///
/// # Safety
///
/// This trait must only be implemented when
/// - `grow(_zeroed)`, `shrink` and `deallocate` can be called with a pointer that was not allocated by this allocator
/// - `deallocate` can be called with any pointer or alignment when the size is `0`
///
/// [into_box]: crate::BumpBox::into_box
/// [from_parts]: crate::BumpVec::from_parts
// FIXME: properly document the methods and remove `doc(hidden)`
// TODO(docs): this should be a super trait of `Allocator`, but we want a blanket impl for `&mut A` which wouldn't work for `Allocator`
#[allow(clippy::missing_errors_doc)]
pub unsafe trait BumpAllocator {
    /// Attempts to extend the memory block.
    ///
    /// Returns a new [`NonNull<[u8]>`][NonNull] containing a pointer and the actual size of the allocated
    /// memory. The pointer is suitable for holding data described by `new_layout`. To accomplish
    /// this, the allocator may extend the allocation referenced by `ptr` to fit the new layout.
    ///
    /// If this returns `Ok`, then ownership of the memory block referenced by `ptr` has been
    /// transferred to this allocator. Any access to the old `ptr` is Undefined Behavior, even if the
    /// allocation was grown in-place. The newly returned pointer is the only valid pointer
    /// for accessing this memory now.
    ///
    /// If this method returns `Err`, then ownership of the memory block has not been transferred to
    /// this allocator, and the contents of the memory block are unaltered.
    ///
    /// # Safety
    ///
    /// * `ptr` must denote a block of memory [*currently allocated*] via this allocator.
    /// * `old_layout` must [*fit*] that block of memory (The `new_layout` argument need not fit it.).
    /// * `new_layout.size()` must be greater than or equal to `old_layout.size()`.
    ///
    /// Note that `new_layout.align()` need not be the same as `old_layout.align()`.
    ///
    /// [*currently allocated*]: #currently-allocated-memory
    /// [*fit*]: #memory-fitting
    ///
    /// # Errors
    ///
    /// Returns `Err` if the new layout does not meet the allocator's size and alignment
    /// constraints of the allocator, or if growing otherwise fails.
    ///
    /// Implementations are encouraged to return `Err` on memory exhaustion rather than panicking or
    /// aborting, but this is not a strict requirement. (Specifically: it is *legal* to implement
    /// this trait atop an underlying native allocation library that aborts on memory exhaustion.)
    ///
    /// Clients wishing to abort computation in response to an allocation error are encouraged to
    /// call the [`handle_alloc_error`] function, rather than directly invoking `panic!` or similar.
    ///
    /// [`handle_alloc_error`]: ../../alloc/alloc/fn.handle_alloc_error.html
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError>;

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// * `ptr` must denote a block of memory [*currently allocated*] via this allocator, and
    /// * `layout` must [*fit*] that block of memory.
    ///
    /// [*currently allocated*]: #currently-allocated-memory
    /// [*fit*]: #memory-fitting
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout);

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_bytes`, <code>self.[stats](Self::stats)().[remaining](Stats::remaining)()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump };
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve_bytes(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    fn reserve_bytes(&self, additional: usize) {
        _ = additional;
        todo!("TODO") // TODO
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    fn stats(&self) -> Stats<'_> {
        Stats::unallocated()
    }

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[doc(hidden)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8>;

    /// Allocates memory as described by the given `Layout`.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[doc(hidden)]
    #[cfg(feature = "panic-on-alloc")]
    #[deprecated = "use `allocate_layout` instead"]
    fn alloc_layout(&self, layout: Layout) -> NonNull<u8> {
        self.allocate_layout(layout)
    }

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    #[doc(hidden)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError>;

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[doc(hidden)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T>;

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    #[doc(hidden)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized;

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[doc(hidden)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized;

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    #[doc(hidden)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized;

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[doc(hidden)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T>
    where
        Self: Sized;

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    #[doc(hidden)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized;

    /// A specialized version of [`shrink`](Allocator::shrink).
    ///
    /// Returns `Some` if a shrink was performed, `None` if not.
    ///
    /// # Safety
    ///
    /// `new_len` must be less than `old_len`
    #[doc(hidden)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>>
    where
        Self: Sized;
}

unsafe impl<A: BumpAllocator> BumpAllocator for &A {
    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        A::grow(self, ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        A::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        A::stats(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        A::allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        A::try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
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
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_slice(self, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_slice_for(self, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_slice_for(self, slice)
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

unsafe impl<A: BumpAllocator> BumpAllocator for &mut A {
    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        A::grow(self, ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        A::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        A::stats(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        A::allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        A::try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
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
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_slice(self, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_slice_for(self, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_slice_for(self, slice)
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

unsafe impl<A: BumpAllocator> BumpAllocator for WithoutDealloc<A> {
    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        A::grow(&self.0, ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        A::deallocate(&self.0, ptr, layout);
    }

    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        A::stats(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        A::allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        A::try_allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_sized(&self.0)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_sized(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_slice(&self.0, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_slice(&self.0, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_slice_for(&self.0, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_slice_for(&self.0, slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        A::shrink_slice(&self.0, ptr, old_len, new_len)
    }
}

unsafe impl<A: BumpAllocator> BumpAllocator for WithoutShrink<A> {
    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        A::grow(&self.0, ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        A::deallocate(&self.0, ptr, layout);
    }

    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        A::stats(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        A::allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        A::try_allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_sized(&self.0)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_sized(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_slice(&self.0, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_slice(&self.0, len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T>
    where
        Self: Sized,
    {
        A::allocate_slice_for(&self.0, slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        A::try_allocate_slice_for(&self.0, slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        A::shrink_slice(&self.0, ptr, old_len, new_len)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as Allocator>::grow(self, ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <Self as Allocator>::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        BumpScope::stats(self).not_guaranteed_allocated()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        BumpScope::alloc_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        BumpScope::try_alloc_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T>
    where
        Self: Sized,
    {
        panic_on_error(self.do_alloc_sized())
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        self.do_alloc_sized()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T>
    where
        Self: Sized,
    {
        panic_on_error(self.do_alloc_slice(len))
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        self.do_alloc_slice(len)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T>
    where
        Self: Sized,
    {
        panic_on_error(self.do_alloc_slice_for(slice))
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        self.do_alloc_slice_for(slice)
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
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        <Self as Allocator>::grow(self, ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        <Self as Allocator>::deallocate(self, ptr, layout);
    }

    #[inline(always)]
    fn stats(&self) -> Stats<'_> {
        Bump::stats(self).not_guaranteed_allocated()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        self.as_scope().allocate_layout(layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        self.as_scope().try_allocate_layout(layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
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
    #[cfg(feature = "panic-on-alloc")]
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
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice_for<T>(&self, slice: &[T]) -> NonNull<T>
    where
        Self: Sized,
    {
        self.as_scope().allocate_slice_for(slice)
    }

    #[inline(always)]
    fn try_allocate_slice_for<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError>
    where
        Self: Sized,
    {
        self.as_scope().try_allocate_slice_for(slice)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>>
    where
        Self: Sized,
    {
        self.as_scope().shrink_slice(ptr, old_len, new_len)
    }
}
