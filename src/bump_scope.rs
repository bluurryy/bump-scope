#[cfg(not(no_global_oom_handling))]
use crate::infallible;
#[cfg(test)]
use crate::WithDrop;
use crate::{
    bump_align_guard::BumpAlignGuard,
    bump_common_methods, bump_scope_methods,
    chunk_size::ChunkSize,
    const_param_assert, doc_align_cant_decrease,
    layout::{ArrayLayout, CustomLayout, LayoutProps, SizedLayout},
    polyfill::{nonnull, pointer},
    BaseAllocator, BumpScopeGuard, Checkpoint, ErrorBehavior, GuaranteedAllocatedStats, MinimumAlignment, NoDrop, RawChunk,
    SizedTypeProperties, Stats, SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};
use allocator_api2::alloc::AllocError;
use core::{
    alloc::Layout,
    cell::Cell,
    fmt::{self, Debug},
    marker::PhantomData,
    mem::ManuallyDrop,
    num::NonZeroUsize,
    ops::Range,
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

macro_rules! bump_scope_declaration {
    ($($allocator_parameter:tt)*) => {
        /// A bump allocation scope whose allocations are valid for the lifetime of its associated [`BumpScopeGuard`] or closure.
        ///
        /// Alternatively a [`Bump`] can be turned into a `BumpScope` with [`as_scope`], [`as_mut_scope`] and `into`.
        ///
        /// [You can see examples in the crate documentation.](crate#scopes)
        ///
        /// [`Bump`]: crate::Bump
        /// [`as_scope`]: crate::Bump::as_scope
        /// [`as_mut_scope`]: crate::Bump::as_mut_scope
        #[repr(transparent)]
        pub struct BumpScope<
            'a,
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        > {
            pub(crate) chunk: Cell<RawChunk<UP, A>>,

            /// Marks the lifetime of the mutably borrowed `BumpScopeGuard(Root)`.
            marker: PhantomData<&'a ()>,
        }
    };
}

crate::maybe_default_allocator!(bump_scope_declaration);

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> UnwindSafe
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + UnwindSafe,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> RefUnwindSafe
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + UnwindSafe,
{
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Debug
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.stats().debug_format("BumpScope", f)
    }
}

/// These functions are only available if the `BumpScope` is [guaranteed allocated](crate#guaranteed_allocated-parameter).
impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScope<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<true>,
{
    bump_scope_methods!(BumpScopeGuard, true);
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<UP, A>) -> Self {
        Self {
            chunk: Cell::new(chunk),
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) fn ensure_allocated<E: ErrorBehavior>(&self) -> Result<(), E> {
        if self.is_unallocated() {
            self.allocate_first_chunk()?;
        }

        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn allocate_first_chunk<B: ErrorBehavior>(&self) -> Result<(), B> {
        // must only be called when we point to the empty chunk
        debug_assert!(self.chunk.get().is_unallocated());

        let allocator = A::default_or_panic();
        let chunk = RawChunk::new_in(ChunkSize::DEFAULT_START, None, allocator)?;

        self.chunk.set(chunk);

        Ok(())
    }

    #[inline(always)]
    pub(crate) unsafe fn consolidate_greed<T>(&mut self, mut start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        let end = nonnull::add(start, len);

        if UP {
            self.set_pos(nonnull::addr(end), T::ALIGN);
            nonnull::slice_from_raw_parts(start, len)
        } else {
            {
                let dst_end = nonnull::add(start, cap);
                let dst = nonnull::sub(dst_end, len);

                nonnull::copy(start, dst, len);
                start = dst;
            }

            self.set_pos(nonnull::addr(start), T::ALIGN);
            nonnull::slice_from_raw_parts(start, len)
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn consolidate_greed_rev<T>(&self, mut end: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        let mut start = nonnull::sub(end, len);

        if UP {
            {
                let dst = nonnull::sub(end, cap);
                let dst_end = nonnull::add(dst, len);

                nonnull::copy(start, dst, len);
                start = dst;
                end = dst_end;
            }

            self.set_pos(nonnull::addr(end), T::ALIGN);
            nonnull::slice_from_raw_parts(start, len)
        } else {
            self.set_pos(nonnull::addr(start), T::ALIGN);
            nonnull::slice_from_raw_parts(start, len)
        }
    }

    #[inline(always)]
    fn set_pos(&self, pos: NonZeroUsize, current_align: usize) {
        let chunk = self.chunk.get();
        debug_assert_eq!(pos.get() % current_align, 0);

        unsafe { chunk.set_pos_addr(pos.get()) }

        if current_align < MIN_ALIGN {
            chunk.align_pos_to::<MIN_ALIGN>();
        }
    }

    #[inline(always)]
    pub(crate) fn alloc_greedy<B: ErrorBehavior, T>(&mut self, cap: usize) -> Result<(NonNull<T>, usize), B> {
        let Range { start, end } = self.alloc_greedy_range::<B, T>(cap)?;

        // NB: We can't use `sub_ptr`, because the size is not a multiple of `T`'s.
        let capacity = unsafe { nonnull::byte_sub_ptr(end, start) } / T::SIZE;

        Ok((start, capacity))
    }

    #[inline(always)]
    pub(crate) fn alloc_greedy_rev<B: ErrorBehavior, T>(&mut self, cap: usize) -> Result<(NonNull<T>, usize), B> {
        let Range { start, end } = self.alloc_greedy_range::<B, T>(cap)?;

        // NB: We can't use `sub_ptr`, because the size is not a multiple of `T`'s.
        let capacity = unsafe { nonnull::byte_sub_ptr(end, start) } / T::SIZE;

        Ok((end, capacity))
    }

    /// Returns a pointer range.
    /// The start and end pointers are aligned.
    /// But `end - start` is *not* a multiple of `size_of::<T>()`.
    /// So `end.sub_ptr(start)` may not be used!
    #[inline(always)]
    fn alloc_greedy_range<B: ErrorBehavior, T>(&mut self, cap: usize) -> Result<Range<NonNull<T>>, B> {
        let layout = match ArrayLayout::array::<T>(cap) {
            Ok(ok) => ok,
            Err(_) => return Err(B::capacity_overflow()),
        };

        let range = match self.chunk.get().alloc_greedy(MinimumAlignment::<MIN_ALIGN>, layout) {
            Some(ptr) => ptr,
            None => self.alloc_greedy_in_another_chunk(*layout)?,
        };

        Ok(range.start.cast::<T>()..range.end.cast::<T>())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_greedy_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<Range<NonNull<u8>>, E> {
        let layout = CustomLayout(layout);
        unsafe {
            self.do_custom_alloc_in_another_chunk(layout, |chunk, layout| {
                chunk.alloc_greedy(MinimumAlignment::<MIN_ALIGN>, layout)
            })
        }
    }

    #[inline(always)]
    pub(crate) fn alloc_in_current_chunk(&self, layout: Layout) -> Option<NonNull<u8>> {
        self.chunk.get().alloc(MinimumAlignment::<MIN_ALIGN>, CustomLayout(layout))
    }

    /// Allocation slow path.
    /// The active chunk must *not* have space for `layout`.
    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        unsafe {
            self.do_custom_alloc_in_another_chunk(CustomLayout(layout), |chunk, layout| {
                chunk.alloc(MinimumAlignment::<MIN_ALIGN>, layout)
            })
        }
    }

    /// Reserve slow path.
    /// The active chunk must *not* have space for `layout`.
    #[cold]
    #[inline(never)]
    pub(crate) fn reserve_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        unsafe {
            self.do_custom_alloc_in_another_chunk(CustomLayout(layout), |chunk, layout| {
                chunk.reserve(MinimumAlignment::<MIN_ALIGN>, layout)
            })
        }
    }

    #[inline(always)]
    pub(crate) fn do_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        E::alloc_or_else(
            self.chunk.get(),
            MinimumAlignment::<MIN_ALIGN>,
            SizedLayout::new::<T>(),
            || self.do_alloc_sized_in_another_chunk::<E, T>(),
        )
        .map(NonNull::cast)
    }

    #[inline(always)]
    pub(crate) fn do_reserve_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        E::reserve_or_else(
            self.chunk.get(),
            MinimumAlignment::<MIN_ALIGN>,
            SizedLayout::new::<T>(),
            || self.do_reserve_sized_in_another_chunk::<E, T>(),
        )
        .map(NonNull::cast)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn do_alloc_sized_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.alloc_in_another_chunk(Layout::new::<T>())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn do_reserve_sized_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.reserve_in_another_chunk(Layout::new::<T>())
    }

    #[inline(always)]
    pub(crate) fn do_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        let layout = match ArrayLayout::array::<T>(len) {
            Ok(layout) => layout,
            Err(_) => return Err(E::capacity_overflow()),
        };

        E::alloc_or_else(self.chunk.get(), MinimumAlignment::<MIN_ALIGN>, layout, || unsafe {
            self.do_alloc_slice_in_another_chunk::<E, T>(len)
        })
        .map(NonNull::cast)
    }

    #[inline(always)]
    pub(crate) fn do_alloc_slice_for<E: ErrorBehavior, T>(&self, value: &[T]) -> Result<NonNull<T>, E> {
        let layout = ArrayLayout::for_value(value);

        E::alloc_or_else(self.chunk.get(), MinimumAlignment::<MIN_ALIGN>, layout, || unsafe {
            self.do_alloc_slice_in_another_chunk::<E, T>(value.len())
        })
        .map(NonNull::cast)
    }

    #[cold]
    #[inline(never)]
    pub(crate) unsafe fn do_alloc_slice_in_another_chunk<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<u8>, E>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        let layout = match Layout::array::<T>(len) {
            Ok(layout) => layout,
            Err(_) => return Err(E::capacity_overflow()),
        };

        self.alloc_in_another_chunk(layout)
    }

    #[inline(always)]
    pub(crate) fn align<const ALIGN: usize>(&self)
    where
        MinimumAlignment<ALIGN>: SupportedMinimumAlignment,
    {
        if ALIGN > MIN_ALIGN {
            self.chunk.get().align_pos_to::<ALIGN>();
        }
    }

    /// Will error at compile time if `NEW_MIN_ALIGN < MIN_ALIGN`.
    #[inline(always)]
    pub(crate) fn must_align_more<const NEW_MIN_ALIGN: usize>(&self)
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        const_param_assert! {
            (const MIN_ALIGN: usize, const NEW_MIN_ALIGN: usize) => NEW_MIN_ALIGN >= MIN_ALIGN, "`into_aligned` or `as_aligned_mut` can't decrease the minimum alignment"
        }

        self.align::<NEW_MIN_ALIGN>();
    }

    /// # Safety
    ///
    /// `allocate` on the new chunk created by `RawChunk::append_for` with the layout `layout` must return `Some`.
    #[inline(always)]
    pub(crate) unsafe fn do_custom_alloc_in_another_chunk<B: ErrorBehavior, L: LayoutProps, R>(
        &self,
        layout: L,
        mut allocate: impl FnMut(RawChunk<UP, A>, L) -> Option<R>,
    ) -> Result<R, B> {
        let new_chunk = if self.is_unallocated() {
            let allocator = A::default_or_panic();
            RawChunk::new_in(
                ChunkSize::for_capacity(*layout).ok_or_else(B::capacity_overflow)?,
                None,
                allocator,
            )
        } else {
            while let Some(chunk) = self.chunk.get().next() {
                // We don't reset the chunk position when we leave a scope, so we need to do it here.
                chunk.reset();

                self.chunk.set(chunk);

                if let Some(ptr) = allocate(chunk, layout) {
                    return Ok(ptr);
                }
            }

            // there is no chunk that fits, we need a new chunk
            self.chunk.get().append_for(*layout)
        }?;

        self.chunk.set(new_chunk);

        if let Some(ptr) = allocate(new_chunk, layout) {
            Ok(ptr)
        } else {
            // SAFETY: We just appended a chunk for that specific layout, it must have enough space.
            // We don't panic here so we don't produce any panic code when using `try_` apis.
            // We check for that in `test-fallibility`.
            core::hint::unreachable_unchecked()
        }
    }

    bump_common_methods!(true);

    /// Returns `&self` as is. This is useful for macros that support both `Bump` and `BumpScope`, like [`bump_vec!`](crate::bump_vec!).
    #[inline(always)]
    pub fn as_scope(&self) -> &Self {
        self
    }

    /// Returns `&mut self` as is. This is useful for macros that support both `Bump` and `BumpScope`, like [`mut_bump_vec!`](crate::mut_bump_vec!).
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut Self {
        self
    }

    /// Converts this `BumpScope` into a `BumpScope` with a new minimum alignment.
    ///
    #[doc = doc_align_cant_decrease!()]
    ///
    /// <details>
    /// <summary>This mustn't decrease alignment because otherwise you could do this:</summary>
    ///
    /// ```ignore
    /// # // We can't `compile_fail,E0080` this doc test because it does not do the compile step
    /// # // that triggers the error.
    /// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
    /// # use bump_scope::{ Bump, allocator_api2::alloc::Global };
    /// let mut bump: Bump<Global, 8, true> = Bump::new();
    /// let mut guard = bump.scope_guard();
    ///
    /// {
    ///     let scope = guard.scope().into_aligned::<1>();
    ///     scope.alloc(0u8);
    /// }
    ///
    /// {
    ///     let scope = guard.scope();
    ///     // scope is not aligned to `MIN_ALIGN`!!
    /// }
    ///
    /// ```
    /// </details>
    #[inline(always)]
    pub fn into_aligned<const NEW_MIN_ALIGN: usize>(self) -> BumpScope<'a, A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align() }
    }

    /// Mutably borrows `BumpScope` with a new minimum alignment.
    ///
    #[doc = doc_align_cant_decrease!()]
    #[inline(always)]
    pub fn as_aligned_mut<const NEW_MIN_ALIGN: usize>(
        &mut self,
    ) -> &mut BumpScope<'a, A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align_mut() }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align<const NEW_MIN_ALIGN: usize>(
        self,
    ) -> BumpScope<'a, A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        BumpScope {
            chunk: self.chunk,
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align_mut<const NEW_MIN_ALIGN: usize>(
        &mut self,
    ) -> &mut BumpScope<'a, A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        &mut *pointer::from_mut(self).cast::<BumpScope<'a, A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>()
    }

    /// Converts this `BumpScope` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `BumpScope`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[cfg(not(no_global_oom_handling))]
    pub fn into_guaranteed_allocated(self) -> BumpScope<'a, A, MIN_ALIGN, UP> {
        infallible(self.generic_into_guaranteed_allocated())
    }

    /// Converts this `BumpScope` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `BumpScope`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    pub fn try_into_guaranteed_allocated(self) -> Result<BumpScope<'a, A, MIN_ALIGN, UP>, AllocError> {
        self.generic_into_guaranteed_allocated()
    }

    fn generic_into_guaranteed_allocated<E: ErrorBehavior>(self) -> Result<BumpScope<'a, A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { self.cast_allocated() })
    }

    /// Borrows `BumpScope` in a [guaranteed allocated](crate#guaranteed_allocated-parameter) state.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[cfg(not(no_global_oom_handling))]
    pub fn as_guaranteed_allocated(&self) -> &BumpScope<'a, A, MIN_ALIGN, UP> {
        infallible(self.generic_as_guaranteed_allocated())
    }

    /// Borrows `BumpScope` in a [guaranteed allocated](crate#guaranteed_allocated-parameter) state.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    pub fn try_as_guaranteed_allocated(&self) -> Result<&BumpScope<'a, A, MIN_ALIGN, UP>, AllocError> {
        self.generic_as_guaranteed_allocated()
    }

    fn generic_as_guaranteed_allocated<E: ErrorBehavior>(&self) -> Result<&BumpScope<'a, A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { self.cast_allocated_ref() })
    }

    /// Mutably borrows `BumpScope` in a [guaranteed allocated](crate#guaranteed_allocated-parameter) state.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[cfg(not(no_global_oom_handling))]
    pub fn as_guaranteed_allocated_mut(&mut self) -> &mut BumpScope<'a, A, MIN_ALIGN, UP> {
        infallible(self.generic_as_guaranteed_allocated_mut())
    }

    /// Mutably borrows `BumpScope` in a [guaranteed allocated](crate#guaranteed_allocated-parameter) state.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    pub fn try_as_guaranteed_allocated_mut(&mut self) -> Result<&mut BumpScope<'a, A, MIN_ALIGN, UP>, AllocError> {
        self.generic_as_guaranteed_allocated_mut()
    }

    fn generic_as_guaranteed_allocated_mut<E: ErrorBehavior>(&mut self) -> Result<&mut BumpScope<'a, A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { self.cast_allocated_mut() })
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_allocated(self) -> BumpScope<'a, A, MIN_ALIGN, UP> {
        BumpScope {
            chunk: self.chunk,
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_allocated_ref(&self) -> &BumpScope<'a, A, MIN_ALIGN, UP> {
        &*pointer::from_ref(self).cast::<BumpScope<'a, A, MIN_ALIGN, UP>>()
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_allocated_mut(&mut self) -> &mut BumpScope<'a, A, MIN_ALIGN, UP> {
        &mut *pointer::from_mut(self).cast::<BumpScope<'a, A, MIN_ALIGN, UP>>()
    }

    /// # Safety
    ///
    /// - `self` must not be used until this clone is gone
    #[inline(always)]
    pub(crate) unsafe fn clone_unchecked(&self) -> BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        BumpScope::new_unchecked(self.chunk.get())
    }

    /// Converts this `BumpScope` into a raw pointer.
    #[inline]
    #[must_use]
    pub fn into_raw(self) -> NonNull<()> {
        let this = ManuallyDrop::new(self);
        this.chunk.get().header_ptr().cast()
    }

    /// Converts the raw pointer that was created with [`into_raw`](Self::into_raw) back into a `BumpScope`.
    ///
    /// # Safety
    /// This is highly unsafe, due to the number of invariants that aren't checked:
    /// - `ptr` must have been created with `Self::into_raw`.
    /// - This function must only be called once with this `ptr`.
    /// - The lifetime must be the original one.
    /// - Nothing must have been allocated since then.
    #[inline]
    #[must_use]
    pub unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        let chunk = Cell::new(RawChunk::from_header(ptr.cast()));
        Self {
            chunk,
            marker: PhantomData,
        }
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> NoDrop for BumpScope<'_, A, MIN_ALIGN, UP> {}
