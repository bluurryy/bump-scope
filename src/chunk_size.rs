use core::{
    alloc::Layout,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    mem,
    num::NonZeroUsize,
    ptr::NonNull,
};

use crate::{
    down_align_usize,
    polyfill::{const_unwrap, nonnull, nonzero},
    up_align_nonzero, up_align_usize, ChunkHeader, ErrorBehavior, CHUNK_ALIGN_MIN,
};

use allocator_api2::alloc::Allocator;

/// We leave some space per allocation for the base allocator.
type AssumedMallocOverhead = [*const u8; 2];
pub(crate) const ASSUMED_MALLOC_OVERHEAD_SIZE: NonZeroUsize =
    const_unwrap(NonZeroUsize::new(mem::size_of::<AssumedMallocOverhead>()));
pub(crate) const ASSUMED_PAGE_SIZE: NonZeroUsize = const_unwrap(NonZeroUsize::new(0x1000));

/// It is a multiple of `align_of` [`ChunkHeader<A>`].
/// It is at least `size_of` [`ChunkHeader<A>`].
/// If smaller than [`ASSUMED_PAGE_SIZE`] it is a power of two,
/// otherwise it is aligned to [`ASSUMED_PAGE_SIZE`].
/// It is never zero.
pub struct ChunkSize<const UP: bool, A>(NonZeroUsize, PhantomData<*const A>);

impl<const UP: bool, A> Debug for ChunkSize<UP, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.0.get(), f)
    }
}

impl<const UP: bool, A> Clone for ChunkSize<UP, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const UP: bool, A> Copy for ChunkSize<UP, A> {}

impl<const UP: bool, A> ChunkSize<UP, A> {
    pub const OVERHEAD: NonZeroUsize = const_unwrap(ASSUMED_MALLOC_OVERHEAD_SIZE.checked_add(Self::HEADER_SIZE.get()));
    pub const MIN: NonZeroUsize = const_unwrap(up_align_nonzero(Self::OVERHEAD, Self::HEADER_ALIGN.get()));

    pub const HEADER_LAYOUT: Layout = Layout::new::<ChunkHeader<A>>();
    pub const HEADER_SIZE: NonZeroUsize = const_unwrap(NonZeroUsize::new(Self::HEADER_LAYOUT.size()));
    pub const HEADER_ALIGN: NonZeroUsize = const_unwrap(NonZeroUsize::new(Self::HEADER_LAYOUT.align()));

    pub const PAGE_SIZE: NonZeroUsize = nonzero::max(ASSUMED_PAGE_SIZE, Self::HEADER_ALIGN.get());

    #[inline]
    pub(crate) fn new<E: ErrorBehavior>(size: usize) -> Result<Self, E> {
        let size = nonzero::max(Self::MIN, size);

        let size = if size.get() < Self::PAGE_SIZE.get() {
            size.checked_next_power_of_two()
        } else {
            up_align_nonzero(size, Self::PAGE_SIZE.get())
        };

        let size = match size {
            Some(size) => size,
            None => return Err(E::capacity_overflow()),
        };

        Ok(unsafe { Self::from_raw(size) })
    }

    #[inline]
    pub unsafe fn from_raw(size: NonZeroUsize) -> Self {
        debug_assert!(size.get() % Self::HEADER_ALIGN.get() == 0);
        debug_assert!(size >= Self::OVERHEAD);
        debug_assert!(if size < Self::PAGE_SIZE {
            size.is_power_of_two()
        } else {
            size.get() % Self::PAGE_SIZE.get() == 0
        });

        Self(size, PhantomData)
    }

    #[inline]
    pub(crate) fn for_capacity<E: ErrorBehavior>(layout: Layout) -> Result<Self, E> {
        let maximum_required_padding = layout.align().saturating_sub(Self::HEADER_ALIGN.get());
        let required_size = match layout.size().checked_add(maximum_required_padding) {
            Some(required_size) => required_size,
            None => return Err(E::capacity_overflow()),
        };
        Self::for_capacity_bytes(required_size)
    }

    #[inline]
    pub(crate) fn for_capacity_bytes<E: ErrorBehavior>(size: usize) -> Result<Self, E> {
        if UP {
            let size = match size.checked_add(Self::OVERHEAD.get()) {
                Some(size) => size,
                None => return Err(E::capacity_overflow()),
            };
            Self::new(size)
        } else {
            let size = match up_align_usize(size, Self::HEADER_ALIGN) {
                Some(size) => size,
                None => return Err(E::capacity_overflow()),
            };
            let size = match size.checked_add(Self::HEADER_SIZE.get()) {
                Some(size) => size,
                None => return Err(E::capacity_overflow()),
            };
            let size = match size.checked_add(ASSUMED_MALLOC_OVERHEAD_SIZE.get()) {
                Some(size) => size,
                None => return Err(E::capacity_overflow()),
            };
            Self::new(size)
        }
    }

    #[inline]
    pub const fn get(self) -> NonZeroUsize {
        self.0
    }

    #[inline(always)]
    pub(crate) fn layout<E: ErrorBehavior>(self) -> Result<Layout, E> {
        let size = self.layout_size();
        match Layout::from_size_align(size, Self::HEADER_ALIGN.get()) {
            Ok(layout) => Ok(layout),
            Err(_) => Err(E::capacity_overflow()),
        }
    }

    #[inline]
    pub fn layout_size(self) -> usize {
        let base_size = self.0.get();
        base_size - ASSUMED_MALLOC_OVERHEAD_SIZE.get()
    }

    #[inline]
    pub const fn max(self, other: Self) -> Self {
        if self.get().get() > other.get().get() {
            self
        } else {
            other
        }
    }

    #[inline]
    pub(crate) fn allocate<E: ErrorBehavior>(self, allocator: impl Allocator) -> Result<NonNull<[u8]>, E> {
        let layout = self.layout()?;

        let slice = match allocator.allocate(layout) {
            Ok(slice) => slice,
            _ => {
                return Err(E::allocation(layout));
            }
        };

        let size = slice.len();
        let ptr = slice.cast::<u8>();

        // `ptr + size` must be an aligned to `CHUNK_ALIGN_MIN`
        // if `!UP`, `ptr + size` must also be an aligned `*const ChunkHeader<_>`
        let down_alignment = if UP {
            CHUNK_ALIGN_MIN
        } else {
            CHUNK_ALIGN_MIN.max(Self::HEADER_ALIGN.get())
        };

        let truncated_size = down_align_usize(size, down_alignment);
        debug_assert!(truncated_size >= layout.size());

        let truncated_slice = nonnull::slice_from_raw_parts(ptr, truncated_size);
        Ok(truncated_slice)
    }
}
