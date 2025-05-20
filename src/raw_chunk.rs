use core::{alloc::Layout, cell::Cell, mem::align_of, num::NonZeroUsize, ops::Range, ptr::NonNull};

use crate::{
    alloc::{AllocError, Allocator},
    bumping::{bump_down, bump_prepare_down, bump_prepare_up, bump_up, BumpProps, BumpUp, MIN_CHUNK_ALIGN},
    chunk_size::{ChunkSize, ChunkSizeHint},
    down_align_usize,
    layout::{ArrayLayout, LayoutProps},
    polyfill::{nonnull, pointer},
    unallocated_chunk_header, up_align_usize_unchecked, ChunkHeader, ErrorBehavior, MinimumAlignment,
    SupportedMinimumAlignment,
};

/// Represents an allocated chunk.
///
/// This type behaves somewhat like a `ManuallyDrop<T>` in the sense that it has safe to
/// use methods that assume the chunk has not been deallocated.
///
/// So just the `deallocate` method is unsafe. You have to make sure the chunk is not used
/// after calling that.
#[repr(transparent)]
pub(crate) struct RawChunk<const UP: bool, A> {
    /// This points to a valid [`ChunkHeader`].
    header: NonNull<ChunkHeader<A>>,
}

impl<const UP: bool, A> Copy for RawChunk<UP, A> {}

#[allow(clippy::expl_impl_clone_on_copy)]
impl<const UP: bool, A> Clone for RawChunk<UP, A> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<const UP: bool, A> PartialEq for RawChunk<UP, A> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
    }

    #[inline(always)]
    fn ne(&self, other: &Self) -> bool {
        self.header != other.header
    }
}

impl<const UP: bool, A> Eq for RawChunk<UP, A> {}

impl<const UP: bool, A> RawChunk<UP, A> {
    pub(crate) fn new_in<E: ErrorBehavior>(chunk_size: ChunkSize<A, UP>, prev: Option<Self>, allocator: A) -> Result<Self, E>
    where
        A: Allocator,
        for<'a> &'a A: Allocator,
    {
        let layout = chunk_size.layout().ok_or_else(E::capacity_overflow)?;

        let allocation = match allocator.allocate(layout) {
            Ok(ok) => ok,
            Err(AllocError) => return Err(E::allocation(layout)),
        };

        let ptr = nonnull::as_non_null_ptr(allocation);
        let size = allocation.len();

        // Note that the allocation's size may be larger than
        // the requested layout's size.
        //
        // We could be ignoring the allocation's size and just use
        // our layout's size, but then we would be wasting
        // the extra space the allocator might have given us.
        //
        // This returned size does not satisfy our invariants though
        // so we need to align it first.
        //
        // Follow this method for details.
        let size = chunk_size.align_allocation_size(size);

        debug_assert!(size >= layout.size());
        debug_assert!(size % MIN_CHUNK_ALIGN == 0);

        let prev = prev.map(|c| c.header);
        let next = Cell::new(None);

        let header = unsafe {
            if UP {
                let header = ptr.cast::<ChunkHeader<A>>();

                header.as_ptr().write(ChunkHeader {
                    pos: Cell::new(nonnull::add(header, 1).cast()),
                    end: nonnull::add(ptr, size),
                    prev,
                    next,
                    allocator,
                });

                header
            } else {
                let header = nonnull::sub(nonnull::add(ptr, size).cast::<ChunkHeader<A>>(), 1);

                header.as_ptr().write(ChunkHeader {
                    pos: Cell::new(header.cast()),
                    end: ptr,
                    prev,
                    next,
                    allocator,
                });

                header
            }
        };

        Ok(RawChunk { header })
    }

    pub(crate) fn header_ptr(self) -> NonNull<ChunkHeader<A>> {
        self.header
    }

    pub(crate) const unsafe fn from_header(header: NonNull<ChunkHeader<A>>) -> Self {
        Self { header }
    }

    pub(crate) fn is_unallocated(self) -> bool {
        self.header.cast() == unallocated_chunk_header()
    }

    /// Attempts to allocate a block of memory.
    ///
    /// On success, returns a [`NonNull<u8>`] meeting the size and alignment guarantees of `layout`.
    #[inline(always)]
    pub(crate) fn alloc<M, L>(self, minimum_alignment: M, layout: L) -> Option<NonNull<u8>>
    where
        M: SupportedMinimumAlignment,
        L: LayoutProps,
    {
        self.alloc_or_else(minimum_alignment, layout, || Err(())).ok()
    }

    /// Attempts to allocate a block of memory.
    /// If there is not enough space, `f` will be called and its result returned.
    ///
    /// We use a callback for the fallback case for performance reasons.
    /// If we were to just expose an api like [`alloc`](Self::alloc) and matched over the `Option`, the compiler would
    /// introduce an unnecessary conditional jump in the infallible code path.
    ///
    /// `rustc` used to be able to optimize this away (see issue #25)
    #[inline(always)]
    pub(crate) fn alloc_or_else<M, L, E, F>(self, minimum_alignment: M, layout: L, f: F) -> Result<NonNull<u8>, E>
    where
        M: SupportedMinimumAlignment,
        L: LayoutProps,
        F: FnOnce() -> Result<NonNull<u8>, E>,
    {
        let props = self.bump_props(minimum_alignment, layout);

        unsafe {
            if UP {
                match bump_up(props) {
                    Some(BumpUp { new_pos, ptr }) => {
                        self.set_pos_addr(new_pos);
                        Ok(self.with_addr(ptr))
                    }
                    None => f(),
                }
            } else {
                match bump_down(props) {
                    Some(ptr) => {
                        let ptr = self.with_addr(ptr);
                        self.set_pos(ptr);
                        Ok(ptr)
                    }
                    None => f(),
                }
            }
        }
    }

    /// Attempts to reserve a block of memory.
    ///
    /// On success, returns a [`NonNull<u8>`] meeting the size and alignment guarantees of `layout`.
    ///
    /// This is like [`alloc`](Self::alloc_or_else), except that it won't change the bump pointer.
    #[inline(always)]
    pub(crate) fn prepare_allocation<M, L>(self, minimum_alignment: M, layout: L) -> Option<NonNull<u8>>
    where
        M: SupportedMinimumAlignment,
        L: LayoutProps,
    {
        self.prepare_allocation_or_else(minimum_alignment, layout, || Err(())).ok()
    }

    /// Attempts to reserve a block of memory.
    /// If there is not enough space, `f` will be called and its result returned.
    ///
    /// This is like [`alloc_or_else`](Self::alloc_or_else), except that it won't change the bump pointer.
    #[inline(always)]
    pub(crate) fn prepare_allocation_or_else<M, L, E, F>(
        self,
        minimum_alignment: M,
        layout: L,
        f: F,
    ) -> Result<NonNull<u8>, E>
    where
        M: SupportedMinimumAlignment,
        L: LayoutProps,
        F: FnOnce() -> Result<NonNull<u8>, E>,
    {
        let props = self.bump_props(minimum_alignment, layout);

        unsafe {
            if UP {
                match bump_up(props) {
                    Some(BumpUp { ptr, .. }) => Ok(self.with_addr(ptr)),
                    None => f(),
                }
            } else {
                match bump_down(props) {
                    Some(ptr) => Ok(self.with_addr(ptr)),
                    None => f(),
                }
            }
        }
    }

    #[inline(always)]
    pub(crate) fn bump_props<M, L>(self, _: M, layout: L) -> BumpProps
    where
        M: SupportedMinimumAlignment,
        L: LayoutProps,
    {
        debug_assert!(nonnull::is_aligned_to(self.pos(), M::MIN_ALIGN));
        let remaining = self.remaining_range();

        BumpProps {
            start: nonnull::addr(remaining.start).get(),
            end: nonnull::addr(remaining.end).get(),
            layout: *layout,
            min_align: M::MIN_ALIGN,
            align_is_const: L::ALIGN_IS_CONST,
            size_is_const: L::SIZE_IS_CONST,
            size_is_multiple_of_align: L::SIZE_IS_MULTIPLE_OF_ALIGN,
        }
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
    pub(crate) fn prepare_allocation_range<M>(self, minimum_alignment: M, layout: ArrayLayout) -> Option<Range<NonNull<u8>>>
    where
        M: SupportedMinimumAlignment,
    {
        debug_assert_ne!(layout.size(), 0);
        let props = self.bump_props(minimum_alignment, layout);

        unsafe {
            if UP {
                let range = bump_prepare_up(props)?;
                Some(self.with_addr_range(range))
            } else {
                let range = bump_prepare_down(props)?;
                Some(self.with_addr_range(range))
            }
        }
    }

    #[inline(always)]
    pub(crate) fn align_pos_to<const ALIGN: usize>(self)
    where
        MinimumAlignment<ALIGN>: SupportedMinimumAlignment,
    {
        let mut pos = nonnull::addr(self.pos()).get();

        if UP {
            // Aligning an address that is `<= range.end` with an alignment
            // that is `<= CHUNK_ALIGN_MIN` can not exceed `range.end` and
            // can not overflow as `range.end` is always aligned to `CHUNK_ALIGN_MIN`.
            pos = up_align_usize_unchecked(pos, ALIGN);
        } else {
            pos = down_align_usize(pos, ALIGN);
        }

        unsafe { self.set_pos_addr(pos) }
    }

    #[inline(always)]
    fn after_header(self) -> NonNull<u8> {
        unsafe { nonnull::add(self.header, 1).cast() }
    }

    #[inline(always)]
    pub(crate) fn chunk_start(self) -> NonNull<u8> {
        unsafe {
            if UP {
                self.header.cast()
            } else {
                self.header.as_ref().end
            }
        }
    }

    #[inline(always)]
    pub(crate) fn chunk_end(self) -> NonNull<u8> {
        unsafe {
            if UP {
                self.header.as_ref().end
            } else {
                self.after_header()
            }
        }
    }

    #[inline(always)]
    pub(crate) fn content_start(self) -> NonNull<u8> {
        if UP {
            self.after_header()
        } else {
            self.chunk_start()
        }
    }

    #[inline(always)]
    pub(crate) fn content_end(self) -> NonNull<u8> {
        if UP {
            self.chunk_end()
        } else {
            self.header.cast()
        }
    }

    #[inline(always)]
    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref().pos.get() }
    }

    /// # Safety
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub(crate) unsafe fn set_pos(self, ptr: NonNull<u8>) {
        self.set_pos_addr(nonnull::addr(ptr).get());
    }

    /// # Safety
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub(crate) unsafe fn set_pos_addr(self, addr: usize) {
        let ptr = self.with_addr(addr);
        self.header.as_ref().pos.set(ptr);
    }

    /// # Safety
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub(crate) unsafe fn with_addr(self, addr: usize) -> NonNull<u8> {
        debug_assert!(self.contains_addr_or_end(addr));
        let ptr = self.header.cast();
        let addr = NonZeroUsize::new_unchecked(addr);
        nonnull::with_addr(ptr, addr)
    }

    #[inline(always)]
    pub(crate) unsafe fn with_addr_range(self, range: Range<usize>) -> Range<NonNull<u8>> {
        debug_assert!(range.start <= range.end);
        let start = self.with_addr(range.start);
        let end = self.with_addr(range.end);
        start..end
    }

    #[inline(always)]
    pub(crate) fn contains_addr_or_end(self, addr: usize) -> bool {
        let start = nonnull::addr(self.content_start()).get();
        let end = nonnull::addr(self.content_end()).get();
        addr >= start && addr <= end
    }

    #[inline(always)]
    pub(crate) fn prev(self) -> Option<Self> {
        unsafe { Some(Self::from_header(self.header.as_ref().prev?)) }
    }

    #[inline(always)]
    pub(crate) fn next(self) -> Option<Self> {
        unsafe { Some(Self::from_header(self.header.as_ref().next.get()?)) }
    }

    #[inline(always)]
    pub(crate) fn capacity(self) -> usize {
        let start = nonnull::addr(self.content_start()).get();
        let end = nonnull::addr(self.content_end()).get();
        end - start
    }

    #[inline(always)]
    fn allocated_range(self) -> Range<NonNull<u8>> {
        if UP {
            self.content_start()..self.pos()
        } else {
            self.pos()..self.content_end()
        }
    }

    #[inline(always)]
    pub(crate) fn allocated(self) -> usize {
        let range = self.allocated_range();
        let start = nonnull::addr(range.start).get();
        let end = nonnull::addr(range.end).get();
        end - start
    }

    #[inline(always)]
    pub(crate) fn remaining(self) -> usize {
        let range = self.remaining_range();
        let start = nonnull::addr(range.start).get();
        let end = nonnull::addr(range.end).get();
        end - start
    }

    pub(crate) fn remaining_range(self) -> Range<NonNull<u8>> {
        if UP {
            let start = self.pos();
            let end = self.content_end();
            start..end
        } else {
            let start = self.content_start();
            let end = self.pos();
            start..end
        }
    }

    #[inline(always)]
    pub(crate) fn size(self) -> NonZeroUsize {
        let start = nonnull::addr(self.chunk_start()).get();
        let end = nonnull::addr(self.chunk_end()).get();
        unsafe { NonZeroUsize::new_unchecked(end - start) }
    }

    #[inline(always)]
    pub(crate) fn layout(self) -> Layout {
        // SAFETY: this layout fits the one we allocated, which means it must be valid
        unsafe { Layout::from_size_align_unchecked(self.size().get(), align_of::<ChunkHeader<A>>()) }
    }

    #[inline(always)]
    fn grow_size<B: ErrorBehavior>(self) -> Result<ChunkSizeHint<A, UP>, B> {
        let size = match self.size().get().checked_mul(2) {
            Some(size) => size,
            None => return Err(B::capacity_overflow()),
        };
        Ok(ChunkSizeHint::<A, UP>::new(size))
    }

    /// # Panic
    ///
    /// [`self.next`](RawChunk::next) must return `None`
    pub(crate) fn append_for<B: ErrorBehavior>(self, layout: Layout) -> Result<Self, B>
    where
        A: Allocator + Clone,
    {
        debug_assert!(self.next().is_none());

        let required_size = ChunkSizeHint::for_capacity(layout).ok_or_else(B::capacity_overflow)?;
        let grown_size = self.grow_size()?;
        let size = required_size.max(grown_size).calc_size().ok_or_else(B::capacity_overflow)?;

        let allocator = unsafe { self.header.as_ref().allocator.clone() };
        let new_chunk = RawChunk::new_in::<B>(size, Some(self), allocator)?;

        self.set_next(Some(new_chunk));
        Ok(new_chunk)
    }

    #[inline(always)]
    pub(crate) fn reset(self) {
        unsafe {
            if UP {
                self.set_pos(self.content_start());
            } else {
                self.set_pos(self.content_end());
            }
        }
    }

    /// # Safety
    /// self must not be used after calling this.
    pub(crate) unsafe fn deallocate(self)
    where
        A: Allocator,
    {
        let ptr = self.chunk_start();
        let layout = self.layout();
        let allocator_ptr = pointer::from_ref(&self.header.as_ref().allocator);
        let allocator = allocator_ptr.read();

        allocator.deallocate(ptr, layout);
    }

    #[inline(always)]
    pub(crate) fn set_prev(mut self, value: Option<Self>) {
        unsafe {
            self.header.as_mut().prev = value.map(|c| c.header);
        }
    }

    #[inline(always)]
    pub(crate) fn set_next(self, value: Option<Self>) {
        unsafe {
            self.header.as_ref().next.set(value.map(|c| c.header));
        }
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](RawChunk::deallocate) on the chunk parameter of `f` is fine.
    pub(crate) fn for_each_prev(self, mut f: impl FnMut(Self)) {
        let mut iter = self.prev();

        while let Some(chunk) = iter {
            iter = chunk.prev();
            f(chunk);
        }
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](RawChunk::deallocate) on the chunk parameter of `f` is fine.
    pub(crate) fn for_each_next(self, mut f: impl FnMut(Self)) {
        let mut iter = self.next();

        while let Some(chunk) = iter {
            iter = chunk.next();
            f(chunk);
        }
    }

    #[inline(always)]
    pub(crate) fn allocator(self) -> NonNull<A> {
        unsafe { NonNull::from(&self.header.as_ref().allocator) }
    }
}
