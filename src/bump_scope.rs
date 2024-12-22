#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;
use crate::{
    bump_align_guard::BumpAlignGuard,
    bump_common_methods,
    bumping::{bump_down, bump_up, BumpUp},
    chunk_size::ChunkSize,
    const_param_assert, down_align_usize,
    layout::{ArrayLayout, CustomLayout, SizedLayout},
    polyfill::{nonnull, pointer, transmute_mut, transmute_ref},
    up_align_usize_unchecked, BaseAllocator, BumpBox, BumpScopeGuard, BumpString, BumpVec, Checkpoint, ErrorBehavior,
    FixedBumpString, FixedBumpVec, MinimumAlignment, MutBumpString, MutBumpVec, MutBumpVecRev, NoDrop, RawChunk,
    SizedTypeProperties, Stats, SupportedMinimumAlignment,
};
use allocator_api2::alloc::AllocError;
use core::{
    alloc::Layout,
    cell::Cell,
    ffi::CStr,
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
        /// A bump allocation scope.
        ///
        /// A `BumpScope`'s allocations are live for `'a`, which is the lifetime of its associated `BumpScopeGuard(Root)` or `scoped` closure.
        ///
        /// `BumpScope` has the same allocation api as `Bump`.
        /// The only thing that is missing is [`reset`] and methods that consume the `Bump`.
        /// For a method overview and examples, have a look at the [`Bump` docs][`Bump`].
        ///
        /// This type is provided as a parameter to the closure of [`Bump::scoped`], [`BumpScope::scoped`] or created
        /// by [`BumpScopeGuard::scope`] and [`BumpScopeGuardRoot::scope`]. A [`Bump`] can also turned into a `BumpScope` using
        /// [`as_scope`], [`as_mut_scope`] or [`into`].
        ///
        /// [`Bump::scoped`]: crate::Bump::scoped
        /// [`BumpScopeGuard::scope`]: crate::BumpScopeGuard::scope
        /// [`BumpScopeGuardRoot::scope`]: crate::BumpScopeGuardRoot::scope
        /// [`Bump`]: crate::Bump
        /// [`scoped`]: Self::scoped
        /// [`as_scope`]: crate::Bump::as_scope
        /// [`as_mut_scope`]: crate::Bump::as_mut_scope
        /// [`reset`]: crate::Bump::reset
        /// [`into`]: crate::Bump#impl-From<%26Bump<A,+MIN_ALIGN,+UP,+GUARANTEED_ALLOCATED>>-for-%26BumpScope<'b,+A,+MIN_ALIGN,+UP,+GUARANTEED_ALLOCATED>
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
    /// Calls `f` with a new child scope.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    ///
    /// bump.scoped(|bump| {
    ///     bump.alloc_str("Hello world!");
    ///     assert_eq!(bump.stats().allocated(), 12);
    /// });
    ///
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    pub fn scoped<R>(&mut self, f: impl FnOnce(BumpScope<A, MIN_ALIGN, UP>) -> R) -> R {
        let mut guard = self.scope_guard();
        f(guard.scope())
    }

    /// Calls `f` with a new child scope of a new minimum alignment.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::{ Bump, Stats };
    /// let mut bump: Bump = Bump::new();
    ///
    /// // bump starts off by being aligned to 16
    /// assert!(bump.stats().current_chunk().bump_position().is_aligned_to(16));
    ///
    /// // allocate one byte
    /// bump.alloc(1u8);
    ///
    /// // now the bump is only aligned to 1
    /// // (if our `MIN_ALIGN` was higher, it would be that)
    /// assert!(bump.stats().current_chunk().bump_position().addr().get() % 2 == 1);
    /// assert_eq!(bump.stats().allocated(), 1);
    ///
    /// bump.scoped_aligned::<8, ()>(|bump| {
    ///    // in here, the bump will have the specified minimum alignment of 8
    ///    assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    ///    assert_eq!(bump.stats().allocated(), 8);
    ///
    ///    // allocating a value with its size being a multiple of 8 will no longer have
    ///    // to align the bump pointer before allocation
    ///    bump.alloc(1u64);
    ///    assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    ///    assert_eq!(bump.stats().allocated(), 16);
    ///    
    ///    // allocating a value smaller than the minimum alignment must align the bump pointer
    ///    // after the allocation, resulting in some wasted space
    ///    bump.alloc(1u8);
    ///    assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    ///    assert_eq!(bump.stats().allocated(), 24);
    /// });
    ///
    /// assert_eq!(bump.stats().allocated(), 1);
    /// ```
    #[inline(always)]
    pub fn scoped_aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(BumpScope<A, NEW_MIN_ALIGN, UP>) -> R,
    ) -> R
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        // This guard will reset the bump pointer to the current position, which is aligned to `MIN_ALIGN`.
        let mut guard = self.scope_guard();
        let scope = guard.scope();
        scope.align::<NEW_MIN_ALIGN>();
        f(unsafe { scope.cast_align() })
    }

    /// Calls `f` with this scope but with a new minimum alignment.
    ///
    /// # Examples
    ///
    /// Increase the minimum alignment:
    /// ```
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_scope();
    ///
    /// // here we're allocating with a `MIN_ALIGN` of `1`
    /// let foo = bump.alloc_str("foo");
    ///
    /// let bar = bump.aligned::<8, _>(|bump| {
    ///     // in here the bump position has been aligned to `8` and will stay aligned to `8`
    ///     assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    ///     assert_eq!(bump.stats().allocated(), 8);
    ///
    ///     // make some allocations that benefit from the higher `MIN_ALIGN` of `8`
    ///     let bar = bump.alloc(0u64);
    ///
    ///     assert_eq!(bump.stats().allocated(), 16);
    ///
    ///     bar
    /// });
    ///
    /// assert_eq!(bump.stats().allocated(), 16);
    ///
    /// // continue making allocations with a `MIN_ALIGN` of `1`
    /// let baz = bump.alloc_str("baz");
    /// assert_eq!(bump.stats().allocated(), 16 + 3);
    ///
    /// dbg!(foo);
    /// dbg!(bar);
    /// dbg!(baz);
    /// ```
    ///
    /// Decrease the minimum alignment:
    /// ```
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::{ Bump, allocator_api2::alloc::Global };
    /// let mut bump: Bump<Global, 8> = Bump::new();
    /// let bump = bump.as_mut_scope();
    ///
    /// // make some allocations that benefit from the `MIN_ALIGN` of `8`
    /// let foo = bump.alloc(0u64);
    ///
    /// let bar = bump.aligned::<1, _>(|bump| {
    ///     // make some allocations that benefit from the lower `MIN_ALIGN` of `1`
    ///     let bar = bump.alloc(0u8);
    ///
    ///     // the bump position will not get aligned to `8` in here
    ///     assert!(bump.stats().current_chunk().bump_position().addr().get() % 2 == 1);
    ///     assert_eq!(bump.stats().allocated(), 8 + 1);
    ///
    ///     bar
    /// });
    ///
    /// // after `aligned()`, the bump position will be aligned to `8` again
    /// // to satisfy our `MIN_ALIGN`
    /// assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    /// assert_eq!(bump.stats().allocated(), 16);
    ///
    /// // continue making allocations that benefit from the `MIN_ALIGN` of `8`
    /// let baz = bump.alloc(0u64);
    ///
    /// dbg!(foo, bar, baz);
    /// ```
    #[inline(always)]
    pub fn aligned<const NEW_MIN_ALIGN: usize, R>(&mut self, f: impl FnOnce(BumpScope<'a, A, NEW_MIN_ALIGN, UP>) -> R) -> R
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        if NEW_MIN_ALIGN < MIN_ALIGN {
            // This guard will align whatever the future bump position is back to `MIN_ALIGN`.
            let guard = BumpAlignGuard::new(self);
            f(unsafe { guard.scope.clone_unchecked().cast_align() })
        } else {
            self.align::<NEW_MIN_ALIGN>();
            f(unsafe { self.clone_unchecked().cast_align() })
        }
    }

    /// Creates a new [`BumpScopeGuard`].
    ///
    /// This allows for creation of child scopes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    ///
    /// {
    ///     let mut guard = bump.scope_guard();
    ///     let bump = guard.scope();
    ///     bump.alloc_str("Hello world!");
    ///     assert_eq!(bump.stats().allocated(), 12);
    /// }
    ///
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn scope_guard(&mut self) -> BumpScopeGuard<A, MIN_ALIGN, UP> {
        BumpScopeGuard::new(self)
    }

    /// Creates a checkpoint of the current bump position.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.stats().allocated(), 5);
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk.get())
    }

    /// Resets the bump position to a previously created checkpoint. The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    /// - the checkpoint must have been created by this bump allocator
    /// - the bump allocator must not have been [`reset`](crate::Bump::reset) since creation of this checkpoint
    /// - there must be no references to allocations made since creation of this checkpoint
    #[inline]
    pub unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        debug_assert!(self.stats().big_to_small().any(|chunk| {
            chunk.header == checkpoint.chunk.cast()
                && crate::stats::raw!(chunk.contains_addr_or_end(checkpoint.address.get()))
        }));

        checkpoint.reset_within_chunk();
        let chunk = RawChunk::from_header(checkpoint.chunk.cast());
        self.chunk.set(chunk);
    }
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
    pub(crate) unsafe fn use_prepared_slice_allocation<T>(
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
    pub(crate) unsafe fn use_prepared_slice_allocation_rev<T>(
        &self,
        mut end: NonNull<T>,
        len: usize,
        cap: usize,
    ) -> NonNull<[T]> {
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
    pub(crate) fn generic_prepare_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B> {
        B::prepare_allocation_or_else(
            self.chunk.get(),
            MinimumAlignment::<MIN_ALIGN>,
            SizedLayout::new::<T>(),
            || self.prepare_allocation_in_another_chunk::<B, T>(),
        )
        .map(NonNull::cast)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn prepare_allocation_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        let layout = Layout::new::<T>();

        unsafe {
            self.in_another_chunk(layout, |chunk, layout| {
                chunk.prepare_allocation(MinimumAlignment::<MIN_ALIGN>, CustomLayout(layout))
            })
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

        let range = match self
            .chunk
            .get()
            .prepare_allocation_range(MinimumAlignment::<MIN_ALIGN>, layout)
        {
            Some(ptr) => ptr,
            None => self.prepare_allocation_range_in_another_chunk(*layout)?,
        };

        Ok(range.start.cast::<T>()..range.end.cast::<T>())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn prepare_allocation_range_in_another_chunk<E: ErrorBehavior>(
        &self,
        layout: Layout,
    ) -> Result<Range<NonNull<u8>>, E> {
        unsafe {
            self.in_another_chunk(layout, |chunk, layout| {
                chunk.prepare_allocation_range(MinimumAlignment::<MIN_ALIGN>, CustomLayout(layout))
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
            self.in_another_chunk(layout, |chunk, layout| {
                chunk.alloc(MinimumAlignment::<MIN_ALIGN>, CustomLayout(layout))
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

    #[cold]
    #[inline(never)]
    pub(crate) fn do_alloc_sized_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.alloc_in_another_chunk(Layout::new::<T>())
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
    /// `f` on the new chunk created by `RawChunk::append_for` with the layout `layout` must return `Some`.
    #[inline(always)]
    pub(crate) unsafe fn in_another_chunk<B: ErrorBehavior, R>(
        &self,
        layout: Layout,
        mut f: impl FnMut(RawChunk<UP, A>, Layout) -> Option<R>,
    ) -> Result<R, B> {
        let new_chunk = if self.is_unallocated() {
            // When this bump allocator is unallocated, `A` is guaranteed to implement `Default`,
            // `default_or_panic` will not panic.
            let allocator = A::default_or_panic();

            RawChunk::new_in(
                ChunkSize::for_capacity(layout).ok_or_else(B::capacity_overflow)?,
                None,
                allocator,
            )
        } else {
            while let Some(chunk) = self.chunk.get().next() {
                // We don't reset the chunk position when we leave a scope, so we need to do it here.
                chunk.reset();

                self.chunk.set(chunk);

                if let Some(ptr) = f(chunk, layout) {
                    return Ok(ptr);
                }
            }

            // there is no chunk that fits, we need a new chunk
            self.chunk.get().append_for(layout)
        }?;

        self.chunk.set(new_chunk);

        if let Some(ptr) = f(new_chunk, layout) {
            Ok(ptr)
        } else {
            // SAFETY: We just appended a chunk for that specific layout, it must have enough space.
            // We don't panic here so we don't produce any panic code when using `try_` apis.
            // We check for that in `test-fallibility`.
            core::hint::unreachable_unchecked()
        }
    }

    bump_common_methods!(true);

    /// Returns `&self` as is. This is useful for macros that support both `Bump` and `BumpScope`.
    #[inline(always)]
    pub fn as_scope(&self) -> &Self {
        self
    }

    /// Returns `&mut self` as is. This is useful for macros that support both `Bump` and `BumpScope`.
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut Self {
        self
    }

    /// Converts this `BumpScope` into a `BumpScope` with a new minimum alignment.
    ///
    /// **This can not decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment.
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
        panic_on_error(self.generic_guaranteed_allocated())
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
        panic_on_error(self.generic_guaranteed_allocated_ref())
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
        panic_on_error(self.generic_guaranteed_allocated_mut())
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
        string.generic_write_fmt(args)?;
        Ok(string.into_boxed_str())
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_fmt_mut<B: ErrorBehavior>(&mut self, args: fmt::Arguments) -> Result<BumpBox<'a, str>, B> {
        if let Some(string) = args.as_str() {
            return self.generic_alloc_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        string.generic_write_fmt(args)?;
        Ok(string.into_boxed_str())
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_cstr<B: ErrorBehavior>(&self, src: &CStr) -> Result<&'a CStr, B> {
        let slice = self.generic_alloc_slice_copy(src.to_bytes_with_nul())?.into_ref();

        // SAFETY: input is `CStr` so this is too
        Ok(unsafe { CStr::from_bytes_with_nul_unchecked(slice) })
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_cstr_from_str<B: ErrorBehavior>(&self, src: &str) -> Result<&'a CStr, B> {
        let src = src.as_bytes();

        if let Some(nul) = src.iter().position(|&c| c == b'\0') {
            let bytes_with_nul = unsafe { src.get_unchecked(..nul + 1) };
            let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) };
            self.generic_alloc_cstr(cstr)
        } else {
            // `src` contains no null
            let dst = self.do_alloc_slice(src.len() + 1)?;

            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
                dst.as_ptr().add(src.len()).write(0);

                let bytes = core::slice::from_raw_parts(dst.as_ptr(), src.len() + 1);
                Ok(CStr::from_bytes_with_nul_unchecked(bytes))
            }
        }
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_cstr_fmt<B: ErrorBehavior>(&self, args: fmt::Arguments) -> Result<&'a CStr, B> {
        if let Some(string) = args.as_str() {
            return self.generic_alloc_cstr_from_str(string);
        }

        let mut string = BumpString::new_in(self);
        string.generic_write_fmt(args)?;
        string.generic_into_cstr()
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_cstr_fmt_mut<B: ErrorBehavior>(&mut self, args: fmt::Arguments) -> Result<&'a CStr, B> {
        if let Some(string) = args.as_str() {
            return self.generic_alloc_cstr_from_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        string.generic_write_fmt(args)?;
        string.generic_into_cstr()
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
                Some(value) => unsafe { vec.push_unchecked(value) },
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
        let ptr = self.generic_prepare_allocation::<B, Result<T, E>>()?;

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
