use core::{
    alloc::Layout,
    cell::Cell,
    marker::PhantomData,
    mem::ManuallyDrop,
    num::NonZeroUsize,
    ops::{Deref, Range},
    ptr::{self, NonNull},
};

use crate::{
    Checkpoint, SizedTypeProperties, align_pos,
    alloc::{AllocError, Allocator},
    bumping::{BumpProps, BumpUp, MIN_CHUNK_ALIGN, bump_down, bump_prepare_down, bump_prepare_up, bump_up},
    chunk::{ChunkHeader, ChunkSize, ChunkSizeHint},
    error_behavior::{self, ErrorBehavior},
    layout::{ArrayLayout, CustomLayout, LayoutProps, SizedLayout},
    polyfill::non_null,
    settings::{BumpAllocatorSettings, False, MinimumAlignment, SupportedMinimumAlignment, True},
    stats::Stats,
};

#[cfg(feature = "alloc")]
use crate::alloc::Global;

/// The internal type used by `Bump` and `Bump(Scope)`.
///
/// All the api that can fail due to allocation failure take a `E: ErrorBehavior`
/// instead of having a `try_` and non-`try_` version.
///
/// It does not concern itself with freeing chunks or the base allocator.
/// A clone of this type is just a bitwise copy, `manually_drop` must only be called
/// once for this bump allocator.
pub(crate) struct RawBump<A, S> {
    /// Either a chunk allocated from the `allocator`, or either a `CLAIMED`
    /// or `UNALLOCATED` dummy chunk.
    pub(crate) chunk: Cell<RawChunk<S>>,

    /// The base allocator.
    pub(crate) allocator: ManuallyDrop<A>,
}

impl<A, S> Clone for RawBump<A, S> {
    fn clone(&self) -> Self {
        Self {
            chunk: self.chunk.clone(),
            allocator: unsafe { ptr::read(&raw const self.allocator) },
        }
    }
}

impl<A, S> RawBump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    pub(crate) const fn new(allocator: A) -> Self {
        const { assert!(!S::GUARANTEED_ALLOCATED) };

        Self {
            chunk: Cell::new(RawChunk::UNALLOCATED),
            allocator: ManuallyDrop::new(allocator),
        }
    }

    #[inline(always)]
    pub(crate) fn with_size<E: ErrorBehavior>(size: ChunkSize<S::Up>, allocator: A) -> Result<Self, E> {
        Ok(Self {
            chunk: Cell::new(NonDummyChunk::new::<E>(size, None, &allocator)?.0),
            allocator: ManuallyDrop::new(allocator),
        })
    }

    #[inline(always)]
    pub(crate) fn is_claimed(&self) -> bool {
        self.chunk.get().is_claimed()
    }

    #[inline(always)]
    pub(crate) fn claim(&self) -> RawBump<A, S> {
        const {
            assert!(S::CLAIMABLE, "`claim` is only available with the setting `CLAIMABLE = true`");
        }

        #[cold]
        #[inline(never)]
        fn already_claimed() {
            panic!("bump allocator is already claimed");
        }

        if self.chunk.get().is_claimed() {
            already_claimed();
        }

        let chunk = Cell::new(self.chunk.replace(RawChunk::<S>::CLAIMED));
        let allocator = unsafe { ptr::read(&raw const self.allocator) };

        RawBump { chunk, allocator }
    }

    #[inline(always)]
    pub(crate) fn reclaim(&self, claimant: &RawBump<A, S>) {
        self.chunk.set(claimant.chunk.get());
    }

    #[inline(always)]
    pub(crate) fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk.get())
    }

    #[inline]
    pub(crate) unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        #[cfg(debug_assertions)]
        if checkpoint.chunk == ChunkHeader::claimed::<S>() || self.chunk.get().is_claimed() {
            error_behavior::panic::claimed();
        }

        // If the checkpoint was created when the bump allocator had no allocated chunk
        // then the chunk pointer will point to the unallocated chunk header.
        //
        // In such cases we reset the bump pointer to the very start of the very first chunk.
        //
        // We don't check if the chunk pointer points to the unallocated chunk header
        // if the bump allocator is `GUARANTEED_ALLOCATED`. We are allowed to not do this check
        // because of this safety condition of `reset_to`:
        // > the checkpoint must not have been created by an`!GUARANTEED_ALLOCATED` when self is `GUARANTEED_ALLOCATED`
        if !S::GUARANTEED_ALLOCATED && checkpoint.chunk == ChunkHeader::unallocated::<S>() {
            if let Some(mut chunk) = self.chunk.get().as_non_dummy() {
                while let Some(prev) = chunk.prev() {
                    chunk = prev;
                }

                chunk.reset();

                // SAFETY: casting from guaranteed-allocated to non-guaranteed-allocated is safe
                self.chunk.set(unsafe { chunk.cast() });
            }

            return;
        }

        debug_assert_ne!(
            checkpoint.chunk,
            ChunkHeader::unallocated::<S>(),
            "the safety conditions state that \"the checkpoint must not have been created by an`!GUARANTEED_ALLOCATED` when self is `GUARANTEED_ALLOCATED`\""
        );

        #[cfg(debug_assertions)]
        {
            let chunk = self
                .stats()
                .small_to_big()
                .find(|chunk| chunk.header() == checkpoint.chunk.cast())
                .expect("this checkpoint does not refer to any chunk of this bump allocator");

            assert!(
                chunk.chunk.contains_addr_or_end(checkpoint.address.get()),
                "checkpoint address does not point within its chunk"
            );
        }

        unsafe {
            checkpoint.reset_within_chunk();

            self.chunk.set(RawChunk {
                header: checkpoint.chunk.cast(),
                marker: PhantomData,
            });
        }
    }

    #[inline(always)]
    pub(crate) fn reset(&self) {
        let Some(mut chunk) = self.chunk.get().as_non_dummy() else {
            return;
        };

        unsafe {
            chunk.for_each_prev(|chunk| chunk.deallocate(&*self.allocator));

            while let Some(next) = chunk.next() {
                chunk.deallocate(&*self.allocator);
                chunk = next;
            }

            chunk.header.as_ref().prev.set(None);
        }

        chunk.reset();

        // SAFETY: casting from guaranteed-allocated to non-guaranteed-allocated is safe
        self.chunk.set(unsafe { chunk.cast() });
    }

    pub(crate) unsafe fn manually_drop(&mut self) {
        #[cold]
        #[inline(never)]
        fn panic_claimed() -> ! {
            panic!("tried to drop a `Bump` while it was still claimed")
        }

        match self.chunk.get().classify() {
            ChunkClass::Claimed => panic_claimed(),
            ChunkClass::Unallocated => (),
            ChunkClass::NonDummy(chunk) => unsafe {
                chunk.for_each_prev(|chunk| chunk.deallocate(&*self.allocator));
                chunk.for_each_next(|chunk| chunk.deallocate(&*self.allocator));
                chunk.deallocate(&*self.allocator);
            },
        }

        unsafe { ManuallyDrop::drop(&mut self.allocator) };
    }

    #[inline(always)]
    pub(crate) fn reserve_bytes<E: ErrorBehavior>(&self, additional: usize) -> Result<(), E> {
        let chunk = self.chunk.get();

        let Ok(layout) = Layout::from_size_align(additional, 1) else {
            return Err(E::capacity_overflow());
        };

        match chunk.classify() {
            ChunkClass::Claimed => Err(E::claimed()),
            ChunkClass::Unallocated => {
                let new_chunk = NonDummyChunk::new(
                    ChunkSize::<S::Up>::from_capacity(layout).ok_or_else(E::capacity_overflow)?,
                    None,
                    &*self.allocator,
                )?;

                self.chunk.set(new_chunk.0);
                Ok(())
            }
            ChunkClass::NonDummy(mut chunk) => {
                let mut additional = additional;

                loop {
                    // TODO: isn't this wrong?, check the `stats::Chunk::remaining` docs
                    if let Some(rest) = additional.checked_sub(chunk.remaining()) {
                        additional = rest;
                    } else {
                        return Ok(());
                    }

                    if let Some(next) = chunk.next() {
                        chunk = next;
                    } else {
                        break;
                    }
                }

                chunk.append_for(layout, &*self.allocator).map(drop)
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
        let cap = unsafe { non_null::byte_offset_from_unsigned(range.end, range.start) } / T::SIZE;

        let ptr = if S::UP { range.start } else { unsafe { range.end.sub(cap) } };

        Ok(NonNull::slice_from_raw_parts(ptr, cap))
    }

    /// Returns a pointer range.
    /// The start and end pointers are aligned.
    /// But `end - start` is *not* a multiple of `size_of::<T>()`.
    /// So `end.offset_from_unsigned(start)` may not be used!
    #[inline(always)]
    fn prepare_allocation_range<B: ErrorBehavior, T>(&self, cap: usize) -> Result<Range<NonNull<T>>, B> {
        // TODO: ZST to return dangling ptrs?

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
        unsafe { self.in_another_chunk(CustomLayout(layout), RawChunk::alloc) }
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

        unsafe { self.in_another_chunk(layout, RawChunk::prepare_allocation) }
    }

    #[cold]
    #[inline(never)]
    fn prepare_allocation_range_in_another_chunk<E: ErrorBehavior>(
        &self,
        layout: ArrayLayout,
    ) -> Result<Range<NonNull<u8>>, E> {
        unsafe { self.in_another_chunk(layout, RawChunk::prepare_allocation_range) }
    }

    /// # Safety
    ///
    /// `f` on the new chunk created by `RawChunk::append_for` with the layout `layout` must return `Some`.
    #[inline(always)]
    pub(crate) unsafe fn in_another_chunk<E: ErrorBehavior, R, L: LayoutProps>(
        &self,
        layout: L,
        mut f: impl FnMut(RawChunk<S>, L) -> Option<R>,
    ) -> Result<R, E> {
        let new_chunk: NonDummyChunk<S> = match self.chunk.get().classify() {
            ChunkClass::Claimed => Err(E::claimed()),
            ChunkClass::Unallocated => NonDummyChunk::new(
                ChunkSize::from_capacity(*layout).ok_or_else(E::capacity_overflow)?,
                None,
                &*self.allocator,
            ),
            ChunkClass::NonDummy(mut chunk) => {
                while let Some(next_chunk) = chunk.next() {
                    chunk = next_chunk;

                    // We don't reset the chunk position when we leave a scope, so we need to do it here.
                    chunk.reset();

                    self.chunk.set(chunk.0);

                    if let Some(ptr) = f(chunk.0, layout) {
                        return Ok(ptr);
                    }
                }

                // there is no chunk that fits, we need a new chunk
                chunk.append_for(*layout, &*self.allocator)
            }
        }?;

        self.chunk.set(new_chunk.0);

        match f(new_chunk.0, layout) {
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
        match self.chunk.get().classify() {
            ChunkClass::Claimed => Err(E::claimed()),
            ChunkClass::Unallocated => {
                let new_chunk = NonDummyChunk::new(
                    ChunkSize::from_hint(512).ok_or_else(E::capacity_overflow)?,
                    None,
                    &*self.allocator,
                )?;

                self.chunk.set(new_chunk.0);
                Ok(())
            }
            ChunkClass::NonDummy(_) => Ok(()),
        }
    }
}

impl<A, S> RawBump<A, S>
where
    S: BumpAllocatorSettings,
{
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats<'a>(&self) -> Stats<'a, S> {
        Stats::from_raw_chunk(self.chunk.get())
    }

    #[inline(always)]
    pub(crate) fn align<const ALIGN: usize>(&self)
    where
        MinimumAlignment<ALIGN>: SupportedMinimumAlignment,
    {
        self.align_to::<MinimumAlignment<ALIGN>>();
    }

    #[inline(always)]
    pub(crate) fn align_to<MinimumAlignment>(&self)
    where
        MinimumAlignment: SupportedMinimumAlignment,
    {
        if MinimumAlignment::VALUE > S::MIN_ALIGN {
            // a dummy chunk is always aligned
            if let Some(chunk) = self.chunk.get().as_non_dummy() {
                let pos = chunk.pos().addr().get();
                let addr = align_pos(S::UP, MinimumAlignment::VALUE, pos);
                unsafe { chunk.set_pos_addr(addr) };
            }
        }
    }

    pub(crate) fn ensure_satisfies_settings<NewS>(&self)
    where
        NewS: BumpAllocatorSettings,
    {
        const {
            assert!(NewS::UP == S::UP, "can't change `UP` setting");

            assert!(
                NewS::GUARANTEED_ALLOCATED <= S::GUARANTEED_ALLOCATED,
                "can't turn a non-guaranteed-allocated bump allocator into a guaranteed-allocated one"
            );

            assert!(
                NewS::CLAIMABLE >= S::CLAIMABLE,
                "can't turn a claimable bump allocator into a non-claimable one"
            );
        }

        self.align_to::<NewS::MinimumAlignment>();
    }

    #[expect(clippy::unused_self)]
    pub(crate) fn ensure_satisfies_settings_for_borrow<NewS>(&self)
    where
        NewS: BumpAllocatorSettings,
    {
        const {
            assert!(NewS::UP == S::UP, "can't change `UP` setting");

            assert!(
                NewS::GUARANTEED_ALLOCATED <= S::GUARANTEED_ALLOCATED,
                "can't turn a non-guaranteed-allocated bump allocator into a guaranteed-allocated one"
            );

            assert!(
                NewS::MIN_ALIGN == S::MIN_ALIGN,
                "can't change minimum alignment when borrowing with new settings"
            );

            assert!(NewS::CLAIMABLE == S::CLAIMABLE, "can't change claimability");
        }
    }

    pub(crate) fn ensure_satisfies_settings_for_borrow_mut<NewS>(&self)
    where
        NewS: BumpAllocatorSettings,
    {
        const {
            assert!(NewS::UP == S::UP, "can't change `UP` setting");

            assert!(
                NewS::GUARANTEED_ALLOCATED == S::GUARANTEED_ALLOCATED,
                "can't change guaranteed-allocated property when mutably borrowing with new settings"
            );

            assert!(
                NewS::MIN_ALIGN >= S::MIN_ALIGN,
                "can't decrease minimum alignment when mutably borrowing with new settings"
            );

            assert!(NewS::CLAIMABLE == S::CLAIMABLE, "can't change claimability");
        }

        self.align_to::<NewS::MinimumAlignment>();
    }
}

#[cfg(feature = "alloc")]
impl<S> RawBump<Global, S>
where
    S: BumpAllocatorSettings,
{
    #[inline]
    pub(crate) fn into_raw(self) -> NonNull<()> {
        self.chunk.get().header.cast()
    }

    #[inline]
    pub(crate) unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        Self {
            chunk: Cell::new(RawChunk {
                header: ptr.cast(),
                marker: PhantomData,
            }),
            allocator: ManuallyDrop::new(Global),
        }
    }
}

pub(crate) struct RawChunk<S> {
    pub(crate) header: NonNull<ChunkHeader>,
    pub(crate) marker: PhantomData<fn() -> S>,
}

impl<S> Clone for RawChunk<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for RawChunk<S> {}

pub(crate) struct NonDummyChunk<S>(RawChunk<S>);

impl<S> Copy for NonDummyChunk<S> {}

impl<S> Clone for NonDummyChunk<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Deref for NonDummyChunk<S> {
    type Target = RawChunk<S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> RawChunk<S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) const UNALLOCATED: Self = {
        assert!(!S::GUARANTEED_ALLOCATED);

        Self {
            header: ChunkHeader::unallocated::<S>(),
            marker: PhantomData,
        }
    };

    const CLAIMED: Self = {
        assert!(S::CLAIMABLE);

        Self {
            header: ChunkHeader::claimed::<S>(),
            marker: PhantomData,
        }
    };

    #[inline(always)]
    pub(crate) fn header(self) -> NonNull<ChunkHeader> {
        self.header
    }

    #[inline(always)]
    fn is_claimed(self) -> bool {
        S::CLAIMABLE && self.header == ChunkHeader::claimed::<S>()
    }

    #[inline(always)]
    pub(crate) fn is_unallocated(self) -> bool {
        !S::GUARANTEED_ALLOCATED && self.header == ChunkHeader::unallocated::<S>()
    }

    #[inline(always)]
    pub(crate) fn is_dummy(self) -> bool {
        self.is_claimed() || self.is_unallocated()
    }

    #[inline(always)]
    pub(crate) fn classify(self) -> ChunkClass<S> {
        if self.is_claimed() {
            return ChunkClass::Claimed;
        }

        if self.is_unallocated() {
            return ChunkClass::Unallocated;
        }

        ChunkClass::NonDummy(NonDummyChunk(self))
    }

    #[inline(always)]
    pub(crate) fn as_non_dummy(self) -> Option<NonDummyChunk<S>> {
        match self.classify() {
            ChunkClass::Claimed | ChunkClass::Unallocated => None,
            ChunkClass::NonDummy(chunk) => Some(chunk),
        }
    }

    /// Attempts to allocate a block of memory.
    ///
    /// On success, returns a [`NonNull<u8>`] meeting the size and alignment guarantees of `layout`.
    #[inline(always)]
    pub(crate) fn alloc(self, layout: impl LayoutProps) -> Option<NonNull<u8>> {
        let props = self.bump_props(layout);

        if S::UP {
            let BumpUp { new_pos, ptr } = bump_up(props)?;

            // SAFETY: allocations never succeed for a dummy chunk
            unsafe {
                let chunk = self.as_non_dummy_unchecked();
                chunk.set_pos_addr(new_pos);
                Some(chunk.content_ptr_from_addr(ptr))
            }
        } else {
            let ptr = bump_down(props)?;

            // SAFETY: allocations never succeed for a dummy chunk
            unsafe {
                let chunk = self.as_non_dummy_unchecked();
                chunk.set_pos_addr(ptr);
                Some(chunk.content_ptr_from_addr(ptr))
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

        let ptr = if S::UP { bump_up(props)?.ptr } else { bump_down(props)? };

        // SAFETY: allocations never succeed for a dummy chunk
        unsafe {
            let chunk = self.as_non_dummy_unchecked();
            Some(chunk.content_ptr_from_addr(ptr))
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
        let props = self.bump_props(layout);

        let range = if S::UP {
            bump_prepare_up(props)
        } else {
            bump_prepare_down(props)
        }?;

        // SAFETY: allocations never succeed for a dummy chunk
        unsafe {
            let chunk = self.as_non_dummy_unchecked();
            Some(chunk.content_ptr_from_addr_range(range))
        }
    }

    #[inline(always)]
    fn bump_props<L>(self, layout: L) -> BumpProps
    where
        L: LayoutProps,
    {
        let pos = self.pos().addr().get();
        let end = unsafe { self.header.as_ref() }.end.addr().get();

        let start = if S::UP { pos } else { end };
        let end = if S::UP { end } else { pos };

        #[cfg(debug_assertions)]
        if self.is_unallocated() {
            assert!(start > end);
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

    #[inline(always)]
    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref().pos.get() }
    }

    #[inline(always)]
    pub(crate) unsafe fn as_non_dummy_unchecked(self) -> NonDummyChunk<S> {
        debug_assert!(!self.is_dummy());
        NonDummyChunk(self)
    }

    /// Cast the settings.
    pub(crate) unsafe fn cast<S2>(self) -> RawChunk<S2> {
        RawChunk {
            header: self.header,
            marker: PhantomData,
        }
    }
}

impl<S> RawChunk<S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True, Claimable = False>,
{
    #[inline(always)]
    pub(crate) fn non_dummy(self) -> NonDummyChunk<S> {
        NonDummyChunk(self)
    }
}

// Methods only available for a non-dummy chunk.
impl<S> NonDummyChunk<S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) fn new<E>(
        chunk_size: ChunkSize<S::Up>,
        prev: Option<NonDummyChunk<S>>,
        allocator: &impl Allocator,
    ) -> Result<NonDummyChunk<S>, E>
    where
        E: ErrorBehavior,
    {
        let min_size = const {
            match ChunkSize::<S::Up>::from_hint(S::MINIMUM_CHUNK_SIZE) {
                Some(some) => some,
                None => panic!("failed to calculate minimum chunk size"),
            }
        };

        let layout = chunk_size.max(min_size).layout().ok_or_else(E::capacity_overflow)?;

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
                let header = ptr.cast::<ChunkHeader>();

                header.write(ChunkHeader {
                    pos: Cell::new(header.add(1).cast()),
                    end: ptr.add(size),
                    prev,
                    next,
                });

                header
            } else {
                let header = ptr.add(size).cast::<ChunkHeader>().sub(1);

                header.write(ChunkHeader {
                    pos: Cell::new(header.cast()),
                    end: ptr,
                    prev,
                    next,
                });

                header
            }
        };

        Ok(NonDummyChunk(RawChunk {
            header,
            marker: PhantomData,
        }))
    }

    /// # Panic
    ///
    /// [`self.next`](RawChunk::next) must return `None`
    pub(crate) fn append_for<B: ErrorBehavior>(self, layout: Layout, allocator: &impl Allocator) -> Result<Self, B> {
        debug_assert!(self.next().is_none());

        let required_size = ChunkSizeHint::for_capacity(layout).ok_or_else(B::capacity_overflow)?;
        let grown_size = self.grow_size()?;
        let size = required_size.max(grown_size).calc_size().ok_or_else(B::capacity_overflow)?;

        let new_chunk = Self::new::<B>(size, Some(self), allocator)?;

        unsafe {
            self.header.as_ref().next.set(Some(new_chunk.header));
        }

        Ok(new_chunk)
    }

    #[inline(always)]
    fn grow_size<B: ErrorBehavior>(self) -> Result<ChunkSizeHint<S::Up>, B> {
        let Some(size) = self.size().get().checked_mul(2) else {
            return Err(B::capacity_overflow());
        };

        Ok(ChunkSizeHint::new(size))
    }

    #[inline(always)]
    pub(crate) fn prev(self) -> Option<NonDummyChunk<S>> {
        unsafe {
            Some(NonDummyChunk(RawChunk {
                header: self.header.as_ref().prev.get()?,
                marker: PhantomData,
            }))
        }
    }

    #[inline(always)]
    pub(crate) fn next(self) -> Option<NonDummyChunk<S>> {
        unsafe {
            Some(NonDummyChunk(RawChunk {
                header: self.header.as_ref().next.get()?,
                marker: PhantomData,
            }))
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
    fn reset(self) {
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
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    pub(crate) unsafe fn set_pos(self, ptr: NonNull<u8>) {
        unsafe { self.set_pos_addr(ptr.addr().get()) };
    }

    /// # Safety
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
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
    /// [`contains_addr_or_end`](RawChunk::contains_addr_or_end) must return true
    #[inline(always)]
    unsafe fn content_ptr_from_addr(self, addr: usize) -> NonNull<u8> {
        unsafe {
            debug_assert!(self.contains_addr_or_end(addr));
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
    fn contains_addr_or_end(self, addr: usize) -> bool {
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
        unsafe { self.header.add(1).cast() }
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](NonDummyChunk::deallocate) on the chunk parameter of `f` is fine.
    fn for_each_prev(self, mut f: impl FnMut(NonDummyChunk<S>)) {
        let mut iter = self.prev();

        while let Some(chunk) = iter {
            iter = chunk.prev();
            f(chunk);
        }
    }

    /// This resolves the next chunk before calling `f`. So calling [`deallocate`](NonDummyChunk::deallocate) on the chunk parameter of `f` is fine.
    fn for_each_next(self, mut f: impl FnMut(NonDummyChunk<S>)) {
        let mut iter = self.next();

        while let Some(chunk) = iter {
            iter = chunk.next();
            f(chunk);
        }
    }

    /// # Safety
    /// - self must not be used after calling this.
    unsafe fn deallocate(self, allocator: &impl Allocator) {
        let ptr = self.chunk_start();
        let layout = self.layout();

        unsafe {
            allocator.deallocate(ptr, layout);
        }
    }

    #[inline(always)]
    pub(crate) fn layout(self) -> Layout {
        // SAFETY: this layout fits the one we allocated, which means it must be valid
        unsafe { Layout::from_size_align_unchecked(self.size().get(), align_of::<ChunkHeader>()) }
    }
}

pub(crate) enum ChunkClass<S: BumpAllocatorSettings> {
    Claimed,
    Unallocated,
    NonDummy(NonDummyChunk<S>),
}
