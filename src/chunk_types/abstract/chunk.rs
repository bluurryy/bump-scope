use core::{cell::Cell, marker::PhantomData, mem::transmute, ops::Range, ptr::NonNull};

use crate::{
    bumping::{BumpProps, BumpUp, bump_down, bump_prepare_down, bump_prepare_up, bump_up},
    chunk_types::{
        r#abstract::{FirstAllocatedChunk, allocated_chunk::AllocatedChunk},
        concrete::{ClaimedChunk, UnallocatedChunk},
        dummy_memory_region,
    },
    layout::LayoutProps,
    settings::BumpAllocatorSettings,
};

#[repr(C, align(16))]
struct Header {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,
}

#[repr(transparent)]
pub(crate) struct Chunk<A, S> {
    header: NonNull<Header>,
    #[expect(clippy::type_complexity)]
    marker: PhantomData<(*const A, fn() -> S)>,
}

impl<A, S> Clone for Chunk<A, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for Chunk<A, S> {}

impl<A, S> Chunk<A, S> {
    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.header.cast()
    }

    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.pos.get()
    }

    pub(crate) unsafe fn from_ptr(header: NonNull<()>) -> Self {
        Self {
            header: header.cast(),
            marker: PhantomData,
        }
    }

    pub const fn from_unallocated(chunk: UnallocatedChunk<A, S>) -> Self {
        unsafe { transmute(chunk) }
    }

    pub const fn from_allocated(chunk: AllocatedChunk<A, S>) -> Self {
        unsafe { transmute(chunk) }
    }

    pub const fn from_claimed(chunk: ClaimedChunk<A, S>) -> Self {
        unsafe { transmute(chunk) }
    }

    pub(crate) fn from_first(zst: FirstAllocatedChunk<A, S>) -> Self {
        unsafe { transmute(zst) }
    }
}

impl<A, S> Chunk<A, S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) fn is_unallocated(self) -> bool {
        !S::GUARANTEED_ALLOCATED && self.pos() == dummy_memory_region::UNALLOCATED_POS
    }

    pub(crate) fn is_claimed(self) -> bool {
        S::CLAIMABLE && self.pos() == dummy_memory_region::CLAIMED_POS
    }

    pub(crate) fn kind(self) -> ChunkKind<A, S> {
        unsafe {
            if !S::GUARANTEED_ALLOCATED && self.is_unallocated() {
                ChunkKind::Unallocated(transmute(self))
            } else if S::CLAIMABLE && self.is_claimed() {
                ChunkKind::Claimed(transmute(self))
            } else {
                ChunkKind::Allocated(transmute(self))
            }
        }
    }

    pub(crate) fn as_allocated(self) -> Option<AllocatedChunk<A, S>> {
        match self.kind() {
            ChunkKind::Claimed(_) | ChunkKind::Unallocated(_) => None,
            ChunkKind::Allocated(chunk) => Some(chunk),
        }
    }

    pub(crate) unsafe fn as_allocated_unchecked(self) -> AllocatedChunk<A, S> {
        debug_assert!(matches!(self.kind(), ChunkKind::Allocated(_)));
        unsafe { transmute(self) }
    }

    /// Attempts to allocate a block of memory.
    ///
    /// On success, returns a [`NonNull<u8>`] meeting the size and alignment guarantees of `layout`.
    #[inline(always)]
    pub(crate) fn alloc(self, layout: impl LayoutProps) -> Option<NonNull<u8>> {
        let props = self.bump_props(layout);

        let ptr = if S::UP {
            let BumpUp { new_pos, ptr } = bump_up(props)?;

            // SAFETY: allocations never succeed for a dummy chunk
            unsafe {
                let chunk = self.as_allocated_unchecked();
                chunk.set_pos_addr(new_pos);
                chunk.content_ptr_from_addr(ptr)
            }
        } else {
            let ptr = bump_down(props)?;

            // SAFETY: allocations never succeed for a dummy chunk
            unsafe {
                let chunk = self.as_allocated_unchecked();
                chunk.set_pos_addr(ptr);
                chunk.content_ptr_from_addr(ptr)
            }
        };

        miri_promise_symbolic_alignment(ptr.as_ptr().cast(), layout.align());

        Some(ptr)
    }

    /// Prepares allocation for a block of memory.
    ///
    /// On success, returns a [`NonNull<u8>`] meeting the size and alignment guarantees of `layout`.
    ///
    /// This is like [`alloc`](Self::alloc), except that it won't change the bump pointer.
    #[inline(always)]
    pub(crate) fn prepare_allocation(self, layout: impl LayoutProps) -> Option<NonNull<u8>> {
        let props = self.bump_props(layout);

        let ptr = if S::UP { bump_up(props)?.ptr } else { bump_down(props)? };

        // SAFETY: allocations never succeed for a dummy chunk
        let ptr = unsafe {
            let chunk = self.as_allocated_unchecked();
            chunk.content_ptr_from_addr(ptr)
        };

        miri_promise_symbolic_alignment(ptr.as_ptr().cast(), layout.align());

        Some(ptr)
    }

    /// Returns the rest of the capacity of the chunk.
    /// This does not change the position within the chunk.
    ///
    /// This is used in [`MutBumpVec`] where we mutably burrow bump access.
    /// In this case we do not want to update the bump pointer. This way
    /// neither reallocations (a new chunk) nor dropping needs to move the bump pointer.
    /// The bump pointer is only updated when we call [`into_slice`].
    ///
    /// - `range.start` and `range.end` are aligned.
    /// - `layout.size` must not be zero
    /// - `layout.size` must be a multiple of `layout.align`
    ///
    /// [`MutBumpVec`]: crate::MutBumpVec
    /// [`into_slice`]: crate::MutBumpVec::into_slice
    #[inline(always)]
    pub(crate) fn prepare_allocation_range(self, layout: impl LayoutProps) -> Option<Range<NonNull<u8>>> {
        let props = self.bump_props(layout);

        let range = if S::UP {
            bump_prepare_up(props)
        } else {
            bump_prepare_down(props)
        }?;

        // SAFETY: allocations never succeed for a dummy chunk
        let ptrs = unsafe {
            let chunk = self.as_allocated_unchecked();
            chunk.content_ptr_from_addr_range(range)
        };

        miri_promise_symbolic_alignment(ptrs.start.as_ptr().cast(), layout.align());
        miri_promise_symbolic_alignment(ptrs.end.as_ptr().cast(), layout.align());

        Some(ptrs)
    }

    #[inline(always)]
    fn bump_props<L>(self, layout: L) -> BumpProps
    where
        L: LayoutProps,
    {
        let pos = self.pos().addr().get();
        let end = unsafe { self.header.as_ref() }.end.addr().get();

        let start = if S::UP { pos } else { end };
        let end = if S::UP { end } else { pos };

        #[cfg(debug_assertions)]
        if !matches!(self.kind(), ChunkKind::Allocated(_)) {
            assert!(start > end);
        }

        BumpProps {
            start,
            end,
            layout: *layout,
            min_align: S::MIN_ALIGN,
            align_is_const: L::ALIGN_IS_CONST,
            size_is_const: L::SIZE_IS_CONST,
            size_is_multiple_of_align: L::SIZE_IS_MULTIPLE_OF_ALIGN,
        }
    }
}

pub(crate) enum ChunkKind<A, S> {
    Claimed(#[expect(dead_code)] ClaimedChunk<A, S>),
    Unallocated(UnallocatedChunk<A, S>),
    Allocated(AllocatedChunk<A, S>),
}

/// Miri doesn't understand how our bit twiddling aligns the pointer but we can promise it with this.
/// See <https://github.com/rust-lang/rust/pull/117840>
#[inline(always)]
fn miri_promise_symbolic_alignment(ptr: *const (), align: usize) {
    #[cfg(all(feature = "nightly-miri-promise-symbolic-alignment", miri))]
    unsafe extern "Rust" {
        /// Miri-provided extern function to promise that a given pointer is properly aligned for
        /// "symbolic" alignment checks. Will fail if the pointer is not actually aligned or `align` is
        /// not a power of two. Has no effect when alignment checks are concrete (which is the default).
        unsafe fn miri_promise_symbolic_alignment(ptr: *const (), align: usize);
    }

    #[cfg(all(feature = "nightly-miri-promise-symbolic-alignment", miri))]
    unsafe {
        miri_promise_symbolic_alignment(ptr, align);
    }

    #[cfg(not(all(feature = "nightly-miri-promise-symbolic-alignment", miri)))]
    {
        _ = (ptr, align);
    }
}
