use core::{alloc::Layout, cell::Cell, fmt, mem::align_of, num::NonZeroUsize, ops::Range, ptr::NonNull};

use crate::{
    ChunkHeader, ErrorBehavior, SupportedMinimumAlignment,
    alloc::{AllocError, Allocator},
    bumping::{BumpProps, BumpUp, MIN_CHUNK_ALIGN, bump_down, bump_prepare_down, bump_prepare_up, bump_up},
    chunk_size::{ChunkSize, ChunkSizeHint},
    layout::LayoutProps,
    polyfill::{self, non_null},
    stats::Stats,
};

/// Represents an allocated chunk.
///
/// This type behaves somewhat like a `ManuallyDrop<T>` in the sense that it has safe to
/// use methods that assume the chunk has not been deallocated.
///
/// So just the `deallocate` method is unsafe. You have to make sure the chunk is not used
/// after calling that.
#[repr(transparent)]
pub(crate) struct RawChunk<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> {
    /// This points to a valid [`ChunkHeader`].
    header: NonNull<ChunkHeader<A>>,
}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> Copy for RawChunk<A, UP, GUARANTEED_ALLOCATED> {}

#[allow(clippy::expl_impl_clone_on_copy)]
impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> Clone for RawChunk<A, UP, GUARANTEED_ALLOCATED> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> PartialEq for RawChunk<A, UP, GUARANTEED_ALLOCATED> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
    }

    #[inline(always)]
    fn ne(&self, other: &Self) -> bool {
        self.header != other.header
    }
}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> Eq for RawChunk<A, UP, GUARANTEED_ALLOCATED> {}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> fmt::Debug for RawChunk<A, UP, GUARANTEED_ALLOCATED> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RawChunk").field(&self.header.as_ptr().cast::<u8>()).finish()
    }
}

impl<A, const UP: bool> RawChunk<A, UP, false> {
    pub(crate) const UNALLOCATED: Self = Self {
        header: ChunkHeader::UNALLOCATED.cast(),
    };
}

impl<A, const UP: bool> RawChunk<A, UP, true> {
    pub(crate) fn new_in<E: ErrorBehavior>(chunk_size: ChunkSize<A, UP>, prev: Option<Self>, allocator: A) -> Result<Self, E>
    where
        A: Allocator,
    {
        let layout = chunk_size.layout().ok_or_else(E::capacity_overflow)?;

        let allocation = match allocator.allocate(layout) {
            Ok(ok) => ok,
            Err(AllocError) => return Err(E::allocation(layout)),
        };

        let ptr = non_null::as_non_null_ptr(allocation);
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

        let prev = Cell::new(prev.map(|c| c.header));
        let next = Cell::new(None);

        let header = unsafe {
            if UP {
                let header = ptr.cast::<ChunkHeader<A>>();

                header.as_ptr().write(ChunkHeader {
                    pos: Cell::new(header.add(1).cast()),
                    end: ptr.add(size),
                    prev,
                    next,
                    allocator,
                });

                header
            } else {
                let header = ptr.add(size).cast::<ChunkHeader<A>>().sub(1);

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

    pub(crate) fn coerce_guaranteed_allocated<const NEW_GUARANTEED_ALLOCATED: bool>(
        self,
    ) -> RawChunk<A, UP, NEW_GUARANTEED_ALLOCATED> {
        RawChunk { header: self.header }
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

    /// # Safety
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub(crate) unsafe fn set_pos(self, ptr: NonNull<u8>) {
        unsafe { self.set_pos_addr(ptr.addr().get()) };
    }

    /// # Safety
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub(crate) unsafe fn set_pos_addr(self, addr: usize) {
        unsafe { self.header.as_ref().pos.set(self.with_addr(addr)) };
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

    #[inline(always)]
    pub(crate) fn allocator<'a>(self) -> &'a A {
        unsafe { &self.header.as_ref().allocator }
    }

    #[inline(always)]
    pub(crate) fn set_prev(self, value: Option<Self>) {
        unsafe {
            self.header.as_ref().prev.set(value.map(|c| c.header));
        }
    }

    #[inline(always)]
    pub(crate) fn set_next(self, value: Option<Self>) {
        unsafe {
            self.header.as_ref().next.set(value.map(|c| c.header));
        }
    }

    /// # Safety
    /// - self must not be used after calling this.
    pub(crate) unsafe fn deallocate(self)
    where
        A: Allocator,
    {
        let ptr = self.chunk_start();
        let layout = self.layout();

        unsafe {
            let allocator_ptr = &raw const (*self.header.as_ptr()).allocator;
            let allocator = allocator_ptr.read();
            allocator.deallocate(ptr, layout);
        }
    }
}

impl<A, const UP: bool, const GUARANTEED_ALLOCATED: bool> RawChunk<A, UP, GUARANTEED_ALLOCATED> {
    pub(crate) fn is_allocated(self) -> bool {
        GUARANTEED_ALLOCATED || self.header.cast() != ChunkHeader::UNALLOCATED
    }

    pub(crate) fn is_unallocated(self) -> bool {
        !GUARANTEED_ALLOCATED && self.header.cast() == ChunkHeader::UNALLOCATED
    }

    pub(crate) fn guaranteed_allocated(self) -> Option<RawChunk<A, UP, true>> {
        if self.is_unallocated() {
            return None;
        }

        Some(RawChunk { header: self.header })
    }

    pub(crate) unsafe fn guaranteed_allocated_unchecked(self) -> RawChunk<A, UP, true> {
        debug_assert!(self.is_allocated());
        RawChunk { header: self.header }
    }

    pub(crate) fn not_guaranteed_allocated(self) -> RawChunk<A, UP, false> {
        RawChunk { header: self.header }
    }

    pub(crate) fn header(self) -> NonNull<ChunkHeader<A>> {
        self.header
    }

    pub(crate) const unsafe fn from_header(header: NonNull<ChunkHeader<A>>) -> Self {
        Self { header }
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
        // Zero sized allocations are a problem for non-GUARANTEED_ALLOCATED chunks
        // since bump_up/bump_down will succeed and the UNALLOCATED chunk would
        // be written to. The UNALLOCATED chunk must not be written to since that
        // could cause a data-race (UB).
        //
        // In many cases, the layout size is statically known not to be zero
        // and this "if" is optimized away.
        if !GUARANTEED_ALLOCATED && layout.size() == 0 {
            return Ok(polyfill::layout::dangling(*layout));
        }

        let props = self.bump_props(minimum_alignment, layout);

        unsafe {
            if UP {
                match bump_up(props) {
                    Some(BumpUp { new_pos, ptr }) => {
                        // non zero sized allocations never succeed for the unallocated chunk
                        let chunk = self.guaranteed_allocated_unchecked();
                        chunk.set_pos_addr(new_pos);
                        Ok(chunk.with_addr(ptr))
                    }
                    None => f(),
                }
            } else {
                match bump_down(props) {
                    Some(ptr) => {
                        // non zero sized allocations never succeed for the unallocated chunk
                        let chunk = self.guaranteed_allocated_unchecked();
                        let ptr = chunk.with_addr(ptr);
                        chunk.set_pos(ptr);
                        Ok(ptr)
                    }
                    None => f(),
                }
            }
        }
    }

    // FIXME: change naming not to confuse it with `prepare_allocation_range`
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
        debug_assert!(non_null::is_aligned_to(self.pos(), M::MIN_ALIGN));
        let remaining = self.remaining_range();

        BumpProps {
            start: remaining.start.addr().get(),
            end: remaining.end.addr().get(),
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
    pub(crate) fn prepare_allocation_range(
        self,
        minimum_alignment: impl SupportedMinimumAlignment,
        layout: impl LayoutProps,
    ) -> Option<Range<NonNull<u8>>> {
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
    fn after_header(self) -> NonNull<u8> {
        unsafe { self.header.add(1).cast() }
    }

    #[inline(always)]
    pub(crate) fn chunk_start(self) -> NonNull<u8> {
        unsafe { if UP { self.header.cast() } else { self.header.as_ref().end } }
    }

    #[inline(always)]
    pub(crate) fn chunk_end(self) -> NonNull<u8> {
        unsafe { if UP { self.header.as_ref().end } else { self.after_header() } }
    }

    #[inline(always)]
    pub(crate) fn content_start(self) -> NonNull<u8> {
        if UP { self.after_header() } else { self.chunk_start() }
    }

    #[inline(always)]
    pub(crate) fn content_end(self) -> NonNull<u8> {
        if UP { self.chunk_end() } else { self.header.cast() }
    }

    #[inline(always)]
    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref().pos.get() }
    }

    /// # Safety
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub(crate) unsafe fn with_addr(self, addr: usize) -> NonNull<u8> {
        unsafe {
            debug_assert!(self.contains_addr_or_end(addr));
            let ptr = self.header.cast();
            let addr = NonZeroUsize::new_unchecked(addr);
            ptr.with_addr(addr)
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn with_addr_range(self, range: Range<usize>) -> Range<NonNull<u8>> {
        unsafe {
            debug_assert!(range.start <= range.end);
            let start = self.with_addr(range.start);
            let end = self.with_addr(range.end);
            start..end
        }
    }

    #[inline(always)]
    pub(crate) fn contains_addr_or_end(self, addr: usize) -> bool {
        let start = self.content_start().addr().get();
        let end = self.content_end().addr().get();
        addr >= start && addr <= end
    }

    #[inline(always)]
    pub(crate) fn prev(self) -> Option<RawChunk<A, UP, true>> {
        // SAFETY: the `UNALLOCATED` chunk header never has a `prev` so this must be an allocated chunk if some
        unsafe { Some(RawChunk::from_header(self.header.as_ref().prev.get()?)) }
    }

    #[inline(always)]
    pub(crate) fn next(self) -> Option<RawChunk<A, UP, true>> {
        // SAFETY: the `UNALLOCATED` chunk header never has a `next` so this must be an allocated chunk if some
        unsafe { Some(RawChunk::from_header(self.header.as_ref().next.get()?)) }
    }

    #[inline(always)]
    pub(crate) fn capacity(self) -> usize {
        let start = self.content_start().addr().get();
        let end = self.content_end().addr().get();
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
        let start = range.start.addr().get();
        let end = range.end.addr().get();
        end - start
    }

    #[inline(always)]
    pub(crate) fn remaining(self) -> usize {
        let range = self.remaining_range();
        let start = range.start.addr().get();
        let end = range.end.addr().get();
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
        let start = self.chunk_start().addr().get();
        let end = self.chunk_end().addr().get();
        unsafe { NonZeroUsize::new_unchecked(end - start) }
    }

    #[inline(always)]
    pub(crate) fn layout(self) -> Layout {
        // SAFETY: this layout fits the one we allocated, which means it must be valid
        unsafe { Layout::from_size_align_unchecked(self.size().get(), align_of::<ChunkHeader<A>>()) }
    }

    #[inline(always)]
    fn grow_size<B: ErrorBehavior>(self) -> Result<ChunkSizeHint<A, UP>, B> {
        let Some(size) = self.size().get().checked_mul(2) else {
            return Err(B::capacity_overflow());
        };

        Ok(ChunkSizeHint::<A, UP>::new(size))
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](RawChunk::deallocate) on the chunk parameter of `f` is fine.
    pub(crate) fn for_each_prev(self, mut f: impl FnMut(RawChunk<A, UP, true>)) {
        let mut iter = self.prev();

        while let Some(chunk) = iter {
            iter = chunk.prev();
            f(chunk);
        }
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](RawChunk::deallocate) on the chunk parameter of `f` is fine.
    pub(crate) fn for_each_next(self, mut f: impl FnMut(RawChunk<A, UP, true>)) {
        let mut iter = self.next();

        while let Some(chunk) = iter {
            iter = chunk.next();
            f(chunk);
        }
    }

    #[inline(always)]
    pub(crate) fn stats<'a>(self) -> Stats<'a, A, UP, GUARANTEED_ALLOCATED> {
        Stats::from_raw_chunk(self)
    }
}
