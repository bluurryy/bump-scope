#![allow(clippy::missing_safety_doc)]

use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};

use crate::{
    alloc::{AllocError, Allocator},
    bump_down,
    polyfill::non_null,
    stats::{AnyStats, Stats},
    traits::assert_implements,
    up_align_usize_unchecked, BaseAllocator, Bump, BumpAllocator, BumpAllocatorScope, BumpScope, Checkpoint,
    MinimumAlignment, MutBumpAllocator, MutBumpAllocatorScope, SizedTypeProperties, SupportedMinimumAlignment,
    WithoutDealloc, WithoutShrink,
};

#[cfg(feature = "panic-on-alloc")]
use crate::{handle_alloc_error, panic_on_error};

// FIXME: #[cfg(feature = "panic-on-alloc")]
pub unsafe trait BumpAllocatorExt: BumpAllocator {
    type Stats<'b>: Into<AnyStats<'b>>
    where
        Self: 'b;

    fn stats(&self) -> Self::Stats<'_>;

    // FIXME: as_scope method?

    // FIXME: document that this requires guaranteed allocated bump
    /// Creates a checkpoint of the current bump position.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate alloc;
    /// # use bump_scope::{ Bump, BumpAllocatorExt};
    /// # use alloc::alloc::Layout;
    /// fn test(bump: impl BumpAllocatorExt) {
    ///     let checkpoint = bump.checkpoint();
    ///
    ///     {
    ///         let hello = bump.allocate_layout(Layout::new::<[u8;5]>());
    ///         assert_eq!(bump.stats().into().allocated(), 5);
    ///         # _ = hello;
    ///     }
    ///
    ///     unsafe { bump.reset_to(checkpoint); }
    ///     assert_eq!(bump.stats().into().allocated(), 0);
    /// }
    ///
    /// test(<Bump>::new());
    /// ```
    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        self.ptr().checkpoint()
    }

    /// Resets the bump position to a previously created checkpoint. The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    ///
    /// - the checkpoint must have been created by this bump allocator
    /// - the bump allocator must not have been [`reset`](crate::Bump::reset) since creation of this checkpoint
    /// - there must be no references to allocations made since creation of this checkpoint
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate alloc;
    /// # use bump_scope::{Bump, BumpAllocatorExt};
    /// # use alloc::alloc::Layout;
    /// fn test(bump: impl BumpAllocatorExt) {
    ///     let checkpoint = bump.checkpoint();
    ///     
    ///     {
    ///         let hello = bump.allocate_layout(Layout::new::<[u8;5]>());
    ///         assert_eq!(bump.stats().into().allocated(), 5);
    ///         # _ = hello;
    ///     }
    ///     
    ///     unsafe { bump.reset_to(checkpoint); }
    ///     assert_eq!(bump.stats().into().allocated(), 0);
    /// }
    ///
    /// test(<Bump>::new());
    /// ```
    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        // FIXME: re-add
        // debug_assert!(self.stats().big_to_small().any(|chunk| {
        //     chunk.header == checkpoint.chunk.cast() && chunk.contains_addr_or_end(checkpoint.address.get())
        // }));

        self.ptr().reset_to(checkpoint);
    }

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        allocate_layout(self, layout)
    }

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        try_allocate_layout(self, layout)
    }

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        allocate_sized(self)
    }

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        try_allocate_sized(self)
    }

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        allocate_slice(self, len)
    }

    /// A specialized version of [`allocate`](Allocator::allocate).
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice(self, len)
    }

    /// A specialized version of [`shrink`](Allocator::shrink).
    ///
    /// Returns `Some` if a shrink was performed, `None` if not.
    ///
    /// # Safety
    ///
    /// `new_len` must be less than `old_len`
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        shrink_slice(self, ptr, old_len, new_len)
    }
}

assert_implements! {
    [BumpAllocatorExt + ?Sized]

    dyn BumpAllocator
    &dyn BumpAllocator
    &mut dyn BumpAllocator

    dyn BumpAllocatorScope
    &dyn BumpAllocatorScope
    &mut dyn BumpAllocatorScope

    dyn MutBumpAllocator
    &dyn MutBumpAllocator
    &mut dyn MutBumpAllocator

    dyn MutBumpAllocatorScope
    &dyn MutBumpAllocatorScope
    &mut dyn MutBumpAllocatorScope
}

unsafe impl BumpAllocatorExt for dyn BumpAllocator + '_ {
    type Stats<'b>
        = AnyStats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> AnyStats<'_> {
        self.ptr().stats(self.chunk_header_size())
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice(self, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        shrink_slice(self, ptr, old_len, new_len)
    }
}

unsafe impl BumpAllocatorExt for dyn MutBumpAllocator + '_ {
    type Stats<'b>
        = AnyStats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> AnyStats<'_> {
        self.ptr().stats(self.chunk_header_size())
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice(self, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        shrink_slice(self, ptr, old_len, new_len)
    }
}

unsafe impl BumpAllocatorExt for dyn BumpAllocatorScope<'_> + '_ {
    type Stats<'b>
        = AnyStats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> AnyStats<'_> {
        self.ptr().stats(self.chunk_header_size())
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice(self, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        shrink_slice(self, ptr, old_len, new_len)
    }
}

unsafe impl BumpAllocatorExt for dyn MutBumpAllocatorScope<'_> + '_ {
    type Stats<'b>
        = AnyStats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> AnyStats<'_> {
        self.ptr().stats(self.chunk_header_size())
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        try_allocate_slice(self, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        shrink_slice(self, ptr, old_len, new_len)
    }
}

#[inline]
#[cfg(feature = "panic-on-alloc")]
fn allocate_layout(bump: impl BumpAllocator, layout: Layout) -> NonNull<u8> {
    match bump.allocate(layout) {
        Ok(ptr) => ptr.cast(),
        Err(AllocError) => handle_alloc_error(layout),
    }
}

#[inline]
fn try_allocate_layout(bump: impl BumpAllocator, layout: Layout) -> Result<NonNull<u8>, AllocError> {
    match bump.allocate(layout) {
        Ok(ptr) => Ok(ptr.cast()),
        Err(err) => Err(err),
    }
}

#[inline]
#[cfg(feature = "panic-on-alloc")]
fn allocate_sized<T>(bump: impl BumpAllocator) -> NonNull<T> {
    let layout = Layout::new::<T>();

    match bump.allocate(layout) {
        Ok(ptr) => ptr.cast(),
        Err(AllocError) => handle_alloc_error(Layout::new::<T>()),
    }
}

#[inline]
fn try_allocate_sized<T>(bump: impl BumpAllocator) -> Result<NonNull<T>, AllocError> {
    match bump.allocate(Layout::new::<T>()) {
        Ok(ptr) => Ok(ptr.cast()),
        Err(err) => Err(err),
    }
}

#[inline]
#[cfg(feature = "panic-on-alloc")]
fn allocate_slice<T>(bump: impl BumpAllocator, len: usize) -> NonNull<T> {
    let layout = match Layout::array::<T>(len) {
        Ok(layout) => layout,
        Err(_) => invalid_slice_layout(),
    };

    match bump.allocate(layout) {
        Ok(ptr) => ptr.cast(),
        Err(AllocError) => handle_alloc_error(layout),
    }
}

#[inline]
fn try_allocate_slice<T>(bump: impl BumpAllocator, len: usize) -> Result<NonNull<T>, AllocError> {
    let layout = match Layout::array::<T>(len) {
        Ok(layout) => layout,
        Err(_) => return Err(AllocError),
    };

    match bump.allocate(layout) {
        Ok(ptr) => Ok(ptr.cast()),
        Err(err) => Err(err),
    }
}

#[inline]
unsafe fn shrink_slice<T>(bump: impl BumpAllocator, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
    // FIXME: do shrink
    _ = (bump, ptr, old_len, new_len);
    None
}

unsafe impl<B: BumpAllocatorExt + ?Sized> BumpAllocatorExt for &B {
    type Stats<'b>
        = B::Stats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> Self::Stats<'_> {
        B::stats(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        B::allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        B::try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        B::allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        B::allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_slice(self, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        B::shrink_slice(self, ptr, old_len, new_len)
    }
}

unsafe impl<B: BumpAllocatorExt + ?Sized> BumpAllocatorExt for &mut B
where
    for<'b> &'b mut B: Allocator,
{
    type Stats<'b>
        = B::Stats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> Self::Stats<'_> {
        B::stats(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        B::allocate_layout(self, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        B::try_allocate_layout(self, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        B::allocate_sized(self)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_sized(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        B::allocate_slice(self, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_slice(self, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        B::shrink_slice(self, ptr, old_len, new_len)
    }
}

unsafe impl<B: BumpAllocatorExt> BumpAllocatorExt for WithoutDealloc<B> {
    type Stats<'b>
        = B::Stats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> Self::Stats<'_> {
        B::stats(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        B::allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        B::try_allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        B::allocate_sized(&self.0)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_sized(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        B::allocate_slice(&self.0, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_slice(&self.0, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        B::shrink_slice(&self.0, ptr, old_len, new_len)
    }
}

unsafe impl<B: BumpAllocatorExt> BumpAllocatorExt for WithoutShrink<B> {
    type Stats<'b>
        = B::Stats<'b>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> Self::Stats<'_> {
        B::stats(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        B::allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        B::try_allocate_layout(&self.0, layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        B::allocate_sized(&self.0)
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_sized(&self.0)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        B::allocate_slice(&self.0, len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        B::try_allocate_slice(&self.0, len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        B::shrink_slice(&self.0, ptr, old_len, new_len)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorExt
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Stats<'b>
        = Stats<'b, A, UP, GUARANTEED_ALLOCATED>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> Self::Stats<'_> {
        BumpScope::stats(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_layout(&self, layout: Layout) -> NonNull<u8> {
        self.alloc_layout(layout)
    }

    #[inline(always)]
    fn try_allocate_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        self.try_alloc_layout(layout)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_sized<T>(&self) -> NonNull<T> {
        panic_on_error(self.do_alloc_sized())
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        self.do_alloc_sized()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        panic_on_error(self.do_alloc_slice(len))
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        self.do_alloc_slice(len)
    }

    #[inline]
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
                let end = non_null::addr(old_ptr).get() + new_size;

                // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(end, MIN_ALIGN);

                self.chunk.get().set_pos_addr(new_pos);
                Some(old_ptr.cast())
            } else {
                let old_addr = non_null::addr(old_ptr);
                let old_addr_old_end = NonZeroUsize::new_unchecked(old_addr.get() + old_size);

                let new_addr = bump_down(old_addr_old_end, new_size, T::ALIGN.max(MIN_ALIGN));
                let new_addr = NonZeroUsize::new_unchecked(new_addr);
                let old_addr_new_end = NonZeroUsize::new_unchecked(old_addr.get() + new_size);

                let new_ptr = non_null::with_addr(old_ptr, new_addr);
                let overlaps = old_addr_new_end > new_addr;

                if overlaps {
                    non_null::copy(old_ptr, new_ptr, new_size);
                } else {
                    non_null::copy_nonoverlapping(old_ptr, new_ptr, new_size);
                }

                self.chunk.get().set_pos(new_ptr);
                Some(new_ptr.cast())
            }
        }
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorExt
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Stats<'b>
        = Stats<'b, A, UP, GUARANTEED_ALLOCATED>
    where
        Self: 'b;

    #[inline(always)]
    fn stats(&self) -> Self::Stats<'_> {
        self.as_scope().stats()
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
    fn allocate_sized<T>(&self) -> NonNull<T> {
        self.as_scope().allocate_sized()
    }

    #[inline(always)]
    fn try_allocate_sized<T>(&self) -> Result<NonNull<T>, AllocError> {
        self.as_scope().try_allocate_sized()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn allocate_slice<T>(&self, len: usize) -> NonNull<T> {
        self.as_scope().allocate_slice(len)
    }

    #[inline(always)]
    fn try_allocate_slice<T>(&self, len: usize) -> Result<NonNull<T>, AllocError> {
        self.as_scope().try_allocate_slice(len)
    }

    #[inline(always)]
    unsafe fn shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        self.as_scope().shrink_slice(ptr, old_len, new_len)
    }
}

#[cold]
#[inline(never)]
#[cfg(feature = "panic-on-alloc")]
pub(crate) const fn invalid_slice_layout() -> ! {
    panic!("invalid slice layout");
}
