use core::{
    alloc::{AllocError, Allocator, Layout},
    cell::Cell,
    num::NonZeroUsize,
    ops::Range,
    ptr::NonNull,
};

use crate::{
    bumping::MIN_CHUNK_ALIGN,
    chunk_size::{ChunkSize, ChunkSizeCapacity, ChunkSizeHint, align_allocation_size},
    config::{BumpConfig, config},
    error_mode::ErrorMode,
    polyfill::non_null,
    util::align_pos,
};

#[repr(C, align(16))]
struct Header<const C: BumpConfig> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<AllocatedChunk<C>>,
    next: Cell<Option<AllocatedChunk<C>>>,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub(crate) struct AllocatedChunk<const C: BumpConfig> {
    header: NonNull<Header<C>>,
}

impl<const C: BumpConfig> AllocatedChunk<C> {
    pub(crate) const HEADER_LAYOUT: Layout = Layout::new::<Header<C>>();

    #[inline(always)]
    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.header.cast()
    }

    #[inline(always)]
    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.pos.get()
    }

    #[inline(always)]
    pub(crate) fn end(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.end
    }

    #[inline(always)]
    pub(crate) fn prev(self) -> Option<Self> {
        unsafe { self.header.as_ref() }.prev
    }

    #[inline(always)]
    pub(crate) fn next(self) -> Option<Self> {
        unsafe { self.header.as_ref() }.next.get()
    }

    #[inline(always)]
    pub(crate) fn set_pos(self, pos: NonNull<u8>) {
        unsafe { self.header.as_ref() }.pos.set(pos);
    }

    #[inline(always)]
    pub(crate) unsafe fn content_ptr_from_addr(self, addr: usize) -> NonNull<u8> {
        unsafe {
            debug_assert!(self.is_addr_in_content_range(addr));
            let ptr = self.header.cast();
            let addr = NonZeroUsize::new_unchecked(addr);
            ptr.with_addr(addr)
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn content_ptr_from_addr_range(self, range: Range<usize>) -> Range<NonNull<u8>> {
        unsafe {
            debug_assert!(range.start <= range.end);
            let start = self.content_ptr_from_addr(range.start);
            let end = self.content_ptr_from_addr(range.end);
            start..end
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn set_pos_addr(self, addr: usize) {
        unsafe { self.set_pos(self.content_ptr_from_addr(addr)) }
    }

    /// Sets the bump position and aligns it to the required `MIN_ALIGN`.
    #[inline(always)]
    pub(crate) unsafe fn set_pos_addr_and_align(self, pos: usize) {
        unsafe {
            let addr = align_pos(config!(upwards), config!(min_align), pos);
            self.set_pos_addr(addr);
        }
    }

    /// A version of [`set_pos_addr_and_align`](Self::set_pos_addr_and_align) that only aligns the pointer
    /// if it the `pos_align` is smaller than the `MIN_ALIGN`.
    ///
    /// This should only be called when the `pos_align` is statically known so
    /// the branch gets optimized out.
    #[inline(always)]
    pub(crate) unsafe fn set_pos_addr_and_align_from(self, mut pos: usize, pos_align: usize) {
        debug_assert_eq!(pos % pos_align, 0);

        if pos_align < config!(min_align) {
            pos = align_pos(config!(upwards), config!(min_align), pos);
        }

        unsafe { self.set_pos_addr(pos) };
    }

    #[inline(always)]
    fn is_addr_in_content_range(self, addr: usize) -> bool {
        let start = self.content_start().addr().get();
        let end = self.content_end().addr().get();
        addr >= start && addr <= end
    }

    #[inline(always)]
    pub(crate) fn mut_clear_prev(mut self) {
        unsafe { self.header.as_mut() }.prev = None;
    }

    #[inline(always)]
    pub(crate) fn set_next(self, next: Option<Self>) {
        unsafe { self.header.as_ref() }.next.set(next);
    }

    #[inline(always)]
    pub(crate) fn layout(self) -> Layout {
        let size = self.size().get();
        let align = Self::HEADER_LAYOUT.align();

        // SAFETY: this layout fits the one we allocated, which means it must be valid
        unsafe { Layout::from_size_align_unchecked(size, align) }
    }

    #[inline(always)]
    fn after_header(self) -> NonNull<u8> {
        unsafe { self.header.add(1).cast() }
    }

    #[inline(always)]
    pub(crate) fn content_start(self) -> NonNull<u8> {
        if config!(upwards) {
            self.after_header()
        } else {
            self.chunk_start()
        }
    }

    #[inline(always)]
    pub(crate) fn content_end(self) -> NonNull<u8> {
        if config!(upwards) {
            self.chunk_end()
        } else {
            self.header.cast()
        }
    }

    #[inline(always)]
    pub(crate) fn remaining(self) -> usize {
        let start = if config!(upwards) { self.pos() } else { self.content_start() };
        let end = if config!(upwards) { self.content_end() } else { self.pos() };
        end.addr().get() - start.addr().get()
    }

    #[inline(always)]
    pub(crate) fn capacity(self) -> usize {
        let start = self.content_start();
        let end = self.content_end();
        end.addr().get() - start.addr().get()
    }

    #[inline(always)]
    pub(crate) fn allocated(self) -> usize {
        let start = if config!(upwards) { self.content_start() } else { self.pos() };
        let end = if config!(upwards) { self.pos() } else { self.content_end() };
        end.addr().get() - start.addr().get()
    }

    #[inline(always)]
    pub(crate) fn reset(self) {
        if config!(upwards) {
            self.set_pos(self.content_start());
        } else {
            self.set_pos(self.content_end());
        }
    }

    #[inline(always)]
    pub(crate) fn chunk_start(self) -> NonNull<u8> {
        if config!(upwards) { self.header.cast() } else { self.end() }
    }

    #[inline(always)]
    pub(crate) fn chunk_end(self) -> NonNull<u8> {
        if config!(upwards) { self.end() } else { self.after_header() }
    }

    pub(crate) fn size(self) -> NonZeroUsize {
        let start = self.chunk_start().addr().get();
        let end = self.chunk_end().addr().get();
        unsafe { NonZeroUsize::new_unchecked(end - start) }
    }

    pub(crate) fn new_first<E: ErrorMode, A: Allocator>(chunk_size: impl ChunkSize, allocator: &A) -> Result<Self, E> {
        let chunk_layout = chunk_size.layout::<C, A>().ok_or_else(E::capacity_overflow)?;
        Self::new_chunk_with_layout(chunk_layout, None, allocator)
    }

    pub(crate) fn new_non_first<E: ErrorMode, A: Allocator>(
        layout: Layout,
        prev: AllocatedChunk<C>,
        allocator: &A,
    ) -> Result<Self, E> {
        // We only add chunks at the end of the list.
        debug_assert!(prev.next().is_none());

        let required_size = ChunkSizeCapacity(layout).to_hint::<C, A>().ok_or_else(E::capacity_overflow)?;

        let grown_size = match prev.size().get().checked_mul(2) {
            Some(size) => ChunkSizeHint(size),
            None => return Err(E::capacity_overflow()),
        };

        let chunk_size = required_size.max(grown_size);
        let chunk_layout = chunk_size.layout::<C, A>().ok_or_else(E::capacity_overflow)?;
        let new_chunk = Self::new_chunk_with_layout(chunk_layout, Some(prev), allocator)?;
        prev.set_next(Some(new_chunk));
        Ok(new_chunk)
    }

    fn new_chunk_with_layout<E: ErrorMode, A: Allocator>(
        layout: Layout,
        prev: Option<AllocatedChunk<C>>,
        allocator: &A,
    ) -> Result<Self, E> {
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
        let size = align_allocation_size::<C, A>(size);

        debug_assert!(size >= layout.size());
        debug_assert_eq!(size % MIN_CHUNK_ALIGN, 0);

        let new_chunk = unsafe {
            let header: NonNull<Header<C>>;
            let pos: NonNull<u8>;
            let end: NonNull<u8>;

            if config!(upwards) {
                header = ptr.cast();
                pos = header.add(1).cast();
                end = ptr.add(size);
            } else {
                header = ptr.add(size).cast::<Header<C>>().sub(1);
                pos = header.cast();
                end = ptr;
            }

            header.write(Header {
                pos: Cell::new(pos),
                end,
                prev,
                next: Cell::new(None),
            });

            Self { header }
        };

        Ok(new_chunk)
    }

    #[inline(always)]
    pub(crate) unsafe fn from_ptr(header: NonNull<()>) -> Self {
        Self { header: header.cast() }
    }
}
