use core::{
    alloc::{Allocator, Layout},
    cell::Cell,
    ops::Range,
    panic,
    ptr::NonNull,
};

use crate::{
    Checkpoint, SizedTypeProperties, align_pos, cant_change_config_due_to_claimable,
    cant_change_config_due_to_const_constructible,
    chunk_size::{ChunkSize, ChunkSizeCapacity, ChunkSizeMin},
    chunk_types::{
        r#abstract::{AllocatedChunk, Chunk, ChunkKind, FirstAllocatedChunk},
        concrete::{ClaimedChunk, ConstConstructorChunk},
    },
    config::{BumpConfig, config},
    error_behavior::ErrorBehavior,
    layout::{ArrayLayout, CustomLayout, LayoutProps, SizedLayout},
    stats::Stats,
};

/// The internal type used by `Bump` and `Bump(Scope)`.
///
/// All the api that can fail due to allocation failure take a `E: ErrorBehavior`
/// instead of having a `try_` and non-`try_` version.
///
/// It does not concern itself with deallocating chunks or the base allocator.
/// A clone of this type is just a bitwise copy, `manually_drop` must only be called
/// once for this bump allocator.
pub(crate) struct RawBump<A, const C: BumpConfig> {
    /// Either a chunk allocated from the `allocator`, or either a `CLAIMED`
    /// or `UNALLOCATED` dummy chunk.
    pub(crate) chunk: Cell<Chunk<A, C>>,
}

impl<A, const C: BumpConfig> Clone for RawBump<A, C> {
    fn clone(&self) -> Self {
        Self {
            chunk: self.chunk.clone(),
        }
    }
}

impl<A, const C: BumpConfig> RawBump<A, C> {
    /// # Panics
    ///
    /// Panics if `self` is **not** [`const_constructible`].
    ///
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    #[inline(always)]
    pub(crate) const fn new() -> Self
    where
        A: Default + 'static,
    {
        assert!(config!(const_constructible));

        Self {
            chunk: Cell::new(Chunk::from_const_constructor(ConstConstructorChunk::VALUE)),
        }
    }

    /// # Safety
    ///
    /// - This method invalidates all memory blocks of this bump allocator
    #[expect(clippy::needless_pass_by_value)]
    pub unsafe fn replace(&self, new: Self) {
        self.chunk.set(new.chunk.get());
    }

    #[inline(always)]
    pub(crate) fn is_claimed(&self) -> bool {
        self.chunk.get().is_claimed()
    }

    /// The caller must ensure the returned reference is dead before calling [`manually_drop`](Self::manually_drop).
    #[inline(always)]
    pub(crate) fn allocator<'a>(&self) -> Option<&'a A> {
        match self.chunk.get().kind() {
            ChunkKind::Claimed(_) | ChunkKind::ConstConstructor(_) => None,
            ChunkKind::Allocated(chunk) => Some(unsafe { chunk.allocator_ptr().as_ref() }),
        }
    }
}

impl<A, const C: BumpConfig> RawBump<A, C>
where
    A: Allocator,
{
    #[inline(always)]
    pub(crate) fn with_size<E: ErrorBehavior>(size: impl ChunkSize, allocator: A) -> Result<Self, E> {
        Ok(Self {
            chunk: Cell::new(Chunk::from_first(FirstAllocatedChunk::new::<E>(size, allocator)?)),
        })
    }

    #[inline(always)]
    pub(crate) fn reset(&self) {
        let Some(first) = self.first_chunk() else {
            return;
        };

        // if it's just this chunk, then no deallocation takes place
        if first.next().is_none() {
            first.reset();
            return;
        }

        // extract the allocator
        let allocator = first.read_allocator();
        let mut chunk = AllocatedChunk::from_first(first);

        // deallocate all chunks except the last
        while let Some(next) = chunk.next() {
            let ptr = chunk.chunk_start();
            let layout = chunk.layout();
            unsafe { allocator.deallocate(ptr, layout) };
            chunk = next;
        }

        // setup the last chunk as the new "first allocated chunk"
        let new_first = unsafe {
            let ptr = chunk.chunk_start();
            let size = chunk.size().get();
            FirstAllocatedChunk::<A, C>::init(ptr, size, allocator)
        };

        self.chunk.set(Chunk::from_first(new_first));
    }

    /// # Safety
    /// - self must not be used after calling this.
    pub(crate) unsafe fn manually_drop(&mut self) {
        let Some(first) = self.first_chunk() else {
            return;
        };

        // extract the allocator
        let allocator = first.read_allocator();
        let mut chunk = AllocatedChunk::from_first(first);

        // deallocate all chunks
        loop {
            let next = chunk.next();

            let ptr = chunk.chunk_start();
            let layout = chunk.layout();
            unsafe { allocator.deallocate(ptr, layout) };

            if let Some(next) = next {
                chunk = next;
            } else {
                break;
            }
        }
    }
}

impl<A, const C: BumpConfig> RawBump<A, C> {
    #[inline(always)]
    fn first_chunk(&self) -> Option<FirstAllocatedChunk<A, C>> {
        let mut chunk = self.chunk.get().as_allocated()?;

        while let Some(prev) = chunk.prev() {
            chunk = prev;
        }

        // SAFETY: it's the first, duh
        let first = unsafe { FirstAllocatedChunk::assume_first(chunk) };
        Some(first)
    }

    #[inline(always)]
    pub(crate) fn claim(&self) -> RawBump<A, C> {
        assert!(config!(claimable), "`claim` is only available when `claimable` is enabled");

        #[cold]
        #[inline(never)]
        fn already_claimed() {
            panic!("bump allocator is already claimed");
        }

        if self.chunk.get().is_claimed() {
            already_claimed();
        }

        RawBump {
            chunk: Cell::new(self.chunk.replace(Chunk::<A, C>::from_claimed(ClaimedChunk::<A, C>::VALUE))),
        }
    }

    #[inline(always)]
    pub(crate) fn reclaim(&self, claimant: &RawBump<A, C>) {
        self.chunk.set(claimant.chunk.get());
    }

    #[inline(always)]
    pub(crate) fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk.get())
    }

    #[cfg_attr(not(debug_assertions), inline(always))]
    pub(crate) unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        if config!(claimable) && checkpoint.chunk.is_claimed() {
            // We do nothing if the checkpoint is claimed.
            // This state is reachable in safe code when a `BumpClaimGuard` is forgotten.
            return;
        }

        // If the checkpoint was created when the bump allocator had no allocated chunk
        // then the chunk pointer will point to the const constructor chunk.
        //
        // In such cases we reset the bump pointer to the very start of the very first chunk.
        //
        // We don't check if the chunk pointer points to the const constructor chunk if the bump allocator is **not** `const_constructible`.
        // We are allowed to not do this check because of this safety condition of `reset_to`:
        // > the checkpoint must not have been created by a `const_constructible` when self is **not** `const_constructible`
        if config!(const_constructible) && checkpoint.chunk.is_const_constructor() {
            self.reset_to_start();
            return;
        }

        #[cfg(debug_assertions)]
        {
            assert!(
                !checkpoint.chunk.is_const_constructor(),
                "the checkpoint must not have been created by a `const_constructible` when self is not `const_constructible`"
            );

            if !self.is_claimed() {
                let chunk = self
                    .stats()
                    .small_to_big()
                    .find(|chunk| chunk.header().addr() == checkpoint.chunk.as_ptr().addr())
                    .expect("this checkpoint does not refer to any chunk in this bump allocator");

                let start = chunk.content_start().addr().get();
                let end = chunk.content_end().addr().get();
                let addr = checkpoint.address.get();

                assert!(
                    addr >= start && addr <= end,
                    "checkpoint address is not in the chunk's content range"
                );
            }
        }

        // We handled the const constructor case above.
        // It can't be a claimed chunk because that would not be allowed due to the safety invariants.
        unsafe {
            // Reset the chunk position of the checkpoint chunk.
            let ptr = checkpoint.chunk.as_ptr().with_addr(checkpoint.address).cast();
            AllocatedChunk::<A, C>::from_ptr(checkpoint.chunk.as_ptr()).set_pos(ptr);

            self.chunk.set(Chunk::from_ptr(checkpoint.chunk.as_ptr()));
        }
    }

    /// Reset's the bump pointer to the very start.
    #[inline]
    pub(crate) fn reset_to_start(&self) {
        let Some(first) = self.first_chunk() else {
            return;
        };

        first.reset();

        self.chunk.set(Chunk::from_first(first));
    }
}

impl<A, const C: BumpConfig> RawBump<A, C>
where
    A: Allocator,
{
    #[inline(always)]
    pub(crate) fn reserve<E: ErrorBehavior>(&self, additional: usize) -> Result<(), E>
where {
        let chunk = self.chunk.get();

        match chunk.kind() {
            ChunkKind::Claimed(_) => Err(E::claimed()),
            ChunkKind::ConstConstructor(chunk) => {
                let Ok(layout) = Layout::from_size_align(additional, 1) else {
                    return Err(E::capacity_overflow());
                };

                let allocator = chunk.construct_base_allocator()?;
                let replacer = RawBump::with_size(ChunkSizeCapacity(layout), allocator)?;
                // SAFETY: the bump has no chunks, ergo no allocations to invalidate
                unsafe { self.replace(replacer) };
                Ok(())
            }
            ChunkKind::Allocated(mut chunk) => {
                let mut additional = additional;

                if let Some(rest) = additional.checked_sub(chunk.remaining()) {
                    additional = rest;
                } else {
                    return Ok(());
                }

                while let Some(next) = chunk.next() {
                    chunk = next;

                    if let Some(rest) = additional.checked_sub(chunk.capacity()) {
                        additional = rest;
                    } else {
                        return Ok(());
                    }
                }

                if additional == 0 {
                    return Ok(());
                }

                let Ok(layout) = Layout::from_size_align(additional, 1) else {
                    return Err(E::capacity_overflow());
                };

                AllocatedChunk::new(layout, chunk).map(drop)
            }
        }
    }

    #[inline(always)]
    pub(crate) fn alloc<B: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, B> {
        match self.chunk.get().alloc(CustomLayout(layout)) {
            Some(ptr) => Ok(ptr),
            None => self.alloc_in_another_chunk(layout),
        }
    }

    #[inline(always)]
    pub(crate) fn alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        match self.chunk.get().alloc(SizedLayout::new::<T>()) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.alloc_sized_in_another_chunk::<E, T>() {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline(always)]
    pub(crate) fn alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        let Ok(layout) = ArrayLayout::array::<T>(len) else {
            return Err(E::capacity_overflow());
        };

        match self.chunk.get().alloc(layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.alloc_slice_in_another_chunk::<E, T>(len) {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline(always)]
    pub(crate) fn alloc_slice_for<E: ErrorBehavior, T>(&self, value: &[T]) -> Result<NonNull<T>, E> {
        let layout = ArrayLayout::for_value(value);

        match self.chunk.get().alloc(layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.alloc_slice_in_another_chunk::<E, T>(value.len()) {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline(always)]
    pub(crate) fn prepare_sized_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B> {
        match self.chunk.get().prepare_allocation(SizedLayout::new::<T>()) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.prepare_allocation_in_another_chunk::<B, T>() {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline(always)]
    pub(crate) fn prepare_slice_allocation<B: ErrorBehavior, T>(&self, min_cap: usize) -> Result<NonNull<[T]>, B> {
        let range = self.prepare_allocation_range::<B, T>(min_cap)?;

        // NB: We can't use `offset_from_unsigned`, because the size is not a multiple of `T`'s.
        let cap = unsafe { range.end.byte_offset_from_unsigned(range.start) } / T::SIZE;

        let ptr = if config!(upwards) {
            range.start
        } else {
            unsafe { range.end.sub(cap) }
        };

        Ok(NonNull::slice_from_raw_parts(ptr, cap))
    }

    #[inline(always)]
    pub(crate) fn prepare_slice_allocation_rev<B: ErrorBehavior, T>(
        &self,
        min_cap: usize,
    ) -> Result<(NonNull<T>, usize), B> {
        let range = self.prepare_allocation_range::<B, T>(min_cap)?;

        // NB: We can't use `offset_from_unsigned`, because the size is not a multiple of `T`'s.
        let cap = unsafe { range.end.byte_offset_from_unsigned(range.start) } / T::SIZE;

        let end = if config!(upwards) {
            unsafe { range.start.add(cap) }
        } else {
            range.end
        };

        Ok((end, cap))
    }

    /// Returns a pointer range.
    /// The start and end pointers are aligned.
    /// But `end - start` is *not* a multiple of `size_of::<T>()`.
    /// So `end.offset_from_unsigned(start)` may not be used!
    #[inline(always)]
    fn prepare_allocation_range<B: ErrorBehavior, T>(&self, cap: usize) -> Result<Range<NonNull<T>>, B> {
        let Ok(layout) = ArrayLayout::array::<T>(cap) else {
            return Err(B::capacity_overflow());
        };

        let range = match self.chunk.get().prepare_allocation_range(layout) {
            Some(ptr) => ptr,
            None => self.prepare_allocation_range_in_another_chunk(layout)?,
        };

        Ok(range.start.cast::<T>()..range.end.cast::<T>())
    }

    /// Allocation slow path.
    /// The active chunk must *not* have space for `layout`.
    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        unsafe { self.in_another_chunk(CustomLayout(layout), Chunk::alloc) }
    }

    #[cold]
    #[inline(never)]
    fn alloc_sized_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E> {
        self.alloc_in_another_chunk(Layout::new::<T>())
    }

    #[cold]
    #[inline(never)]
    fn alloc_slice_in_another_chunk<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<u8>, E> {
        let Ok(layout) = Layout::array::<T>(len) else {
            return Err(E::capacity_overflow());
        };

        self.alloc_in_another_chunk(layout)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn prepare_allocation_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E> {
        let layout = CustomLayout(Layout::new::<T>());

        unsafe { self.in_another_chunk(layout, Chunk::prepare_allocation) }
    }

    #[cold]
    #[inline(never)]
    fn prepare_allocation_range_in_another_chunk<E: ErrorBehavior>(
        &self,
        layout: ArrayLayout,
    ) -> Result<Range<NonNull<u8>>, E> {
        unsafe { self.in_another_chunk(layout, Chunk::prepare_allocation_range) }
    }

    /// # Safety
    ///
    /// `f` on the new chunk created by `RawChunk::append_for` with the layout `layout` must return `Some`.
    #[inline(always)]
    pub(crate) unsafe fn in_another_chunk<E: ErrorBehavior, R, L: LayoutProps>(
        &self,
        layout: L,
        mut f: impl FnMut(Chunk<A, C>, L) -> Option<R>,
    ) -> Result<R, E> {
        let new_chunk: AllocatedChunk<A, C> = match self.chunk.get().kind() {
            ChunkKind::Claimed(_) => return Err(E::claimed()),
            ChunkKind::ConstConstructor(chunk) => {
                let allocator = chunk.construct_base_allocator()?;
                let replacer = RawBump::with_size(ChunkSizeCapacity(*layout), allocator)?;
                // SAFETY: the bump has no chunks, ergo no allocations to invalidate
                unsafe { self.replace(replacer) };
                // SAFETY: the `with_size` constructor is guaranteed to preduce an allocated chunk
                unsafe { self.chunk.get().as_allocated_unchecked() }
            }
            ChunkKind::Allocated(mut chunk) => {
                while let Some(next_chunk) = chunk.next() {
                    chunk = next_chunk;

                    // We don't reset the chunk position when we leave a scope, so we need to do it here.
                    chunk.reset();

                    self.chunk.set(Chunk::from_allocated(chunk));

                    if let Some(ptr) = f(Chunk::from_allocated(chunk), layout) {
                        return Ok(ptr);
                    }
                }

                // there is no chunk that fits, we need a new chunk
                AllocatedChunk::new(*layout, chunk)?
            }
        };

        self.chunk.set(Chunk::from_allocated(new_chunk));

        match f(Chunk::from_allocated(new_chunk), layout) {
            Some(ptr) => Ok(ptr),
            _ => {
                // SAFETY: We just appended a chunk for that specific layout, it must have enough space.
                // We don't panic here so we don't produce any panic code when using `try_` apis.
                // We check for that in `test-no-panic`.
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
    }

    pub(crate) fn make_allocated<E: ErrorBehavior>(&self) -> Result<(), E> {
        match self.chunk.get().kind() {
            ChunkKind::Claimed(_) => Err(E::claimed()),
            ChunkKind::ConstConstructor(chunk) => {
                let allocator = chunk.construct_base_allocator()?;
                let replacer = RawBump::with_size(ChunkSizeMin, allocator)?;
                // SAFETY: the bump has no chunks, ergo no allocations to invalidate
                unsafe { self.replace(replacer) };
                Ok(())
            }
            ChunkKind::Allocated(_) => Ok(()),
        }
    }
}

impl<A, const C: BumpConfig> RawBump<A, C> {
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats<'a>(&self) -> Stats<'a, A, C> {
        Stats::from_raw_chunk(self.chunk.get())
    }

    #[inline(always)]
    pub(crate) fn align_to(&self, align: usize) {
        assert!(align.is_power_of_two() && align <= 16);

        if align > config!(min_align) {
            // a dummy chunk is always aligned
            if let Some(chunk) = self.chunk.get().as_allocated() {
                let pos = chunk.pos().addr().get();
                let addr = align_pos(config!(upwards), align, pos);
                unsafe { chunk.set_pos_addr(addr) };
            }
        }
    }

    pub(crate) const fn const_assert_upwards_matches<const NEW_C: BumpConfig>() {
        const {
            let old = config!(upwards);
            let new = config!(NEW_C, upwards);
            assert!(old == new, "can't change bump direction");
        }
    }

    fn can_convert_claimable<const NEW_C: BumpConfig>(&self) -> bool {
        // either nothing changed, the new config is claimable, the old one isn't, or the bump is currently not claimed
        (const {
            let old = config!(claimable);
            let new = config!(NEW_C, claimable);
            old == new || new || !old
        }) || !self.is_claimed()
    }

    pub(crate) fn assert_convertible_claimable<const NEW_C: BumpConfig>(&self) {
        if !self.can_convert_claimable::<NEW_C>() {
            cant_change_config_due_to_claimable();
        }
    }

    fn can_convert_const_constructible<const NEW_C: BumpConfig>(&self) -> bool {
        // To be able to convert const_constructible, either:
        // - nothing changed
        // - the new config is const_constructible
        // - the old config isn't const_constructible
        // - the bump currently has an allocated chunk
        (const {
            let old = config!(const_constructible);
            let new = config!(NEW_C, const_constructible);
            old == new || new || !old
        }) || self.stats().current_chunk().is_some()
    }

    pub(crate) fn assert_convertible_const_constructible<const NEW_C: BumpConfig>(&self) {
        if !self.can_convert_const_constructible::<NEW_C>() {
            cant_change_config_due_to_const_constructible();
        }
    }

    pub(crate) fn ensure_satisfies_config_for_borrow<const NEW_C: BumpConfig>(&self) {
        Self::const_assert_upwards_matches::<NEW_C>();

        const {
            assert!(
                config!(NEW_C, claimable) == config!(claimable),
                "can't change `claimable` using `Bump(Scope)::ref_change_config`"
            );

            assert!(
                config!(NEW_C, min_align) == config!(min_align),
                "can't change `min_align` using `Bump(Scope)::ref_change_config`"
            );
        }

        // Can't make this bump allocator's chunk be the const constructor chunk
        // from a shared reference so this is enough.
        self.assert_convertible_const_constructible::<NEW_C>();
    }

    pub(crate) fn ensure_satisfies_config_for_borrow_mut<const NEW_C: BumpConfig>(&self) {
        Self::const_assert_upwards_matches::<NEW_C>();

        const {
            assert!(
                config!(NEW_C, min_align) >= config!(min_align),
                "can't decrease `min_align` using `Bump(Scope)::mut_change_config`"
            );

            assert!(
                config!(NEW_C, claimable) == config!(claimable),
                "can't change `claimable` using `Bump(Scope)::mut_change_config`"
            );

            assert!(
                config!(NEW_C, const_constructible) == config!(const_constructible),
                "can't change `const_constructible` using `Bump(Scope)::mut_change_config`"
            );
        }

        self.align_to(config!(NEW_C, min_align));
    }

    #[inline]
    pub(crate) fn into_raw(self) -> NonNull<()> {
        self.chunk.get().as_ptr()
    }

    #[inline]
    pub(crate) unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        Self {
            chunk: Cell::new(unsafe { Chunk::from_ptr(ptr.cast()) }),
        }
    }
}
