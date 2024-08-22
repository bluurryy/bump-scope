use core::{alloc::Layout, cell::Cell, num::NonZeroUsize, ops::Range, ptr::NonNull};

use crate::{
    down_align_usize,
    layout::LayoutProps,
    polyfill::{const_unwrap, nonnull, pointer},
    unallocated_chunk_header, up_align_nonzero, up_align_nonzero_unchecked, up_align_usize_unchecked, ChunkHeader,
    ChunkSize, ErrorBehavior, MinimumAlignment, SizedTypeProperties, SupportedMinimumAlignment, CHUNK_ALIGN_MIN,
};

use allocator_api2::alloc::Allocator;

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
    pub fn new_in<E: ErrorBehavior>(size: ChunkSize<UP, A>, prev: Option<Self>, allocator: A) -> Result<Self, E>
    where
        A: Allocator,
        for<'a> &'a A: Allocator,
    {
        let ptr = size.allocate(&allocator)?;

        let size = ptr.len();
        let ptr = ptr.cast::<u8>();

        // The chunk size must always be a multiple of `CHUNK_ALIGN_MIN`.
        // We use optimizations in `alloc` that require this.
        // `ChunkSize::allocate` must have already trimmed the allocation size to a multiple of `CHUNK_ALIGN_MIN`
        debug_assert!(size % CHUNK_ALIGN_MIN == 0);

        let header = unsafe {
            if UP {
                let header = ptr.cast::<ChunkHeader<A>>();

                header.as_ptr().write(ChunkHeader {
                    pos: Cell::new(nonnull::add(header, 1).cast()),
                    end: nonnull::add(ptr, size),

                    prev: prev.map(|c| c.header),
                    next: Cell::new(None),

                    allocator,
                });

                header
            } else {
                let header = nonnull::sub(nonnull::add(ptr, size).cast::<ChunkHeader<A>>(), 1);

                header.as_ptr().write(ChunkHeader {
                    pos: Cell::new(header.cast()),
                    end: ptr,

                    prev: prev.map(|c| c.header),
                    next: Cell::new(None),

                    allocator,
                });

                header
            }
        };

        Ok(RawChunk { header })
    }

    pub fn header_ptr(self) -> NonNull<ChunkHeader<A>> {
        self.header
    }

    pub const unsafe fn from_header(header: NonNull<ChunkHeader<A>>) -> Self {
        Self { header }
    }

    pub fn is_unallocated(self) -> bool {
        self.header.cast() == unallocated_chunk_header()
    }

    /// Attempts to allocate a block of memory.
    ///
    /// On success, returns a [`NonNull<u8>`] meeting the size and alignment guarantees of `layout`.
    #[inline(always)]
    pub fn alloc<M, L>(self, minimum_alignment: M, layout: L) -> Option<NonNull<u8>>
    where
        M: SupportedMinimumAlignment,
        L: LayoutProps,
    {
        self.alloc_or_else(minimum_alignment, layout, || Err(())).ok()
    }

    #[inline(always)]
    pub fn alloc_or_else<M, L, E, F>(self, _: M, layout: L, f: F) -> Result<NonNull<u8>, E>
    where
        M: SupportedMinimumAlignment,
        L: LayoutProps,
        F: FnOnce() -> Result<NonNull<u8>, E>,
    {
        debug_assert!(nonnull::is_aligned_to(self.pos(), M::MIN_ALIGN));

        if L::SIZE_IS_MULTIPLE_OF_ALIGN {
            debug_assert!(layout.size() % layout.align() == 0);
        }

        let remaining = self.remaining_range();

        if UP {
            let mut start = nonnull::addr(remaining.start).get();
            let end = nonnull::addr(remaining.end).get();

            debug_assert!(start <= end);
            debug_assert!(end % CHUNK_ALIGN_MIN == 0);

            let mut new_pos;

            // doing the `layout.size() < CHUNK_ALIGN_MIN` trick here (as seen in !UP)
            // results in worse codegen, so we don't

            if L::ALIGN_IS_CONST && layout.align() <= CHUNK_ALIGN_MIN {
                // Constant, small alignment fast path!

                if L::ALIGN_IS_CONST && layout.align() <= M::MIN_ALIGN {
                    // alignment is already sufficient
                } else {
                    // Aligning an address that is `<= range.end` with an alignment
                    // that is `<= CHUNK_ALIGN_MIN` can not exceed `range.end` and
                    // can not overflow as `range.end` is always aligned to `CHUNK_ALIGN_MIN`
                    start = up_align_usize_unchecked(start, layout.align());
                }

                let remaining = end - start;

                if layout.size() > remaining {
                    return f();
                }

                // doesn't exceed `end` because of the check above
                new_pos = start + layout.size();
            } else {
                // Alignment is `> CHUNK_ALIGN_MIN` or unknown.

                // start and align are both nonzero
                // `aligned_down` is the aligned pointer minus `layout.align()`
                let aligned_down = (start - 1) & !(layout.align() - 1);

                // align + size cannot overflow as per `Layout`'s rules
                //
                // this could also be a `checked_add`, but we use `saturating_add` to save us a branch;
                // the `if` below will return None if the addition saturated and returned `usize::MAX`
                new_pos = aligned_down.saturating_add(layout.align() + layout.size());

                // note that `new_pos` being `usize::MAX` is an invalid value for `new_pos` and we MUST return None;
                // due to `end` being always aligned to `CHUNK_ALIGN_MIN`, it can't be `usize::MAX`;
                // thus when `new_pos` is `usize::MAX` this will always return None;
                if new_pos > end {
                    return f();
                }

                // doesn't exceed `end` because `aligned_down + align + size` didn't
                start = aligned_down + layout.align();
            };

            if (L::ALIGN_IS_CONST && L::SIZE_IS_MULTIPLE_OF_ALIGN && layout.align() >= M::MIN_ALIGN)
                || (L::SIZE_IS_CONST && (layout.size() % M::MIN_ALIGN == 0))
            {
                // we are already aligned to `MIN_ALIGN`
            } else {
                // up aligning an address `<= range.end` with an alignment `<= CHUNK_ALIGN_MIN` (which `MIN_ALIGN` is)
                // can not exceed `range.end`, and thus also can't overflow
                new_pos = up_align_usize_unchecked(new_pos, M::MIN_ALIGN);
            }

            debug_assert!(is_aligned(start, layout.align()));
            debug_assert!(is_aligned(start, M::MIN_ALIGN));
            debug_assert!(is_aligned(new_pos, M::MIN_ALIGN));

            unsafe {
                self.set_pos(self.with_addr(new_pos));
                Ok(self.with_addr(start))
            }
        } else {
            let start = nonnull::addr(remaining.start).get();
            let mut end = nonnull::addr(remaining.end).get();

            debug_assert!(start <= end);

            if L::SIZE_IS_CONST && layout.size() <= CHUNK_ALIGN_MIN {
                // When `size <= CHUNK_ALIGN_MIN` subtracting it from `end` can't overflow, as the lowest value for `end` would be `start` which is aligned to `CHUNK_ALIGN_MIN`,
                // thus its address can't be smaller than it.
                end -= layout.size();

                let needs_align_for_min_align =
                    (!L::ALIGN_IS_CONST || !L::SIZE_IS_MULTIPLE_OF_ALIGN || layout.align() < M::MIN_ALIGN)
                        && (!L::SIZE_IS_CONST || (layout.size() % M::MIN_ALIGN != 0));
                let needs_align_for_layout =
                    !L::ALIGN_IS_CONST || !L::SIZE_IS_MULTIPLE_OF_ALIGN || layout.align() > M::MIN_ALIGN;

                if needs_align_for_min_align || needs_align_for_layout {
                    // At this point layout's align is const, because we assume `L::SIZE_IS_CONST` implies `L::ALIGN_IS_CONST`.
                    // That means `max` is evaluated at compile time, so we don't bother having different cases for either alignment.
                    end = down_align_usize(end, layout.align().max(M::MIN_ALIGN));
                }

                if end < start {
                    return f();
                }
            } else if L::ALIGN_IS_CONST && layout.align() <= CHUNK_ALIGN_MIN {
                // Constant, small alignment fast path!
                let remaining = end - start;

                if layout.size() > remaining {
                    return f();
                }

                // doesn't overflow because of the check above
                end -= layout.size();

                let needs_align_for_min_align =
                    (!L::ALIGN_IS_CONST || !L::SIZE_IS_MULTIPLE_OF_ALIGN || layout.align() < M::MIN_ALIGN)
                        && (!L::SIZE_IS_CONST || (layout.size() % M::MIN_ALIGN != 0));
                let needs_align_for_layout =
                    !L::ALIGN_IS_CONST || !L::SIZE_IS_MULTIPLE_OF_ALIGN || layout.align() > M::MIN_ALIGN;

                if needs_align_for_min_align || needs_align_for_layout {
                    // down aligning an address `>= range.start` with an alignment `<= CHUNK_ALIGN_MIN` (which `layout.align()` is)
                    // can not exceed `range.start`, and thus also can't overflow
                    end = down_align_usize(end, layout.align().max(M::MIN_ALIGN));
                }
            } else {
                // Alignment is `> CHUNK_ALIGN_MIN` or unknown.

                // this could also be a `checked_sub`, but we use `saturating_sub` to save us a branch;
                // the `if` below will return None if the addition saturated and returned `0`
                end = end.saturating_sub(layout.size());
                end = down_align_usize(end, layout.align().max(M::MIN_ALIGN));

                // note that `end` being `0` is an invalid value for `end` and we MUST return None;
                // due to `start` being `NonNull`, it can't be `0`;
                // thus when `end` is `0` this will always return None;
                if end < start {
                    return f();
                }
            };

            debug_assert!(is_aligned(end, layout.align()));
            debug_assert!(is_aligned(end, M::MIN_ALIGN));

            unsafe {
                self.set_pos(self.with_addr(end));
                Ok(self.with_addr(end))
            }
        }
    }

    #[inline(always)]
    pub fn alloc_no_bump_for<const MIN_ALIGN: usize, T>(self) -> Option<NonNull<u8>>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        // for comments see `alloc`

        let remaining = self.remaining_range();

        if UP {
            let mut start = nonnull::addr(remaining.start).get();
            let end = nonnull::addr(remaining.end).get();

            debug_assert!(start <= end);
            debug_assert!(end % CHUNK_ALIGN_MIN == 0);

            if T::ALIGN <= CHUNK_ALIGN_MIN {
                if T::ALIGN > MIN_ALIGN {
                    start = up_align_usize_unchecked(start, T::ALIGN);
                }

                let remaining = end - start;

                if T::SIZE > remaining {
                    return None;
                }
            } else {
                let aligned_down = (start - 1) & !(T::ALIGN - 1);
                let new_pos = aligned_down.saturating_add(T::ALIGN + T::SIZE);

                if new_pos > end {
                    return None;
                }

                start = aligned_down + T::ALIGN;
            };

            debug_assert!(is_aligned(start, T::ALIGN));

            unsafe { Some(self.with_addr(start)) }
        } else {
            let start = nonnull::addr(remaining.start).get();
            let mut end = nonnull::addr(remaining.end).get();

            debug_assert!(start <= end);

            if T::SIZE <= CHUNK_ALIGN_MIN {
                end -= T::SIZE;

                if T::ALIGN != MIN_ALIGN {
                    end = down_align_usize(end, T::ALIGN);
                }

                if end < start {
                    return None;
                }
            } else if T::ALIGN <= CHUNK_ALIGN_MIN {
                let remaining = end - start;

                if T::SIZE > remaining {
                    return None;
                }

                end -= T::SIZE;

                if T::ALIGN != MIN_ALIGN {
                    end = down_align_usize(end, T::ALIGN);
                }
            } else {
                end = end.saturating_sub(T::SIZE);
                end = down_align_usize(end, T::ALIGN);

                if end < start {
                    return None;
                }
            };

            debug_assert!(is_aligned(end, T::SIZE));

            unsafe { Some(self.with_addr(end)) }
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
    ///
    /// [`MutBumpVec`]: crate::MutBumpVec
    /// [`into_slice`]: crate::MutBumpVec::into_slice
    #[inline(always)]
    pub fn alloc_greedy<M, L>(self, _: M, layout: L) -> Option<Range<NonNull<u8>>>
    where
        M: SupportedMinimumAlignment,
        L: LayoutProps,
    {
        debug_assert_ne!(layout.size(), 0);

        if UP {
            let remaining = self.remaining_range();
            let mut start = nonnull::addr(remaining.start);
            let end = nonnull::addr(remaining.end);

            debug_assert!(start <= end);
            debug_assert!(end.get() % CHUNK_ALIGN_MIN == 0);

            if L::ALIGN_IS_CONST && layout.align() <= M::MIN_ALIGN {
                // alignment is already sufficient
            } else {
                // `start` needs to be aligned

                if L::ALIGN_IS_CONST && layout.align() <= CHUNK_ALIGN_MIN {
                    // SAFETY:
                    // Aligning an address that is `<= range.end` with an alignment
                    // that is `<= CHUNK_ALIGN_MIN` can not exceed `range.end` and
                    // can not overflow
                    start = unsafe { up_align_nonzero_unchecked(start, layout.align()) }
                } else {
                    start = up_align_nonzero(start, layout.align())?;
                }
            }

            let remaining = end.get() - start.get();

            if layout.size() > remaining {
                return None;
            }

            // layout does fit, we just trim off the excess to make end aligned
            let end = down_align_usize(end.get(), layout.align());

            debug_assert!(is_aligned(start.get(), layout.align()));
            debug_assert!(is_aligned(end, layout.align()));

            Some(unsafe { self.with_addr(start.get())..self.with_addr(end) })
        } else {
            let remaining = self.remaining_range();
            let start = nonnull::addr(remaining.start);
            let end = nonnull::addr(remaining.end);

            debug_assert!(start <= end);

            let mut end = end.get();

            if L::ALIGN_IS_CONST && layout.align() <= M::MIN_ALIGN {
                // alignment is already sufficient
            } else {
                end = down_align_usize(end, layout.align());

                if L::ALIGN_IS_CONST && layout.align() <= CHUNK_ALIGN_MIN {
                    // end is valid
                } else {
                    // end could be less than start at this point
                    if end < start.get() {
                        return None;
                    }
                }
            }

            let remaining = end - start.get();

            if layout.size() > remaining {
                return None;
            }

            // layout does fit, we just trim off the excess to make start aligned
            let start = up_align_usize_unchecked(start.get(), layout.align());

            debug_assert!(is_aligned(end, layout.align()));
            debug_assert!(is_aligned(start, layout.align()));
            Some(unsafe { self.with_addr(start)..self.with_addr(end) })
        }
    }

    #[inline(always)]
    pub fn align_pos_to<const ALIGN: usize>(self)
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

        unsafe { self.set_pos(self.with_addr(pos)) }
    }

    #[inline(always)]
    fn after_header(self) -> NonNull<u8> {
        unsafe { nonnull::add(self.header, 1).cast() }
    }

    #[inline(always)]
    pub fn chunk_start(self) -> NonNull<u8> {
        unsafe {
            if UP {
                self.header.cast()
            } else {
                self.header.as_ref().end
            }
        }
    }

    #[inline(always)]
    pub fn chunk_end(self) -> NonNull<u8> {
        unsafe {
            if UP {
                self.header.as_ref().end
            } else {
                self.after_header()
            }
        }
    }

    #[inline(always)]
    pub fn content_start(self) -> NonNull<u8> {
        if UP {
            self.after_header()
        } else {
            self.chunk_start()
        }
    }

    #[inline(always)]
    pub fn content_end(self) -> NonNull<u8> {
        if UP {
            self.chunk_end()
        } else {
            self.header.cast()
        }
    }

    #[inline(always)]
    pub fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref().pos.get() }
    }

    #[inline(always)]
    pub fn set_pos(mut self, ptr: NonNull<u8>) {
        unsafe { self.header.as_mut().pos.set(ptr) }
    }

    #[inline(always)]
    pub unsafe fn set_pos_addr(self, addr: usize) {
        let ptr = self.with_addr(addr);
        self.set_pos(ptr);
    }

    /// # Safety
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub unsafe fn with_addr(self, addr: usize) -> NonNull<u8> {
        debug_assert!(self.contains_addr_or_end(addr));
        let ptr = self.header.cast();
        let addr = NonZeroUsize::new_unchecked(addr);
        nonnull::with_addr(ptr, addr)
    }

    #[inline(always)]
    pub fn contains_addr_or_end(self, addr: usize) -> bool {
        let start = nonnull::addr(self.content_start()).get();
        let end = nonnull::addr(self.content_end()).get();
        addr >= start && addr <= end
    }

    #[inline(always)]
    pub fn prev(self) -> Option<Self> {
        unsafe { Some(Self::from_header(self.header.as_ref().prev?)) }
    }

    #[inline(always)]
    pub fn next(self) -> Option<Self> {
        unsafe { Some(Self::from_header(self.header.as_ref().next.get()?)) }
    }

    #[inline(always)]
    pub fn capacity(self) -> usize {
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
    pub fn allocated(self) -> usize {
        let range = self.allocated_range();
        let start = nonnull::addr(range.start).get();
        let end = nonnull::addr(range.end).get();
        end - start
    }

    #[inline(always)]
    pub fn remaining(self) -> usize {
        let range = self.remaining_range();
        let start = nonnull::addr(range.start).get();
        let end = nonnull::addr(range.end).get();
        end - start
    }

    pub fn remaining_range(self) -> Range<NonNull<u8>> {
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
    pub fn size(self) -> NonZeroUsize {
        let start = nonnull::addr(self.chunk_start()).get();
        let end = nonnull::addr(self.chunk_end()).get();
        unsafe { NonZeroUsize::new_unchecked(end - start) }
    }

    #[inline(always)]
    pub fn layout(self) -> Layout {
        // SAFETY: this layout fits the one we allocated, which means it must be valid
        unsafe { Layout::from_size_align_unchecked(self.size().get(), ChunkSize::<UP, A>::HEADER_ALIGN.get()) }
    }

    #[inline(always)]
    fn grow_size<E: ErrorBehavior>(self) -> Result<ChunkSize<UP, A>, E> {
        const TWO: NonZeroUsize = const_unwrap(NonZeroUsize::new(2));
        let size = match self.size().checked_mul(TWO) {
            Some(size) => size,
            None => return Err(E::capacity_overflow()),
        };
        ChunkSize::<UP, A>::new(size.get())
    }

    /// # Panic
    ///
    /// [`self.next`](RawChunk::next) must return `None`
    pub fn append_for<E: ErrorBehavior>(self, layout: Layout) -> Result<Self, E>
    where
        A: Allocator + Clone,
    {
        debug_assert!(self.next().is_none());

        let required_size = ChunkSize::for_capacity(layout)?;
        let grown_size = self.grow_size()?;
        let size = required_size.max(grown_size);

        let allocator = unsafe { self.header.as_ref().allocator.clone() };
        let new_chunk = RawChunk::new_in::<E>(size, Some(self), allocator)?;

        self.set_next(Some(new_chunk));
        Ok(new_chunk)
    }

    #[inline(always)]
    pub fn reset(self) {
        if UP {
            self.set_pos(self.content_start());
        } else {
            self.set_pos(self.content_end());
        }
    }

    /// # Safety
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub unsafe fn reset_to(self, addr: usize) {
        let ptr = self.with_addr(addr);
        self.set_pos(ptr);
    }

    /// # Safety
    /// self must not be used after calling this.
    pub unsafe fn deallocate(self)
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
    pub fn set_prev(mut self, value: Option<Self>) {
        unsafe {
            self.header.as_mut().prev = value.map(|c| c.header);
        }
    }

    #[inline(always)]
    pub fn set_next(mut self, value: Option<Self>) {
        unsafe {
            self.header.as_mut().next.set(value.map(|c| c.header));
        }
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](RawChunk::deallocate) on the chunk parameter of `f` is fine.
    pub fn for_each_prev(self, mut f: impl FnMut(Self)) {
        let mut iter = self.prev();

        while let Some(chunk) = iter {
            iter = chunk.prev();
            f(chunk);
        }
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](RawChunk::deallocate) on the chunk parameter of `f` is fine.
    pub fn for_each_next(self, mut f: impl FnMut(Self)) {
        let mut iter = self.next();

        while let Some(chunk) = iter {
            iter = chunk.next();
            f(chunk);
        }
    }

    #[inline(always)]
    pub fn allocator(self) -> NonNull<A> {
        unsafe { NonNull::from(&self.header.as_ref().allocator) }
    }

    #[inline(always)]
    pub fn without_allocator(self) -> RawChunk<UP, ()> {
        RawChunk {
            header: self.header.cast(),
        }
    }
}

#[must_use]
#[inline(always)]
fn is_aligned(addr: usize, align: usize) -> bool {
    assert!(align.is_power_of_two());
    addr & (align - 1) == 0
}
