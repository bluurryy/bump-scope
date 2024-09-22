use crate::{
    down_align_usize,
    polyfill::{const_unwrap, nonzero},
    up_align_nonzero, ChunkHeader, CHUNK_ALIGN_MIN,
};
use core::{
    alloc::Layout,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    mem::{align_of, size_of},
    num::NonZeroUsize,
};

/// We leave some space per allocation for the base allocator.
pub(crate) type AssumedMallocOverhead = [*const u8; 2];
pub(crate) const ASSUMED_PAGE_SIZE: NonZeroUsize = const_unwrap(NonZeroUsize::new(0x1000));

/// The actual size used for allocation (see [`layout`](Self::layout)) is this size minus <code>size_of::<[AssumedMallocOverhead]>()</code>.
///
/// Invariants:
/// - is never zero
/// - is a multiple of <code>align_of::<[`ChunkHeader<A>`](ChunkHeader)>()</code>.
/// - is at least [`Self::MIN`]
/// - if smaller than [`Self::SIZE_STEP`] it is a power of two
/// - if larger than [`Self::SIZE_STEP`] it is a multiple of [`Self::SIZE_STEP`]
pub(crate) struct ChunkSize<const UP: bool, A>(NonZeroUsize, PhantomData<*const A>);

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
    pub(crate) const DEFAULT_START: Self = const_unwrap(Self::new(512));

    pub(crate) const MIN: Self = Self(
        {
            let size = ByteSize(0);
            let size = const_unwrap(size.added::<AssumedMallocOverhead>());
            let size = const_unwrap(size.added::<ChunkHeader<A>>());
            const_unwrap(NonZeroUsize::new(size.0))
        },
        PhantomData,
    );

    pub(crate) const SIZE_STEP: NonZeroUsize = nonzero::max(ASSUMED_PAGE_SIZE, align_of::<ChunkHeader<A>>());

    #[inline]
    pub(crate) const fn new(size: usize) -> Option<Self> {
        let size = nonzero::max(Self::MIN.0, size);

        let size = if size.get() < Self::SIZE_STEP.get() {
            // the name is misleading, this will return `size` if it is already a power of two
            size.checked_next_power_of_two()
        } else {
            up_align_nonzero(size, Self::SIZE_STEP.get())
        };

        let size = match size {
            Some(some) => some,
            None => return None,
        };

        let size_for_layout = size.get() - size_of::<AssumedMallocOverhead>();
        let align = align_of::<ChunkHeader<A>>();

        // lets make sure we can create a layout from this size
        // so later on we can create a layout without checking
        if Layout::from_size_align(size_for_layout, align).is_err() {
            return None;
        }

        debug_assert!(size.get() % align_of::<ChunkHeader<A>>() == 0);
        debug_assert!(size.get() >= Self::MIN.0.get());

        debug_assert!(if size.get() < Self::SIZE_STEP.get() {
            size.is_power_of_two()
        } else {
            size.get() % Self::SIZE_STEP.get() == 0
        });

        Some(Self(size, PhantomData))
    }

    #[inline]
    pub(crate) fn for_capacity(layout: Layout) -> Option<Self> {
        let maximum_required_padding = layout.align().saturating_sub(align_of::<ChunkHeader<A>>());
        let required_size = layout.size().checked_add(maximum_required_padding)?;
        Self::for_capacity_bytes(required_size)
    }

    #[inline]
    fn for_capacity_bytes(bytes: usize) -> Option<Self> {
        let mut size = ByteSize(0);

        if UP {
            size.add::<AssumedMallocOverhead>().ok()?;
            size.add::<ChunkHeader<A>>().ok()?;
            size.add_bytes(bytes).ok()?;
        } else {
            size.add::<AssumedMallocOverhead>().ok()?;
            size.add_bytes(bytes).ok()?;
            size.add::<ChunkHeader<A>>().ok()?;
        }

        Self::new(size.0)
    }

    #[inline(always)]
    pub(crate) fn layout(self) -> Layout {
        // we checked in `new` that we can create a layout from this size

        let size_without_overhead = self.0.get() - size_of::<AssumedMallocOverhead>();

        let downwards_align = if UP {
            CHUNK_ALIGN_MIN
        } else {
            CHUNK_ALIGN_MIN.max(align_of::<ChunkHeader<A>>())
        };

        let size_for_layout = down_align_usize(size_without_overhead, downwards_align);
        let align = align_of::<ChunkHeader<A>>();

        unsafe { Layout::from_size_align_unchecked(size_for_layout, align) }
    }

    #[inline]
    pub(crate) const fn max(self, other: Self) -> Self {
        if self.0.get() > other.0.get() {
            self
        } else {
            other
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ByteSize(pub(crate) usize);

impl ByteSize {
    pub(crate) fn add<T>(&mut self) -> Result<(), ()> {
        *self = self.added::<T>().ok_or(())?;
        Ok(())
    }

    pub(crate) fn add_bytes(&mut self, count: usize) -> Result<(), ()> {
        *self = self.added_bytes(count).ok_or(())?;
        Ok(())
    }

    pub(crate) const fn added<T>(self) -> Option<Self> {
        self.added_layout(Layout::new::<T>())
    }

    pub(crate) const fn added_layout(mut self, layout: Layout) -> Option<Self> {
        self.0 = match up_align(self.0, layout.align()) {
            Some(some) => some,
            None => return None,
        };

        self.0 = match self.0.checked_add(layout.size()) {
            Some(some) => some,
            None => return None,
        };

        Some(self)
    }

    pub(crate) const fn added_bytes(mut self, count: usize) -> Option<Self> {
        self.0 = match self.0.checked_add(count) {
            Some(some) => some,
            None => return None,
        };

        Some(self)
    }
}

#[inline(always)]
const fn up_align(addr: usize, align: usize) -> Option<usize> {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;

    let addr_plus_mask = match addr.checked_add(mask) {
        Some(some) => some,
        None => return None,
    };

    let aligned = addr_plus_mask & !mask;
    Some(aligned)
}
