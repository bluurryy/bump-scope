use core::{
    alloc::Layout, cell::Cell, fmt, marker::PhantomData, mem::align_of, num::NonZeroUsize, ops::Range, ptr::NonNull,
};

use crate::{
    ErrorBehavior,
    alloc::{AllocError, Allocator},
    bumping::{BumpProps, BumpUp, MIN_CHUNK_ALIGN, bump_down, bump_prepare_down, bump_prepare_up, bump_up},
    chunk::{ChunkHeader, ChunkSize, ChunkSizeHint},
    layout::LayoutProps,
    polyfill::non_null,
    settings::{Boolean, BumpAllocatorSettings, False, True},
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
pub(crate) struct RawChunk<A, S: BumpAllocatorSettings> {
    /// This points to a valid [`ChunkHeader`].
    header: NonNull<ChunkHeader<A>>,
    marker: PhantomData<fn() -> S>,
}

impl<A, S> Copy for RawChunk<A, S> where S: BumpAllocatorSettings {}

impl<A, S> Clone for RawChunk<A, S>
where
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> PartialEq for RawChunk<A, S>
where
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
    }

    #[inline(always)]
    fn ne(&self, other: &Self) -> bool {
        self.header != other.header
    }
}

impl<A, S> Eq for RawChunk<A, S> where S: BumpAllocatorSettings {}

impl<A, S> fmt::Debug for RawChunk<A, S>
where
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RawChunk").field(&self.header.cast::<u8>()).finish()
    }
}

/// Methods available for any chunk.
impl<A, S> RawChunk<A, S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) fn new_in<E: ErrorBehavior>(
        chunk_size: ChunkSize<A, S::Up>,
        prev: Option<Self>,
        allocator: A,
    ) -> Result<Self, E>
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
            if S::UP {
                let header = ptr.cast::<ChunkHeader<A>>();

                header.write(ChunkHeader {
                    pos: Cell::new(header.add(1).cast()),
                    end: ptr.add(size),
                    prev,
                    next,
                    allocator,
                });

                header
            } else {
                let header = ptr.add(size).cast::<ChunkHeader<A>>().sub(1);

                header.write(ChunkHeader {
                    pos: Cell::new(header.cast()),
                    end: ptr,
                    prev,
                    next,
                    allocator,
                });

                header
            }
        };

        Ok(RawChunk {
            header,
            marker: PhantomData,
        })
    }

    /// Cast the `S`ettings generic.
    pub(crate) unsafe fn cast<S2>(self) -> RawChunk<A, S2>
    where
        S2: BumpAllocatorSettings,
    {
        RawChunk {
            header: self.header,
            marker: PhantomData,
        }
    }

    pub(crate) fn is_allocated(self) -> bool {
        <S::GuaranteedAllocated as Boolean>::VALUE || self.header.cast() != ChunkHeader::unallocated::<S>()
    }

    pub(crate) fn is_unallocated(self) -> bool {
        !<S::GuaranteedAllocated as Boolean>::VALUE && self.header.cast() == ChunkHeader::unallocated::<S>()
    }

    pub(crate) fn guaranteed_allocated(self) -> Option<RawChunk<A, S::WithGuaranteedAllocated<true>>> {
        if self.is_unallocated() {
            return None;
        }

        Some(RawChunk {
            header: self.header,
            marker: PhantomData,
        })
    }

    pub(crate) unsafe fn guaranteed_allocated_unchecked(self) -> RawChunk<A, S::WithGuaranteedAllocated<true>> {
        debug_assert!(self.is_allocated());
        RawChunk {
            header: self.header,
            marker: PhantomData,
        }
    }

    pub(crate) fn not_guaranteed_allocated(self) -> RawChunk<A, S::WithGuaranteedAllocated<false>> {
        RawChunk {
            header: self.header,
            marker: PhantomData,
        }
    }

    pub(crate) fn header(self) -> NonNull<ChunkHeader<A>> {
        self.header
    }

    pub(crate) const unsafe fn from_header(header: NonNull<ChunkHeader<A>>) -> Self {
        Self {
            header,
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) fn bump_props<L>(self, layout: L) -> BumpProps
    where
        L: LayoutProps,
    {
        let pos = self.pos().addr().get();
        let end = unsafe { self.header.as_ref() }.end.addr().get();

        debug_assert_eq!(pos % S::MIN_ALIGN, 0);
        debug_assert_eq!(end % MIN_CHUNK_ALIGN, 0);

        let start = if S::UP { pos } else { end };
        let end = if S::UP { end } else { pos };

        #[cfg(debug_assertions)]
        if self.is_unallocated() {
            assert_eq!(end, MIN_CHUNK_ALIGN);
            assert_eq!(start, MIN_CHUNK_ALIGN * 2);
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

    /// Attempts to allocate a block of memory.
    ///
    /// On success, returns a [`NonNull<u8>`] meeting the size and alignment guarantees of `layout`.
    #[inline(always)]
    pub(crate) fn alloc(self, layout: impl LayoutProps) -> Option<NonNull<u8>> {
        let props = self.bump_props(layout);

        unsafe {
            if S::UP {
                let BumpUp { new_pos, ptr } = bump_up(props)?;

                // allocations never succeed for the unallocated chunk
                let chunk = self.guaranteed_allocated_unchecked();
                chunk.set_pos_addr(new_pos);
                Some(chunk.with_addr(ptr))
            } else {
                let ptr = bump_down(props)?;

                // allocations never succeed for the unallocated chunk
                let chunk = self.guaranteed_allocated_unchecked();
                let ptr = chunk.with_addr(ptr);
                chunk.set_pos(ptr);
                Some(ptr)
            }
        }
    }

    /// Prepares allocation for a block of memory.
    ///
    /// On success, returns a [`NonNull<u8>`] meeting the size and alignment guarantees of `layout`.
    ///
    /// This is like [`alloc`](Self::alloc), except that it won't change the bump pointer.
    #[inline(always)]
    pub(crate) fn prepare_allocation(self, layout: impl LayoutProps) -> Option<NonNull<u8>> {
        let props = self.bump_props(layout);

        unsafe {
            if S::UP {
                let BumpUp { ptr, .. } = bump_up(props)?;
                Some(self.with_addr(ptr))
            } else {
                let ptr = bump_down(props)?;
                Some(self.with_addr(ptr))
            }
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
    pub(crate) fn prepare_allocation_range(self, layout: impl LayoutProps) -> Option<Range<NonNull<u8>>> {
        debug_assert_ne!(layout.size(), 0);
        let props = self.bump_props(layout);

        unsafe {
            if S::UP {
                let range = bump_prepare_up(props)?;
                Some(self.with_addr_range(range))
            } else {
                let range = bump_prepare_down(props)?;
                Some(self.with_addr_range(range))
            }
        }
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
            debug_assert!(if let Some(chunk) = self.guaranteed_allocated() {
                chunk.contains_addr_or_end(addr)
            } else {
                true // can't check
            });
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
    pub(crate) fn prev(self) -> Option<RawChunk<A, S>> {
        // SAFETY: the `UNALLOCATED` chunk header never has a `prev` so this must be an allocated chunk if some
        unsafe { Some(RawChunk::from_header(self.header.as_ref().prev.get()?)) }
    }

    #[inline(always)]
    pub(crate) fn next(self) -> Option<RawChunk<A, S>> {
        // SAFETY: the `UNALLOCATED` chunk header never has a `next` so this must be an allocated chunk if some
        unsafe { Some(RawChunk::from_header(self.header.as_ref().next.get()?)) }
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](RawChunk::deallocate) on the chunk parameter of `f` is fine.
    pub(crate) fn for_each_prev(self, mut f: impl FnMut(RawChunk<A, S>)) {
        let mut iter = self.prev();

        while let Some(chunk) = iter {
            iter = chunk.prev();
            f(chunk);
        }
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](RawChunk::deallocate) on the chunk parameter of `f` is fine.
    pub(crate) fn for_each_next(self, mut f: impl FnMut(RawChunk<A, S>)) {
        let mut iter = self.next();

        while let Some(chunk) = iter {
            iter = chunk.next();
            f(chunk);
        }
    }

    #[inline(always)]
    pub(crate) fn stats<'a>(self) -> Stats<'a, A, S> {
        Stats::from_raw_chunk(self)
    }
}

/// Methods available for a non-guaranteed-allocated chunk.
impl<A, S> RawChunk<A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = False>,
{
    pub(crate) const UNALLOCATED: Self = Self {
        header: ChunkHeader::unallocated::<S>().cast(),
        marker: PhantomData,
    };
}

/// Methods available for a guaranteed-allocated chunk.
impl<A, S> RawChunk<A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
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
            if S::UP {
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

    #[inline(always)]
    fn after_header(self) -> NonNull<u8> {
        unsafe { self.header.add(1).cast() }
    }

    #[inline(always)]
    pub(crate) fn chunk_start(self) -> NonNull<u8> {
        unsafe { if S::UP { self.header.cast() } else { self.header.as_ref().end } }
    }

    #[inline(always)]
    pub(crate) fn chunk_end(self) -> NonNull<u8> {
        unsafe {
            if S::UP {
                self.header.as_ref().end
            } else {
                self.after_header()
            }
        }
    }

    #[inline(always)]
    pub(crate) fn content_start(self) -> NonNull<u8> {
        if S::UP { self.after_header() } else { self.chunk_start() }
    }

    #[inline(always)]
    pub(crate) fn content_end(self) -> NonNull<u8> {
        if S::UP { self.chunk_end() } else { self.header.cast() }
    }

    #[inline(always)]
    pub(crate) fn capacity(self) -> usize {
        let start = self.content_start().addr().get();
        let end = self.content_end().addr().get();
        end - start
    }

    #[inline(always)]
    fn allocated_range(self) -> Range<NonNull<u8>> {
        if S::UP {
            self.content_start()..self.pos()
        } else {
            self.pos()..self.content_end()
        }
    }

    #[inline(always)]
    pub(crate) fn contains_addr_or_end(self, addr: usize) -> bool {
        let start = self.content_start().addr().get();
        let end = self.content_end().addr().get();
        addr >= start && addr <= end
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
        if S::UP {
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
    fn grow_size<B: ErrorBehavior>(self) -> Result<ChunkSizeHint<A, S::Up>, B> {
        let Some(size) = self.size().get().checked_mul(2) else {
            return Err(B::capacity_overflow());
        };

        Ok(ChunkSizeHint::new(size))
    }
}
