use core::{alloc::Layout, ops::Range, ptr::NonNull};

use crate::{
    BaseAllocator, Bump, BumpScope, Checkpoint, WithoutDealloc, WithoutShrink,
    alloc::{AllocError, Allocator},
    chunk::{ChunkHeader, RawChunk},
    layout::CustomLayout,
    settings::BumpAllocatorSettings,
    stats::AnyStats,
    traits::{assert_dyn_compatible, assert_implements},
};

pub trait Sealed {}

impl<B: Sealed + ?Sized> Sealed for &B {}
impl<B: Sealed + ?Sized> Sealed for &mut B {}
impl<B: Sealed> Sealed for WithoutDealloc<B> {}
impl<B: Sealed> Sealed for WithoutShrink<B> {}

impl<A, S> Sealed for Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
}

impl<A, S> Sealed for BumpScope<'_, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
}

/// A bump allocator.
///
/// This trait provides additional methods and guarantees on top of an [`Allocator`].
///
/// A `BumpAllocatorCore` has laxer safety conditions when using `Allocator` methods:
/// - You can call `grow*`, `shrink` and `deallocate` with pointers that did not come from this allocator. In this case:
///   - `grow*` will always allocate a new memory block.
///   - `deallocate` will do nothing
///   - `shrink` will either do nothing or allocate iff the alignment increases
/// - Memory blocks can be split.
/// - `deallocate` can be called with any pointer or alignment when the size is `0`.
/// - `shrink` never errors unless the new alignment is greater
///
/// Those invariants are used here:
/// - Handling of foreign pointers is necessary for implementing [`BumpVec::from_parts`], [`BumpBox::into_box`] and [`Bump(Scope)::dealloc`][Bump::dealloc].
/// - Memory block splitting is necessary for [`split_off`] and [`split_at`].
/// - Deallocate with a size of `0` is used in the drop implementation of [`BumpVec`].
/// - The non-erroring behavior of `shrink` is necessary for [`BumpAllocatorTyped::shrink_slice`]
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
/// [`BumpAllocatorTyped::shrink_slice`]: crate::traits::BumpAllocatorTyped::shrink_slice
pub unsafe trait BumpAllocatorCore: Allocator + Sealed {
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    fn any_stats(&self) -> AnyStats<'_>;

    /// Creates a checkpoint of the current bump position.
    ///
    /// The bump position can be reset to this checkpoint with [`reset_to`].
    ///
    /// [`reset_to`]: BumpAllocatorCore::reset_to
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
    /// - the bump allocator must be [unclaimed] at the time the checkpoint is created and when this function is called
    ///
    /// [`reset`]: crate::Bump::reset
    /// [unclaimed]: crate::traits::BumpAllocatorScope::claim
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate alloc;
    /// use bump_scope::{
    ///     Bump,
    ///     alloc::Global,
    ///     settings::{BumpSettings, BumpAllocatorSettings},
    ///     traits::BumpAllocatorCore,  
    /// };
    /// # use alloc::alloc::Layout;
    ///
    /// fn test(bump: impl BumpAllocatorCore) {
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
    /// test(Bump::<Global, <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>>::unallocated());
    /// ```
    unsafe fn reset_to(&self, checkpoint: Checkpoint);

    /// Returns true if the bump allocator is currently [claimed].
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    fn is_claimed(&self) -> bool;

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
    /// - `range` must have been returned from a call to [`prepare_allocation`]
    /// - no allocation, grow, shrink or deallocate must have taken place since then
    /// - no resets must have taken place since then
    /// - `layout` must be less than or equal to the `layout` used when calling
    ///   [`prepare_allocation`], both in size and alignment
    /// - the bump allocator must be [unclaimed] at the time [`prepare_allocation`] was called and when calling this function
    ///
    /// [`prepare_allocation`]: BumpAllocatorCore::prepare_allocation
    /// [unclaimed]: crate::traits::BumpAllocatorScope::claim
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8>;

    /// Allocate part of the free space returned from a [`prepare_allocation`] call starting at the end.
    ///
    /// # Safety
    /// - `range` must have been returned from a call to [`prepare_allocation`]
    /// - no allocation, grow, shrink or deallocate must have taken place since then
    /// - no resets must have taken place since then
    /// - `layout` must be less than or equal to the `layout` used when calling
    ///   [`prepare_allocation`], both in size and alignment
    /// - the bump allocator must be [unclaimed] at the time [`prepare_allocation`] was called and when calling this function
    ///
    /// [`prepare_allocation`]: BumpAllocatorCore::prepare_allocation
    /// [`allocate_prepared`]: BumpAllocatorCore::allocate_prepared
    /// [unclaimed]: crate::traits::BumpAllocatorScope::claim
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8>;
}

assert_dyn_compatible!(BumpAllocatorCore);

assert_implements! {
    [BumpAllocatorCore + ?Sized]

    Bump
    &Bump
    &mut Bump

    BumpScope
    &BumpScope
    &mut BumpScope

    dyn BumpAllocatorCore
    &dyn BumpAllocatorCore
    &mut dyn BumpAllocatorCore

    dyn BumpAllocatorCoreScope
    &dyn BumpAllocatorCoreScope
    &mut dyn BumpAllocatorCoreScope

    dyn MutBumpAllocatorCore
    &dyn MutBumpAllocatorCore
    &mut dyn MutBumpAllocatorCore

    dyn MutBumpAllocatorCoreScope
    &dyn MutBumpAllocatorCoreScope
    &mut dyn MutBumpAllocatorCoreScope
}

macro_rules! impl_for_ref {
    ($($ty:ty)*) => {
        $(
            unsafe impl<B: BumpAllocatorCore + ?Sized> BumpAllocatorCore for $ty {
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
                    unsafe { B::reset_to(self, checkpoint) };
                }

                #[inline(always)]
                fn is_claimed(&self) -> bool {
                    B::is_claimed(self)
                }

                #[inline(always)]
                fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
                    B::prepare_allocation(self, layout)
                }

                #[inline(always)]
                unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
                    unsafe { B::allocate_prepared(self, layout, range) }
                }

                #[inline(always)]
                unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
                    unsafe { B::allocate_prepared_rev(self, layout, range) }
                }
            }
        )*
    };
}

impl_for_ref! {
    &B
    &mut B
}

unsafe impl<B: BumpAllocatorCore> BumpAllocatorCore for WithoutDealloc<B> {
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
        unsafe { B::reset_to(&self.0, checkpoint) };
    }

    #[inline(always)]
    fn is_claimed(&self) -> bool {
        B::is_claimed(&self.0)
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        B::prepare_allocation(&self.0, layout)
    }

    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        unsafe { B::allocate_prepared(&self.0, layout, range) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        unsafe { B::allocate_prepared_rev(&self.0, layout, range) }
    }
}

unsafe impl<B: BumpAllocatorCore> BumpAllocatorCore for WithoutShrink<B> {
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
        unsafe { B::reset_to(&self.0, checkpoint) };
    }

    #[inline(always)]
    fn is_claimed(&self) -> bool {
        B::is_claimed(&self.0)
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        B::prepare_allocation(&self.0, layout)
    }

    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        unsafe { B::allocate_prepared(&self.0, layout, range) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        unsafe { B::allocate_prepared_rev(&self.0, layout, range) }
    }
}

unsafe impl<A, S> BumpAllocatorCore for Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
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
        unsafe { self.as_scope().reset_to(checkpoint) };
    }

    #[inline(always)]
    fn is_claimed(&self) -> bool {
        self.as_scope().is_claimed()
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        self.as_scope().prepare_allocation(layout)
    }

    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        unsafe { self.as_scope().allocate_prepared(layout, range) }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        unsafe { self.as_scope().allocate_prepared_rev(layout, range) }
    }
}

unsafe impl<A, S> BumpAllocatorCore for BumpScope<'_, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        self.stats().into()
    }

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk.get())
    }

    #[inline]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        debug_assert!(!self.is_claimed());

        // If the checkpoint was created when the bump allocator had no allocated chunk
        // then the chunk pointer will point to the unallocated chunk header.
        //
        // In such cases we reset the bump pointer to the very start of the very first chunk.
        //
        // We don't check if the chunk pointer points to the unallocated chunk header
        // if the bump allocator is `GUARANTEED_ALLOCATED`. We are allowed to not do this check
        // because of this safety condition of `reset_to`:
        // > the checkpoint must not have been created by an`!GUARANTEED_ALLOCATED` when self is `GUARANTEED_ALLOCATED`
        if !S::GUARANTEED_ALLOCATED && checkpoint.chunk == ChunkHeader::unallocated::<S>() {
            if let Some(mut chunk) = self.chunk.get().guaranteed_allocated() {
                while let Some(prev) = chunk.prev() {
                    chunk = prev;
                }

                chunk.reset();

                // SAFETY: casting from guaranteed-allocated to non-guaranteed-allocated is safe
                self.chunk.set(unsafe { chunk.cast() });
            }
        } else {
            debug_assert_ne!(
                checkpoint.chunk,
                ChunkHeader::unallocated::<S>(),
                "the safety conditions state that \"the checkpoint must not have been created by an`!GUARANTEED_ALLOCATED` when self is `GUARANTEED_ALLOCATED`\""
            );

            #[cfg(debug_assertions)]
            {
                let chunk = self
                    .stats()
                    .small_to_big()
                    .find(|chunk| chunk.header() == checkpoint.chunk.cast())
                    .expect("this checkpoint does not refer to any chunk of this bump allocator");

                assert!(
                    chunk.contains_addr_or_end(checkpoint.address.get()),
                    "checkpoint address does not point within its chunk"
                );
            }

            unsafe {
                checkpoint.reset_within_chunk();
                let chunk = RawChunk::from_header(checkpoint.chunk.cast());
                self.chunk.set(chunk);
            }
        }
    }

    #[inline(always)]
    fn is_claimed(&self) -> bool {
        self.chunk.get().is_claimed()
    }

    #[inline(always)]
    fn prepare_allocation(&self, layout: Layout) -> Result<Range<NonNull<u8>>, AllocError> {
        #[cold]
        #[inline(never)]
        unsafe fn prepare_allocation_in_another_chunk<A, S>(
            this: &BumpScope<'_, A, S>,
            layout: Layout,
        ) -> Result<Range<NonNull<u8>>, AllocError>
        where
            A: BaseAllocator<S::GuaranteedAllocated>,
            S: BumpAllocatorSettings,
        {
            unsafe { this.in_another_chunk(CustomLayout(layout), RawChunk::prepare_allocation_range) }
        }

        match self.chunk.get().prepare_allocation_range(CustomLayout(layout)) {
            Some(ptr) => Ok(ptr),
            None => unsafe { prepare_allocation_in_another_chunk(self, layout) },
        }
    }

    #[inline(always)]
    unsafe fn allocate_prepared(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        debug_assert_eq!(range.start.addr().get() % layout.align(), 0);
        debug_assert_eq!(range.end.addr().get() % layout.align(), 0);
        debug_assert_eq!(layout.size() % layout.align(), 0);

        unsafe {
            if S::UP {
                let end = range.start.add(layout.size());
                self.set_pos(end.addr());
                range.start
            } else {
                let src = range.start;
                let dst_end = range.end;
                let dst = dst_end.sub(layout.size());
                src.copy_to(dst, layout.size());
                self.set_pos(dst.addr());
                dst
            }
        }
    }

    #[inline(always)]
    unsafe fn allocate_prepared_rev(&self, layout: Layout, range: Range<NonNull<u8>>) -> NonNull<u8> {
        debug_assert_eq!(range.start.addr().get() % layout.align(), 0);
        debug_assert_eq!(range.end.addr().get() % layout.align(), 0);
        debug_assert_eq!(layout.size() % layout.align(), 0);

        unsafe {
            if S::UP {
                let dst = range.start;
                let dst_end = dst.add(layout.size());

                let src_end = range.end;
                let src = src_end.sub(layout.size());

                src.copy_to(dst, layout.size());

                self.set_pos(dst_end.addr());

                dst
            } else {
                let dst_end = range.end;
                let dst = dst_end.sub(layout.size());
                self.set_pos(dst.addr());
                dst
            }
        }
    }
}
