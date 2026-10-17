use core::{
    alloc::{AllocError, Allocator, Layout},
    cell::Cell,
    marker::PhantomData,
    mem::{MaybeUninit, transmute},
    num::NonZeroUsize,
    ops::Range,
    ptr::NonNull,
};

use crate::{
    SizedTypeProperties, align_pos,
    bumping::MIN_CHUNK_ALIGN,
    chunk_size::{ChunkSize, ChunkSizeCapacity, ChunkSizeHint, align_allocation_size},
    chunk_types::{
        r#abstract::{FirstAllocatedChunk, NonZstAllocatedChunk},
        concrete::{NonZstFirstAllocatedChunk, NonZstNonFirstAllocatedChunk, ZstAllocatedChunk},
    },
    config::{BumpConfig, config},
    error_behavior::ErrorBehavior,
    polyfill::non_null,
};

use super::non_zst_allocated_chunk::NonZstAllocatedChunkKind;

#[repr(C, align(16))]
struct Header<A, const C: BumpConfig> {
    pos: Cell<NonNull<u8>>,
    end: NonNull<u8>,

    prev: Option<AllocatedChunk<A, C>>,
    next: Cell<Option<AllocatedChunk<A, C>>>,
}

#[repr(transparent)]
pub(crate) struct AllocatedChunk<A, const C: BumpConfig> {
    header: NonNull<Header<A, C>>,
    marker: PhantomData<*const A>,
}

impl<A, const C: BumpConfig> Clone for AllocatedChunk<A, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const C: BumpConfig> Copy for AllocatedChunk<A, C> {}

impl<A, const C: BumpConfig> AllocatedChunk<A, C> {
    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.header.cast()
    }

    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.pos.get()
    }

    pub(crate) fn end(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.end
    }

    pub(crate) fn prev(self) -> Option<Self> {
        unsafe { self.header.as_ref() }.prev
    }

    pub(crate) fn next(self) -> Option<Self> {
        unsafe { self.header.as_ref() }.next.get()
    }

    pub(crate) fn set_pos(self, pos: NonNull<u8>) {
        unsafe { self.header.as_ref() }.pos.set(pos);
    }

    pub(crate) unsafe fn content_ptr_from_addr(self, addr: usize) -> NonNull<u8> {
        unsafe {
            debug_assert!(self.is_addr_in_content_range(addr));
            let ptr = self.header.cast();
            let addr = NonZeroUsize::new_unchecked(addr);
            ptr.with_addr(addr)
        }
    }

    pub(crate) unsafe fn content_ptr_from_addr_range(self, range: Range<usize>) -> Range<NonNull<u8>> {
        unsafe {
            debug_assert!(range.start <= range.end);
            let start = self.content_ptr_from_addr(range.start);
            let end = self.content_ptr_from_addr(range.end);
            start..end
        }
    }

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

    fn is_addr_in_content_range(self, addr: usize) -> bool {
        let start = self.content_start().addr().get();
        let end = self.content_end().addr().get();
        addr >= start && addr <= end
    }

    pub(crate) fn set_next(self, next: Option<Self>) {
        unsafe { self.header.as_ref() }.next.set(next);
    }

    pub(crate) fn kind(self) -> AllocatedChunkKind<A, C> {
        unsafe {
            if A::IS_ZST {
                AllocatedChunkKind::Zst(transmute(self))
            } else {
                AllocatedChunkKind::NonZst(transmute(self))
            }
        }
    }

    pub(crate) fn layout(self) -> Layout {
        let size = self.size().get();

        let align = if A::IS_ZST {
            ZstAllocatedChunk::<A, C>::HEADER_LAYOUT.align()
        } else {
            NonZstFirstAllocatedChunk::<A, C>::HEADER_LAYOUT.align()
        };

        // SAFETY: this layout fits the one we allocated, which means it must be valid
        unsafe { Layout::from_size_align_unchecked(size, align) }
    }

    fn after_header(self) -> NonNull<u8> {
        unsafe {
            self.header.cast::<u8>().byte_add(match self.kind() {
                AllocatedChunkKind::Zst(_) => ZstAllocatedChunk::<A, C>::HEADER_LAYOUT.size(),
                AllocatedChunkKind::NonZst(chunk) => match chunk.kind() {
                    NonZstAllocatedChunkKind::First(_) => NonZstFirstAllocatedChunk::<A, C>::HEADER_LAYOUT.size(),
                    NonZstAllocatedChunkKind::NonFirst(_) => NonZstNonFirstAllocatedChunk::<A, C>::HEADER_LAYOUT.size(),
                },
            })
        }
    }

    pub(crate) fn content_start(self) -> NonNull<u8> {
        if config!(upwards) {
            self.after_header()
        } else {
            self.chunk_start()
        }
    }

    pub(crate) fn content_end(self) -> NonNull<u8> {
        if config!(upwards) {
            self.chunk_end()
        } else {
            self.header.cast()
        }
    }

    pub(crate) fn remaining(self) -> usize {
        let start = if config!(upwards) { self.pos() } else { self.content_start() };
        let end = if config!(upwards) { self.content_end() } else { self.pos() };
        end.addr().get() - start.addr().get()
    }

    pub(crate) fn capacity(self) -> usize {
        let start = self.content_start();
        let end = self.content_end();
        end.addr().get() - start.addr().get()
    }

    pub(crate) fn allocated(self) -> usize {
        let start = if config!(upwards) { self.content_start() } else { self.pos() };
        let end = if config!(upwards) { self.pos() } else { self.content_end() };
        end.addr().get() - start.addr().get()
    }

    pub(crate) fn allocator_ptr(self) -> NonNull<A> {
        match self.kind() {
            AllocatedChunkKind::Zst(chunk) => chunk.allocator_ptr(),
            AllocatedChunkKind::NonZst(chunk) => chunk.allocator_ptr(),
        }
    }

    pub(crate) fn reset(self) {
        if config!(upwards) {
            self.set_pos(self.content_start());
        } else {
            self.set_pos(self.content_end());
        }
    }

    pub(crate) fn from_zst(zst: ZstAllocatedChunk<A, C>) -> Self {
        unsafe { transmute(zst) }
    }

    pub(crate) fn from_non_zst(non_zst: NonZstAllocatedChunk<A, C>) -> Self {
        unsafe { transmute(non_zst) }
    }

    pub(crate) fn from_first(zst: FirstAllocatedChunk<A, C>) -> Self {
        unsafe { transmute(zst) }
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

    pub(crate) fn new<B: ErrorBehavior>(layout: Layout, prev: AllocatedChunk<A, C>) -> Result<Self, B>
    where
        A: Allocator,
    {
        // We only add chunks at the end of the list.
        debug_assert!(prev.next().is_none());

        let size = {
            let required_size = ChunkSizeCapacity(layout).to_hint::<A, C>().ok_or_else(B::capacity_overflow)?;
            let grown_size = match prev.size().get().checked_mul(2) {
                Some(size) => ChunkSizeHint(size),
                None => return Err(B::capacity_overflow()),
            };
            required_size.max(grown_size)
        };

        let new_chunk = match prev.kind() {
            AllocatedChunkKind::Zst(prev) => {
                let allocator = MaybeUninit::<A>::uninit();

                // SAFETY: a zst does not have an invalid value and we're allowed
                // access to an instance of it, since we "store" one in the first chunk
                let allocator = unsafe { allocator.assume_init_ref() };

                let (ptr, size) = Self::allocate(size, allocator)?;

                AllocatedChunk::from_zst(unsafe { ZstAllocatedChunk::init(ptr, size, prev) })
            }
            AllocatedChunkKind::NonZst(prev) => {
                let (ptr, size) = Self::allocate(size, unsafe { prev.allocator_ptr().as_ref() })?;

                AllocatedChunk::from_non_zst(unsafe { NonZstAllocatedChunk::init(ptr, size, prev) })
            }
        };

        prev.set_next(Some(new_chunk));
        Ok(new_chunk)
    }

    pub(crate) fn allocate<E>(chunk_size: impl ChunkSize, allocator: &A) -> Result<(NonNull<u8>, usize), E>
    where
        A: Allocator,
        E: ErrorBehavior,
    {
        let layout = chunk_size.layout::<A, C>().ok_or_else(E::capacity_overflow)?;

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
        let size = align_allocation_size::<A, C>(size);

        debug_assert!(size >= layout.size());
        debug_assert_eq!(size % MIN_CHUNK_ALIGN, 0);

        Ok((ptr, size))
    }

    pub(crate) unsafe fn from_ptr(header: NonNull<()>) -> Self {
        Self {
            header: header.cast(),
            marker: PhantomData,
        }
    }
}

pub(crate) enum AllocatedChunkKind<A, const C: BumpConfig> {
    Zst(ZstAllocatedChunk<A, C>),
    NonZst(NonZstAllocatedChunk<A, C>),
}
