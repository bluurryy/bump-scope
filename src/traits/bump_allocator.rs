use core::{alloc::Layout, mem, ops::Range, ptr::NonNull};

use crate::{
    alloc::{AllocError, Allocator},
    chunk_header::ChunkHeader,
    layout::CustomLayout,
    polyfill::{self, non_null},
    traits::{assert_dyn_compatible, assert_implements},
    BaseAllocator, Bump, BumpAllocatorChunks, BumpAllocatorScope, BumpScope, MinimumAlignment, MutBumpAllocator,
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
    /// Returns a type that can be used to [create checkpoints], [reset to them],
    /// [get an `AnyStats` object] and check if the bump allocator [has an allocated chunk].
    ///
    /// [create checkpoints]: BumpAllocatorChunks::checkpoint
    /// [reset to them]: BumpAllocatorChunks::reset_to
    /// [get an `AnyStats` object]: BumpAllocatorChunks::stats
    /// [has an allocated chunk]: BumpAllocatorChunks::is_allocated
    fn chunks(&self) -> &BumpAllocatorChunks;

    /// Returns the size of the chunk header. This value is needed to create an `AnyStats` object via
    /// <code>self.[chunks]\().[stats]\(self.[chunk_header_size]\())</code>.
    ///
    /// [chunks]: BumpAllocator::chunks
    /// [stats]: BumpAllocatorChunks::stats
    /// [chunk_header_size]: BumpAllocator::chunk_header_size
    fn chunk_header_size(&self) -> usize;

    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError>;

    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8>;
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
    fn chunks(&self) -> &BumpAllocatorChunks {
        B::chunks(self)
    }

    #[inline(always)]
    fn chunk_header_size(&self) -> usize {
        B::chunk_header_size(self)
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
    fn chunks(&self) -> &BumpAllocatorChunks {
        B::chunks(self)
    }

    #[inline(always)]
    fn chunk_header_size(&self) -> usize {
        B::chunk_header_size(self)
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
    fn chunks(&self) -> &BumpAllocatorChunks {
        B::chunks(&self.0)
    }

    #[inline(always)]
    fn chunk_header_size(&self) -> usize {
        B::chunk_header_size(&self.0)
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
    fn chunks(&self) -> &BumpAllocatorChunks {
        B::chunks(&self.0)
    }

    #[inline(always)]
    fn chunk_header_size(&self) -> usize {
        B::chunk_header_size(&self.0)
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
    fn chunks(&self) -> &BumpAllocatorChunks {
        unsafe { polyfill::transmute_ref(&self.chunk) }
    }

    #[inline(always)]
    fn chunk_header_size(&self) -> usize {
        mem::size_of::<ChunkHeader<A>>()
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
    fn chunks(&self) -> &BumpAllocatorChunks {
        unsafe { polyfill::transmute_ref(&self.chunk) }
    }

    #[inline(always)]
    fn chunk_header_size(&self) -> usize {
        mem::size_of::<ChunkHeader<A>>()
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
