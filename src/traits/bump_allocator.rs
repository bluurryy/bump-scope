use core::{alloc::Layout, ops::Range, ptr::NonNull};

use crate::{
    alloc::{AllocError, Allocator},
    chunk_header::unallocated_chunk_header,
    layout::CustomLayout,
    polyfill::non_null,
    raw_chunk::RawChunk,
    stats::AnyStats,
    traits::{assert_dyn_compatible, assert_implements},
    BaseAllocator, Bump, BumpAllocatorScope, BumpScope, Checkpoint, MinimumAlignment, MutBumpAllocator,
    MutBumpAllocatorScope, SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};

/// A bump allocator.
///
/// This trait provides additional methods and guarantees on top of an [`Allocator`].
///
/// A `BumpAllocator` has laxer safety conditions when using `Allocator` methods:
/// - You can call `grow*`, `shrink` and `deallocate` with pointers that did not come from this allocator. In this case:
///   - `grow*` will always allocate a new memory block.
///   - `deallocate` will do nothing
///   - `shrink` will either do nothing or allocate iff the alignment increases
/// - Memory blocks can be split.
/// - `deallocate` can be called with any pointer or alignment when the size is `0`.
///
/// Examples:
/// - Handling of foreign pointers is necessary for implementing [`BumpVec::from_parts`] and [`BumpBox::into_box`].
/// - Memory block splitting is necessary for [`split_off`] and [`split_at`].
/// - Deallocate with a size of `0` is used in the drop implementation of [`BumpVec`].
///
/// # Safety
///
/// An implementor must support the conditions described above.
///
/// [`BumpVec::from_parts`]: crate::BumpVec::from_parts
/// [`BumpBox::into_box`]: crate::BumpBox::into_box
/// [`split_off`]: crate::BumpVec::split_off
/// [`split_at`]: crate::BumpBox::split_at
/// [`BumpVec`]: crate::BumpVec
// FIXME: SEAL ME
pub unsafe trait BumpAllocator: Allocator {
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    fn any_stats(&self) -> AnyStats<'_>;

    /// Creates a checkpoint of the current bump position.
    ///
    /// The bump position can be reset to this checkpoint with [`reset_to`].
    ///
    /// [`reset_to`]: BumpAllocator::reset_to
    fn checkpoint(&self) -> Checkpoint;

    /// Resets the bump position to a previously created checkpoint.
    /// The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    ///
    /// - the checkpoint must have been created by this bump allocator
    /// - the bump allocator must not have been [`reset`] since creation of this checkpoint
    /// - there must be no references to allocations made since creation of this checkpoint
    /// - the checkpoint must not have been created by an`!GUARANTEED_ALLOCATED` when self is `GUARANTEED_ALLOCATED`
    ///
    /// [`reset`]: crate::Bump::reset
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate alloc;
    /// # use bump_scope::{Bump, BumpAllocator, alloc::Global};
    /// # use alloc::alloc::Layout;
    /// fn test(bump: impl BumpAllocator) {
    ///     let checkpoint = bump.checkpoint();
    ///     
    ///     {
    ///         let hello = bump.allocate(Layout::new::<[u8;5]>()).unwrap();
    ///         assert_eq!(bump.any_stats().allocated(), 5);
    ///         # _ = hello;
    ///     }
    ///     
    ///     unsafe { bump.reset_to(checkpoint); }
    ///     assert_eq!(bump.any_stats().allocated(), 0);
    /// }
    ///
    /// test(<Bump>::new());
    /// test(Bump::<Global, 1, true, false>::unallocated());
    /// ```
    unsafe fn reset_to(&self, checkpoint: Checkpoint);

    /// Returns a pointer range of free space in the bump allocator with a size of at least `layout.size()`.
    ///
    /// Both the start and the end of the range is aligned to `layout.align()`.
    ///
    /// The pointer range takes up as much of the free space of the chunk as possible while satisfying the other conditions.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError>;

    /// Allocate part of the free space returned from a [`prepare_allocation`] call.
    ///
    /// # Safety
    /// - `range` must have been returned by a call to [`prepare_allocation`]
    /// - no allocations or resets must have been done since that call
    ///
    /// [`prepare_allocation`]: BumpAllocator::prepare_allocation
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8>;

    /// Allocate part of the free space returned from a [`prepare_allocation`] call starting at the end.
    ///
    /// # Safety
    /// - `range` must have been returned by a call to [`prepare_allocation`]
    /// - no allocations or resets must have been done since that call
    ///
    /// [`prepare_allocation`]: BumpAllocator::prepare_allocation
    /// [`allocate_prepared`]: BumpAllocator::allocate_prepared
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8>;
}

assert_dyn_compatible!(BumpAllocator);

assert_implements! {
    [BumpAllocator + ?Sized]

    Bump
    &Bump
    &mut Bump

    BumpScope
    &BumpScope
    &mut BumpScope

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
    // TODO:
    // &mut dyn MutBumpAllocatorScope
}

unsafe impl Allocator for &mut (dyn BumpAllocator + '_) {
    #[inline(always)]
    fn allocate(&self, layout: core::alloc::Layout) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).allocate(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: core::alloc::Layout) {
        (**self).deallocate(ptr, layout);
    }

    #[inline(always)]
    fn allocate_zeroed(&self, layout: core::alloc::Layout) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).allocate_zeroed(layout)
    }

    #[inline(always)]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).grow(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).grow_zeroed(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: core::alloc::Layout,
        new_layout: core::alloc::Layout,
    ) -> Result<NonNull<[u8]>, crate::alloc::AllocError> {
        (**self).shrink(ptr, old_layout, new_layout)
    }

    #[inline(always)]
    fn by_ref(&self) -> &Self
    where
        Self: Sized,
    {
        self
    }
}

unsafe impl<B: BumpAllocator + ?Sized> BumpAllocator for &B {
    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        B::any_stats(self)
    }

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        B::checkpoint(self)
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        B::reset_to(self, checkpoint);
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        B::prepare_allocation(self, layout)
    }

    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        B::allocate_prepared(self, layout, range)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        B::allocate_prepared_rev(self, layout, range)
    }
}

unsafe impl<B: BumpAllocator + ?Sized> BumpAllocator for &mut B
where
    for<'b> &'b mut B: Allocator,
{
    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        B::any_stats(self)
    }

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        B::checkpoint(self)
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        B::reset_to(self, checkpoint);
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        B::prepare_allocation(self, layout)
    }

    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        B::allocate_prepared(self, layout, range)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        B::allocate_prepared_rev(self, layout, range)
    }
}

unsafe impl<B: BumpAllocator> BumpAllocator for WithoutDealloc<B> {
    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        B::any_stats(&self.0)
    }

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        B::checkpoint(&self.0)
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        B::reset_to(&self.0, checkpoint);
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        B::prepare_allocation(&self.0, layout)
    }

    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        B::allocate_prepared(&self.0, layout, range)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        B::allocate_prepared_rev(&self.0, layout, range)
    }
}

unsafe impl<B: BumpAllocator> BumpAllocator for WithoutShrink<B> {
    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        B::any_stats(&self.0)
    }

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        B::checkpoint(&self.0)
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        B::reset_to(&self.0, checkpoint);
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        B::prepare_allocation(&self.0, layout)
    }

    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        B::allocate_prepared(&self.0, layout, range)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        B::allocate_prepared_rev(&self.0, layout, range)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        self.as_scope().any_stats()
    }

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        self.as_scope().checkpoint()
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        self.as_scope().reset_to(checkpoint);
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        self.as_scope().prepare_allocation(layout)
    }

    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        self.as_scope().allocate_prepared(layout, range)
    }

    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        self.as_scope().allocate_prepared_rev(layout, range)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        self.stats().into()
    }

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk.get())
    }

    // FIXME: make `Bump(Scope)`'s `reset_to` work this way too?
    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        if !GUARANTEED_ALLOCATED && checkpoint.chunk == unallocated_chunk_header() {
            let mut chunk = self.chunk.get();

            while let Some(prev) = chunk.prev() {
                chunk = prev;
            }

            chunk.reset();
            self.chunk.set(chunk);
        } else {
            debug_assert!(self.stats().big_to_small().any(|chunk| {
                chunk.header == checkpoint.chunk.cast() && chunk.contains_addr_or_end(checkpoint.address.get())
            }));

            checkpoint.reset_within_chunk();
            let chunk = RawChunk::from_header(checkpoint.chunk.cast());
            self.chunk.set(chunk);
        }
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        #[cold]
        #[inline(never)]
        unsafe fn prepare_allocation_in_another_chunk<
            A,
            const MIN_ALIGN: usize,
            const UP: bool,
            const GUARANTEED_ALLOCATED: bool,
        >(
            this: &BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
            layout: Layout,
        ) -> Result<Range<NonNull<u8>>, AllocError>
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
            this.in_another_chunk(CustomLayout(layout), |chunk, layout| {
                chunk.prepare_allocation_range(MinimumAlignment::<MIN_ALIGN>, layout)
            })
        }

        match self
            .chunk
            .get()
            .prepare_allocation_range(MinimumAlignment::<MIN_ALIGN>, CustomLayout(layout))
        {
            Some(ptr) => Ok(ptr),
            None => unsafe { prepare_allocation_in_another_chunk(self, layout) },
        }
    }

    // TODO: test
    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        debug_assert_eq!(
            (non_null::addr(range.end).get() - non_null::addr(range.start).get()) % layout.align(),
            0
        );
        debug_assert_eq!(layout.size() % layout.align(), 0);

        if UP {
            let end = non_null::add(range.start, layout.size());
            self.set_pos2(non_null::addr(end));
            range.start
        } else {
            let src = range.start;
            let dst_end = range.end;
            let dst = non_null::sub(dst_end, layout.size());
            non_null::copy(src, dst, layout.size());
            self.set_pos2(non_null::addr(dst));
            dst
        }
    }

    // TODO: test
    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        debug_assert_eq!(
            (non_null::addr(range.end).get() - non_null::addr(range.start).get()) % layout.align(),
            0
        );
        debug_assert_eq!(layout.size() % layout.align(), 0);

        if UP {
            let dst = range.start;
            let dst_end = non_null::add(dst, layout.size());

            let src_end = range.end;
            let src = non_null::sub(src_end, layout.size());

            non_null::copy(src, dst, layout.size());

            self.set_pos2(non_null::addr(dst_end));

            dst
        } else {
            let dst_end = range.end;
            let dst = non_null::sub(dst_end, layout.size());
            self.set_pos2(non_null::addr(dst));
            dst
        }
    }
}
