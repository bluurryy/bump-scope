use core::{marker::PhantomData, ops::Range, ptr::NonNull};

use crate::{
    bumping::{BumpProps, BumpUp, bump_down, bump_prepare_down, bump_prepare_up, bump_up},
    chunk::{
        AllocatedChunk, ChunkClass, ChunkHeader, DummyChunkHeader, UnallocatedChunk, UnallocatedChunkHeader,
        erased_chunk::ErasedChunk,
    },
    layout::LayoutProps,
    settings::BumpAllocatorSettings,
};

/// Represents a chunk of the bump allocator.
///
/// This type behaves somewhat like a `ManuallyDrop<T>` in the sense that it has safe to
/// use methods that assume the chunk is still live.
#[repr(transparent)]
pub(crate) struct Chunk<A, S> {
    header: NonNull<ChunkHeader>,
    marker: PhantomData<fn() -> (A, S)>,
}

impl<A, S> Clone for Chunk<A, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Copy for Chunk<A, S> {}

impl<A, S> Chunk<A, S>
where
    A: Default + 'static,
    S: BumpAllocatorSettings,
{
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) const UNALLOCATED_UNCHECKED: Self = {
        Self {
            header: if S::UP {
                UnallocatedChunkHeader::<A>::UP
            } else {
                UnallocatedChunkHeader::<A>::DOWN
            }
            .cast(),
            marker: PhantomData,
        }
    };

    /// This is only valid when `!S::GUARANTEED_ALLOCATED`.
    pub(crate) const UNALLOCATED: Self = {
        assert!(!S::GUARANTEED_ALLOCATED);

        Self {
            header: if S::UP {
                UnallocatedChunkHeader::<A>::UP
            } else {
                UnallocatedChunkHeader::<A>::DOWN
            }
            .cast(),
            marker: PhantomData,
        }
    };
}

impl<A, S> Chunk<A, S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) const CLAIMED: Self = {
        assert!(S::CLAIMABLE);

        Self {
            header: if S::UP {
                DummyChunkHeader::CLAIMED_UP
            } else {
                DummyChunkHeader::CLAIMED_DOWN
            }
            .cast(),

            marker: PhantomData,
        }
    };

    pub(crate) unsafe fn from_raw(header: NonNull<ChunkHeader>) -> Self {
        Self {
            header,
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) fn erase(self) -> ErasedChunk {
        unsafe { ErasedChunk::from_raw(self.header.cast()) }
    }

    #[inline(always)]
    pub(crate) fn header(self) -> NonNull<ChunkHeader> {
        self.header
    }

    #[inline(always)]
    pub(crate) fn is_claimed(self) -> bool {
        S::CLAIMABLE && unsafe { self.header.as_ref().is_claimed() }
    }

    #[inline(always)]
    pub(crate) fn is_unallocated(self) -> bool {
        !S::GUARANTEED_ALLOCATED && unsafe { self.header.as_ref().is_unallocated() }
    }

    #[inline(always)]
    pub(crate) fn classify(self) -> ChunkClass<A, S> {
        if self.is_claimed() {
            return ChunkClass::Claimed;
        }

        if self.is_unallocated() {
            return ChunkClass::Unallocated(UnallocatedChunk {
                header: self.header.cast(),
                marker: self.marker,
            });
        }

        // SAFETY: we checked that the chunk is not a dummy chunk; now it can only be an allocated chunk
        ChunkClass::Allocated(unsafe { AllocatedChunk::from_raw(self.header.cast()) })
    }

    #[inline(always)]
    pub(crate) fn as_non_dummy(self) -> Option<AllocatedChunk<A, S>> {
        match self.classify() {
            ChunkClass::Claimed | ChunkClass::Unallocated(_) => None,
            ChunkClass::Allocated(chunk) => Some(chunk),
        }
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
        if !matches!(self.classify(), ChunkClass::Allocated(_)) {
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

    #[inline(always)]
    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref().pos.get() }
    }

    #[inline(always)]
    pub(crate) unsafe fn as_allocated_unchecked(self) -> AllocatedChunk<A, S> {
        debug_assert!(matches!(self.classify(), ChunkClass::Allocated(_)));
        unsafe { AllocatedChunk::from_raw(self.header.cast()) }
    }
}

/// Miri doesn't understand how our bit twiddling aligns the pointer but we can promise it with this.
/// See <https://github.com/rust-lang/rust/pull/117840>
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
