use core::{
    alloc::{AllocError, Allocator, Layout},
    cell::Cell,
    marker::PhantomData,
    mem::MaybeUninit,
    num::NonZeroUsize,
    ops::{Deref, Range},
    ptr::NonNull,
};

use crate::{
    SizedTypeProperties, align_pos,
    chunk::{
        AllocatedChunkHeader, ChunkSize, ChunkSizeCapacity, ChunkSizeHint, MIN_CHUNK_ALIGN, NonZstAllocatedChunkHeader,
        NonZstFirstAllocatedChunkHeader, align_allocation_size, chunk::Chunk,
    },
    error_behavior::ErrorBehavior,
    polyfill::{non_null, transmute_ref},
    settings::BumpAllocatorSettings,
};

/// Represents **an allocated** chunk of the bump allocator.
///
/// This type behaves somewhat like a `ManuallyDrop<T>` in the sense that it has safe to
/// use methods that assume the chunk has not been deallocated.
///
/// So just the `deallocate` method is unsafe. You have to make sure the chunk is not used
/// after calling that.
#[repr(transparent)]
pub(crate) struct AllocatedChunk<A, S> {
    header: NonNull<AllocatedChunkHeader>,
    marker: PhantomData<fn() -> (A, S)>,
}

impl<A, S> Copy for AllocatedChunk<A, S> {}

impl<A, S> Clone for AllocatedChunk<A, S> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> Deref for AllocatedChunk<A, S> {
    type Target = Chunk<A, S>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { transmute_ref(self) }
    }
}

impl<A, S> AllocatedChunk<A, S>
where
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    pub(crate) unsafe fn from_raw(header: NonNull<AllocatedChunkHeader>) -> Self {
        Self {
            header,
            marker: PhantomData,
        }
    }

    pub(crate) fn new<B: ErrorBehavior>(layout: Layout, prev: AllocatedChunk<A, S>) -> Result<Self, B>
    where
        A: Allocator,
    {
        // We only add chunks at the end of the list.
        debug_assert!(prev.next().is_none());

        let size = {
            let required_size = ChunkSizeCapacity(layout).to_hint::<A, S>().ok_or_else(B::capacity_overflow)?;
            let grown_size = prev.grow_size()?;
            required_size.max(grown_size)
        };

        let header: NonNull<AllocatedChunkHeader> = if A::IS_ZST {
            let allocator = MaybeUninit::<A>::uninit();

            // SAFETY: a zst does not have an invalid value and we're allowed
            // access to an instance of it, since we "store" one in the first chunk
            let allocator = unsafe { allocator.assume_init_ref() };

            let (ptr, size) = Self::allocate(size, allocator)?;

            unsafe {
                let header = if S::UP {
                    ptr.cast::<AllocatedChunkHeader>()
                } else {
                    ptr.add(size).cast::<AllocatedChunkHeader>().sub(1)
                };

                let pos = Cell::new(if S::UP { header.add(1).cast() } else { header.cast() });
                let end = if S::UP { ptr.add(size) } else { ptr };

                header.write(AllocatedChunkHeader {
                    pos,
                    end,
                    prev: Cell::new(Some(prev.header.cast())),
                    next: Cell::new(None),
                });

                header
            }
        } else {
            let allocator_ptr = prev.allocator_ptr();
            let (ptr, size) = Self::allocate(size, unsafe { allocator_ptr.as_ref() })?;

            unsafe {
                let header = if S::UP {
                    ptr.cast::<NonZstAllocatedChunkHeader<A>>()
                } else {
                    ptr.add(size).cast::<NonZstAllocatedChunkHeader<A>>().sub(1)
                };

                let pos = Cell::new(if S::UP { header.add(1).cast() } else { header.cast() });
                let end = if S::UP { ptr.add(size) } else { ptr };

                header.write(NonZstAllocatedChunkHeader {
                    pos,
                    end,
                    prev: Cell::new(Some(prev.header.cast())),
                    next: Cell::new(None),
                    allocator_ptr,
                });

                header.cast()
            }
        };

        unsafe { prev.header.as_ref() }.next.set(Some(header.cast()));

        Ok(Self {
            header: header.cast(),
            marker: PhantomData,
        })
    }

    pub(super) fn allocate<E>(chunk_size: impl ChunkSize, allocator: &A) -> Result<(NonNull<u8>, usize), E>
    where
        A: Allocator,
        E: ErrorBehavior,
    {
        let layout = chunk_size.layout::<A, S>().ok_or_else(E::capacity_overflow)?;

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
        let size = align_allocation_size::<A, S>(size);

        debug_assert!(size >= layout.size());
        debug_assert_eq!(size % MIN_CHUNK_ALIGN, 0);

        Ok((ptr, size))
    }

    #[inline(always)]
    fn grow_size<B: ErrorBehavior>(self) -> Result<ChunkSizeHint, B> {
        let Some(size) = self.size().get().checked_mul(2) else {
            return Err(B::capacity_overflow());
        };

        Ok(ChunkSizeHint(size))
    }

    #[inline(always)]
    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref().pos.get() }
    }

    #[inline(always)]
    pub(crate) fn header(self) -> NonNull<AllocatedChunkHeader> {
        self.header
    }

    /// The caller must ensure the returned reference is dead before calling [`deallocate`](Self::deallocate).
    #[inline(always)]
    pub(crate) fn allocator<'a>(self) -> &'a A {
        unsafe { self.allocator_ptr().as_ref() }
    }

    #[inline(always)]
    pub(crate) fn allocator_ptr(self) -> NonNull<A> {
        if A::IS_ZST {
            NonNull::dangling()
        } else {
            unsafe { self.as_non_zst().as_ref().allocator_ptr }
        }
    }

    pub(crate) unsafe fn as_non_zst(self) -> NonNull<NonZstAllocatedChunkHeader<A>> {
        debug_assert!(!A::IS_ZST);
        self.header.cast()
    }

    #[inline(always)]
    pub(crate) fn prev(self) -> Option<AllocatedChunk<A, S>> {
        unsafe {
            Some(AllocatedChunk {
                header: self.header.as_ref().prev.get()?,
                marker: PhantomData,
            })
        }
    }

    #[inline(always)]
    pub(crate) fn next(self) -> Option<AllocatedChunk<A, S>> {
        unsafe {
            Some(AllocatedChunk {
                header: self.header.as_ref().next.get()?,
                marker: PhantomData,
            })
        }
    }

    #[inline(always)]
    pub(crate) fn size(self) -> NonZeroUsize {
        let start = self.chunk_start().addr().get();
        let end = self.chunk_end().addr().get();
        unsafe { NonZeroUsize::new_unchecked(end - start) }
    }

    #[inline(always)]
    pub(crate) fn capacity(self) -> usize {
        let start = self.content_start().addr().get();
        let end = self.content_end().addr().get();
        end - start
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

    /// # Safety
    /// [`contains_addr_or_end`](AllocatedChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub(crate) unsafe fn set_pos(self, ptr: NonNull<u8>) {
        unsafe { self.set_pos_addr(ptr.addr().get()) };
    }

    /// # Safety
    /// [`contains_addr_or_end`](AllocatedChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub(crate) unsafe fn set_pos_addr(self, addr: usize) {
        unsafe { self.header.as_ref().pos.set(self.content_ptr_from_addr(addr)) };
    }

    /// Sets the bump position and aligns it to the required `MIN_ALIGN`.
    #[inline(always)]
    pub(crate) unsafe fn set_pos_addr_and_align(self, pos: usize) {
        unsafe {
            let addr = align_pos(S::UP, S::MIN_ALIGN, pos);
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

        if pos_align < S::MIN_ALIGN {
            pos = align_pos(S::UP, S::MIN_ALIGN, pos);
        }

        unsafe { self.set_pos_addr(pos) };
    }

    /// # Safety
    /// [`contains_addr_or_end`](AllocatedChunk::contains_addr_or_end) must return true
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
    fn is_addr_in_content_range(self, addr: usize) -> bool {
        let start = self.content_start().addr().get();
        let end = self.content_end().addr().get();
        addr >= start && addr <= end
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
    fn remaining_range(self) -> Range<NonNull<u8>> {
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
    fn after_header(self) -> NonNull<u8> {
        unsafe {
            if A::IS_ZST {
                self.header.add(1).cast()
            } else if self.prev().is_none() {
                self.header.cast::<NonZstFirstAllocatedChunkHeader<A>>().add(1).cast()
            } else {
                self.header.cast::<NonZstAllocatedChunkHeader<A>>().add(1).cast()
            }
        }
    }

    #[inline(always)]
    pub(crate) fn layout(self) -> Layout {
        let size = self.size().get();

        let align = if A::IS_ZST {
            align_of::<AllocatedChunkHeader>()
        } else {
            align_of::<NonZstFirstAllocatedChunkHeader<A>>()
        };

        // SAFETY: this layout fits the one we allocated, which means it must be valid
        unsafe { Layout::from_size_align_unchecked(size, align) }
    }
}
