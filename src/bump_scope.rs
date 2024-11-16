use crate::{
    bump_align_guard::BumpAlignGuard,
    bump_common_methods, bump_scope_methods,
    bumping::{bump_down, bump_up, BumpUp},
    chunk_size::ChunkSize,
    const_param_assert, down_align_usize,
    layout::{ArrayLayout, CustomLayout, LayoutProps, SizedLayout},
    polyfill::{nonnull, pointer, transmute_mut, transmute_ref},
    up_align_usize_unchecked, BaseAllocator, BumpBox, BumpScopeGuard, BumpString, BumpVec, Checkpoint, ErrorBehavior,
    FixedBumpString, FixedBumpVec, MinimumAlignment, MutBumpString, MutBumpVec, MutBumpVecRev, NoDrop, RawChunk,
    SizedTypeProperties, Stats, SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};
#[cfg(feature = "panic-on-alloc")]
use crate::{infallible, Infallibly};
use allocator_api2::alloc::AllocError;
use core::{
    alloc::Layout,
    cell::Cell,
    fmt::{self, Debug},
    marker::PhantomData,
    mem::{transmute, ManuallyDrop, MaybeUninit},
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
#[allow(clippy::needless_lifetimes)]
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
    pub(crate) unsafe fn use_reserved_allocation<T>(
        &mut self,
        mut start: NonNull<T>,
        len: usize,
        cap: usize,
    ) -> NonNull<[T]> {
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
    pub(crate) unsafe fn use_reserved_allocation_rev<T>(&self, mut end: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
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
    pub(crate) fn generic_prepare_slice_allocation<B: ErrorBehavior, T>(
        &mut self,
        cap: usize,
    ) -> Result<(NonNull<T>, usize), B> {
        let Range { start, end } = self.prepare_allocation_range::<B, T>(cap)?;

        // NB: We can't use `sub_ptr`, because the size is not a multiple of `T`'s.
        let capacity = unsafe { nonnull::byte_sub_ptr(end, start) } / T::SIZE;

        Ok((start, capacity))
    }

    #[inline(always)]
    pub(crate) fn generic_prepare_slice_allocation_rev<B: ErrorBehavior, T>(
        &mut self,
        cap: usize,
    ) -> Result<(NonNull<T>, usize), B> {
        let Range { start, end } = self.prepare_allocation_range::<B, T>(cap)?;

        // NB: We can't use `sub_ptr`, because the size is not a multiple of `T`'s.
        let capacity = unsafe { nonnull::byte_sub_ptr(end, start) } / T::SIZE;

        Ok((end, capacity))
    }

    /// Returns a pointer range.
    /// The start and end pointers are aligned.
    /// But `end - start` is *not* a multiple of `size_of::<T>()`.
    /// So `end.sub_ptr(start)` may not be used!
    #[inline(always)]
    fn prepare_allocation_range<B: ErrorBehavior, T>(&mut self, cap: usize) -> Result<Range<NonNull<T>>, B> {
        let layout = match ArrayLayout::array::<T>(cap) {
            Ok(ok) => ok,
            Err(_) => return Err(B::capacity_overflow()),
        };

        let range = match self.chunk.get().prepare_allocation(MinimumAlignment::<MIN_ALIGN>, layout) {
            Some(ptr) => ptr,
            None => self.prepare_allocation_in_another_chunk(*layout)?,
        };

        Ok(range.start.cast::<T>()..range.end.cast::<T>())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn prepare_allocation_in_another_chunk<E: ErrorBehavior>(
        &self,
        layout: Layout,
    ) -> Result<Range<NonNull<u8>>, E> {
        let layout = CustomLayout(layout);
        unsafe {
            self.do_custom_alloc_in_another_chunk(layout, |chunk, layout| {
                chunk.prepare_allocation(MinimumAlignment::<MIN_ALIGN>, layout)
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
    /// **This can not decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment."
    ///
    /// To decrease alignment we need to ensure that we return to our original alignment.
    /// That can only be guaranteed by a function taking a closure like the ones mentioned above.
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
    /// **This can not decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment."
    ///
    /// To decrease alignment we need to ensure that we return to our original alignment.
    /// That can only be guaranteed by a function taking a closure like the ones mentioned above.
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
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn guaranteed_allocated(self) -> BumpScope<'a, A, MIN_ALIGN, UP> {
        infallible(self.generic_guaranteed_allocated())
    }

    /// Converts this `BumpScope` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `BumpScope`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn try_guaranteed_allocated(self) -> Result<BumpScope<'a, A, MIN_ALIGN, UP>, AllocError> {
        self.generic_guaranteed_allocated()
    }

    #[inline(always)]
    fn generic_guaranteed_allocated<E: ErrorBehavior>(self) -> Result<BumpScope<'a, A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { transmute(self) })
    }

    /// Borrows `BumpScope` as a [guaranteed allocated](crate#guaranteed_allocated-parameter) `BumpScope`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn guaranteed_allocated_ref(&self) -> &BumpScope<'a, A, MIN_ALIGN, UP> {
        infallible(self.generic_guaranteed_allocated_ref())
    }

    /// Borrows `BumpScope` as a [guaranteed allocated](crate#guaranteed_allocated-parameter) `BumpScope`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn try_guaranteed_allocated_ref(&self) -> Result<&BumpScope<'a, A, MIN_ALIGN, UP>, AllocError> {
        self.generic_guaranteed_allocated_ref()
    }

    #[inline(always)]
    fn generic_guaranteed_allocated_ref<E: ErrorBehavior>(&self) -> Result<&BumpScope<'a, A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { transmute_ref(self) })
    }

    /// Mutably borrows `BumpScope` as a [guaranteed allocated](crate#guaranteed_allocated-parameter) `BumpScope`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn guaranteed_allocated_mut(&mut self) -> &mut BumpScope<'a, A, MIN_ALIGN, UP> {
        infallible(self.generic_guaranteed_allocated_mut())
    }

    /// Mutably borrows `BumpScope` as a [guaranteed allocated](crate#guaranteed_allocated-parameter) `BumpScope`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn try_guaranteed_allocated_mut(&mut self) -> Result<&mut BumpScope<'a, A, MIN_ALIGN, UP>, AllocError> {
        self.generic_guaranteed_allocated_mut()
    }

    #[inline(always)]
    fn generic_guaranteed_allocated_mut<E: ErrorBehavior>(&mut self) -> Result<&mut BumpScope<'a, A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { transmute_mut(self) })
    }

    /// Converts this `BumpScope` into a ***not*** [guaranteed allocated](crate#guaranteed_allocated-parameter) `BumpScope`.
    #[inline(always)]
    pub fn not_guaranteed_allocated(self) -> BumpScope<'a, A, MIN_ALIGN, UP, false> {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute(self) }
    }

    /// Borrows `BumpScope` as a ***not*** [guaranteed allocated](crate#guaranteed_allocated-parameter) `BumpScope`.
    ///
    /// Note that it's not possible to mutably borrow as a not guaranteed allocated bump allocator. That's because
    /// a user could `mem::swap` it with an actual unallocated bump allocator which in turn would make `&mut self`
    /// unallocated.
    #[inline(always)]
    pub fn not_guaranteed_allocated_ref(&self) -> &BumpScope<'a, A, MIN_ALIGN, UP, false> {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute_ref(self) }
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

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    pub(crate) fn generic_alloc<B: ErrorBehavior, T>(&self, value: T) -> Result<BumpBox<'a, T>, B> {
        self.generic_alloc_with(|| value)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_with<B: ErrorBehavior, T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<'a, T>, B> {
        if T::IS_ZST {
            let value = f();
            return Ok(BumpBox::zst(value));
        }

        let chunk = self.chunk.get();
        let props = chunk.bump_props(MinimumAlignment::<MIN_ALIGN>, crate::layout::SizedLayout::new::<T>());

        unsafe {
            let ptr = if UP {
                if let Some(BumpUp { new_pos, ptr }) = bump_up(props) {
                    chunk.set_pos_addr(new_pos);
                    chunk.with_addr(ptr)
                } else {
                    self.do_alloc_sized_in_another_chunk::<B, T>()?
                }
            } else {
                if let Some(addr) = bump_down(props) {
                    chunk.set_pos_addr(addr);
                    chunk.with_addr(addr)
                } else {
                    self.do_alloc_sized_in_another_chunk::<B, T>()?
                }
            };

            let ptr = ptr.cast::<T>();

            nonnull::write_with(ptr, f);
            Ok(BumpBox::from_raw(ptr))
        }
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_default<B: ErrorBehavior, T: Default>(&self) -> Result<BumpBox<'a, T>, B> {
        self.generic_alloc_with(Default::default)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_copy<B: ErrorBehavior, T: Copy>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice_clone(slice));
        }

        let len = slice.len();
        let src = slice.as_ptr();
        let dst = self.do_alloc_slice_for(slice)?;

        unsafe {
            core::ptr::copy_nonoverlapping(src, dst.as_ptr(), len);
            Ok(BumpBox::from_raw(nonnull::slice_from_raw_parts(dst, len)))
        }
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice_clone(slice));
        }

        Ok(self.generic_alloc_uninit_slice_for(slice)?.init_clone(slice))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_fill<B: ErrorBehavior, T: Clone>(
        &self,
        len: usize,
        value: T,
    ) -> Result<BumpBox<'a, [T]>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice_fill(len, value));
        }

        Ok(self.generic_alloc_uninit_slice(len)?.init_fill(value))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_fill_with<B: ErrorBehavior, T>(
        &self,
        len: usize,
        f: impl FnMut() -> T,
    ) -> Result<BumpBox<'a, [T]>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice_fill_with(len, f));
        }

        Ok(self.generic_alloc_uninit_slice(len)?.init_fill_with(f))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_str<B: ErrorBehavior>(&self, src: &str) -> Result<BumpBox<'a, str>, B> {
        let slice = self.generic_alloc_slice_copy(src.as_bytes())?;

        // SAFETY: input is `str` so this is too
        Ok(unsafe { BumpBox::from_utf8_unchecked(slice) })
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_fmt<B: ErrorBehavior>(&self, args: fmt::Arguments) -> Result<BumpBox<'a, str>, B> {
        if let Some(string) = args.as_str() {
            return self.generic_alloc_str(string);
        }

        let mut string = BumpString::new_in(self);

        let string = if B::IS_FALLIBLE {
            if fmt::Write::write_fmt(&mut string, args).is_err() {
                // Either the allocation failed or the formatting trait
                // implementation returned an error.
                // Either way we return an `AllocError`, it doesn't matter how.
                return Err(B::format_trait_error());
            }

            string
        } else {
            #[cfg(feature = "panic-on-alloc")]
            {
                let mut string = Infallibly(string);

                if fmt::Write::write_fmt(&mut string, args).is_err() {
                    // This can only be a formatting trait error.
                    // If allocation failed we'd have already panicked.
                    return Err(B::format_trait_error());
                }

                string.0
            }

            #[cfg(not(feature = "panic-on-alloc"))]
            {
                unreachable!()
            }
        };

        Ok(string.into_boxed_str())
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_fmt_mut<B: ErrorBehavior>(&mut self, args: fmt::Arguments) -> Result<BumpBox<'a, str>, B> {
        if let Some(string) = args.as_str() {
            return self.generic_alloc_str(string);
        }

        let mut string = MutBumpString::generic_with_capacity_in(0, self)?;

        let string = if B::IS_FALLIBLE {
            if fmt::Write::write_fmt(&mut string, args).is_err() {
                // Either the allocation failed or the formatting trait
                // implementation returned an error.
                // Either way we return an `AllocError`, it doesn't matter how.
                return Err(B::format_trait_error());
            }

            string
        } else {
            #[cfg(feature = "panic-on-alloc")]
            {
                let mut string = Infallibly(string);

                if fmt::Write::write_fmt(&mut string, args).is_err() {
                    // This can only be a formatting trait error.
                    // If allocation failed we'd have already panicked.
                    return Err(B::format_trait_error());
                }

                string.0
            }

            #[cfg(not(feature = "panic-on-alloc"))]
            {
                unreachable!()
            }
        };

        Ok(string.into_boxed_str())
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_iter<B: ErrorBehavior, T>(
        &self,
        iter: impl IntoIterator<Item = T>,
    ) -> Result<BumpBox<'a, [T]>, B> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = BumpVec::<T, &Self>::generic_with_capacity_in(capacity, self)?;

        for value in iter {
            vec.generic_push(value)?;
        }

        Ok(vec.into_boxed_slice())
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_iter_exact<B: ErrorBehavior, T, I>(
        &self,
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> Result<BumpBox<'a, [T]>, B>
    where
        I: ExactSizeIterator<Item = T>,
    {
        let mut iter = iter.into_iter();
        let len = iter.len();

        let mut vec = BumpVec::<T, &Self>::generic_with_capacity_in(len, self)?;

        while vec.len() != vec.capacity() {
            match iter.next() {
                // SAFETY: we checked above that `len != capacity`, so there is space
                Some(value) => unsafe { vec.unchecked_push(value) },
                None => break,
            }
        }

        Ok(vec.into_fixed_vec().into_boxed_slice())
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_iter_mut<B: ErrorBehavior, T>(
        &mut self,
        iter: impl IntoIterator<Item = T>,
    ) -> Result<BumpBox<'a, [T]>, B> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVec::<T, &mut Self>::generic_with_capacity_in(capacity, self)?;

        for value in iter {
            vec.generic_push(value)?;
        }

        Ok(vec.into_boxed_slice())
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_iter_mut_rev<B: ErrorBehavior, T>(
        &mut self,
        iter: impl IntoIterator<Item = T>,
    ) -> Result<BumpBox<'a, [T]>, B> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVecRev::<T, &mut Self>::generic_with_capacity_in(capacity, self)?;

        for value in iter {
            vec.generic_push(value)?;
        }

        Ok(vec.into_boxed_slice())
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_uninit<B: ErrorBehavior, T>(&self) -> Result<BumpBox<'a, MaybeUninit<T>>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::zst(MaybeUninit::uninit()));
        }

        let ptr = self.do_alloc_sized::<B, T>()?.cast::<MaybeUninit<T>>();
        unsafe { Ok(BumpBox::from_raw(ptr)) }
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_uninit_slice<B: ErrorBehavior, T>(
        &self,
        len: usize,
    ) -> Result<BumpBox<'a, [MaybeUninit<T>]>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::uninit_zst_slice(len));
        }

        let ptr = self.do_alloc_slice::<B, T>(len)?.cast::<MaybeUninit<T>>();

        unsafe {
            let ptr = nonnull::slice_from_raw_parts(ptr, len);
            Ok(BumpBox::from_raw(ptr))
        }
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_uninit_slice_for<B: ErrorBehavior, T>(
        &self,
        slice: &[T],
    ) -> Result<BumpBox<'a, [MaybeUninit<T>]>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::uninit_zst_slice(slice.len()));
        }

        let ptr = self.do_alloc_slice_for::<B, T>(slice)?.cast::<MaybeUninit<T>>();

        unsafe {
            let ptr = nonnull::slice_from_raw_parts(ptr, slice.len());
            Ok(BumpBox::from_raw(ptr))
        }
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_fixed_vec<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        Ok(FixedBumpVec::from_uninit(self.generic_alloc_uninit_slice(capacity)?))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_fixed_string<B: ErrorBehavior>(&self, capacity: usize) -> Result<FixedBumpString<'a>, B> {
        Ok(FixedBumpString::from_uninit(self.generic_alloc_uninit_slice(capacity)?))
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_layout<B: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, B> {
        match self.chunk.get().alloc(MinimumAlignment::<MIN_ALIGN>, CustomLayout(layout)) {
            Some(ptr) => Ok(ptr),
            None => self.alloc_in_another_chunk(layout),
        }
    }

    #[inline(always)]
    pub(crate) fn generic_reserve_bytes<B: ErrorBehavior>(&self, additional: usize) -> Result<(), B> {
        let layout = match Layout::from_size_align(additional, 1) {
            Ok(ok) => ok,
            Err(_) => return Err(B::capacity_overflow()),
        };

        if self.is_unallocated() {
            let allocator = A::default_or_panic();
            let new_chunk = RawChunk::new_in(
                ChunkSize::for_capacity(layout).ok_or_else(B::capacity_overflow)?,
                None,
                allocator,
            )?;
            self.chunk.set(new_chunk);
            return Ok(());
        }

        let mut additional = additional;
        let mut chunk = self.chunk.get();

        loop {
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

        chunk.append_for(layout).map(drop)
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScope<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    #[inline(always)]
    pub(crate) fn generic_alloc_try_with<B: ErrorBehavior, T, E>(
        &self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpBox<'a, T>, E>, B> {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(BumpBox::zst(value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let checkpoint_before_alloc = self.checkpoint();
        let uninit = self.generic_alloc_uninit::<B, Result<T, E>>()?;
        let ptr = BumpBox::into_raw(uninit).cast::<Result<T, E>>();

        // When bumping downwards the chunk's position is the same as `ptr`.
        // Using `ptr` is faster so we use that.
        let pos = if UP { self.chunk.get().pos() } else { ptr.cast() };

        Ok(unsafe {
            nonnull::write_with(ptr, f);

            // If `f` made allocations on this bump allocator we can't shrink the allocation.
            let can_shrink = pos == self.chunk.get().pos();

            match nonnull::result(ptr) {
                Ok(value) => Ok({
                    if can_shrink {
                        let new_pos = if UP {
                            let pos = nonnull::addr(nonnull::add(value, 1)).get();
                            up_align_usize_unchecked(pos, MIN_ALIGN)
                        } else {
                            let pos = nonnull::addr(value).get();
                            down_align_usize(pos, MIN_ALIGN)
                        };

                        self.chunk.get().set_pos_addr(new_pos);
                    }

                    BumpBox::from_raw(value)
                }),
                Err(error) => Err({
                    let error = error.as_ptr().read();

                    if can_shrink {
                        self.reset_to(checkpoint_before_alloc);
                    }

                    error
                }),
            }
        })
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_try_with_mut<B: ErrorBehavior, T, E>(
        &mut self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpBox<'a, T>, E>, B> {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(BumpBox::zst(value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let checkpoint = self.checkpoint();
        let ptr = self.do_reserve_sized::<B, Result<T, E>>()?;

        Ok(unsafe {
            nonnull::write_with(ptr, f);

            // There is no need for `can_shrink` checks, because we have a mutable reference
            // so there's no way anyone else has allocated in `f`.
            match nonnull::result(ptr) {
                Ok(value) => Ok({
                    let new_pos = if UP {
                        let pos = nonnull::addr(nonnull::add(value, 1)).get();
                        up_align_usize_unchecked(pos, MIN_ALIGN)
                    } else {
                        let pos = nonnull::addr(value).get();
                        down_align_usize(pos, MIN_ALIGN)
                    };

                    self.chunk.get().set_pos_addr(new_pos);
                    BumpBox::from_raw(value)
                }),
                Err(error) => Err({
                    let error = error.as_ptr().read();
                    self.reset_to(checkpoint);
                    error
                }),
            }
        })
    }
}
