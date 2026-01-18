use core::{
    alloc::Layout,
    cell::Cell,
    ffi::CStr,
    fmt::{self, Debug},
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit, transmute},
    num::NonZeroUsize,
    ops::Range,
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::{self, NonNull},
};

#[cfg(feature = "nightly-clone-to-uninit")]
use core::clone::CloneToUninit;

use crate::{
    BaseAllocator, Bump, BumpBox, BumpScopeGuard, Checkpoint, ErrorBehavior, NoDrop, SizedTypeProperties, align_pos,
    alloc::{AllocError, Allocator},
    allocator_impl,
    bump_align_guard::BumpAlignGuard,
    chunk::{ChunkHeader, ChunkSize, RawChunk},
    down_align_usize,
    layout::{ArrayLayout, CustomLayout, LayoutProps, SizedLayout},
    maybe_default_allocator,
    owned_slice::OwnedSlice,
    polyfill::{self, non_null, transmute_mut, transmute_ref},
    settings::{Boolean, BumpAllocatorSettings, BumpSettings, MinimumAlignment, SupportedMinimumAlignment, True},
    stats::{AnyStats, Stats},
    traits::{self, BumpAllocatorTyped, BumpAllocatorTypedScope, MutBumpAllocatorTypedScope},
    up_align_usize_unchecked,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

macro_rules! make_type {
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
        /// by [`BumpScopeGuard::scope`] and [`BumpScopeGuardRoot::scope`]. A [`Bump`] can also be turned into a `BumpScope` using
        /// [`as_scope`], [`as_mut_scope`] or `from` / `into`.
        ///
        /// [`Bump::scoped`]: crate::Bump::scoped
        /// [`BumpScopeGuard::scope`]: crate::BumpScopeGuard::scope
        /// [`BumpScopeGuardRoot::scope`]: crate::BumpScopeGuardRoot::scope
        /// [`Bump`]: crate::Bump
        /// [`scoped`]: Self::scoped
        /// [`as_scope`]: crate::Bump::as_scope
        /// [`as_mut_scope`]: crate::Bump::as_mut_scope
        /// [`reset`]: crate::Bump::reset
        #[repr(transparent)]
        pub struct BumpScope<'a, $($allocator_parameter)*, S = BumpSettings>
        where
            S: BumpAllocatorSettings,
        {
            pub(crate) chunk: Cell<RawChunk<A, S>>,

            /// Marks the lifetime of the mutably borrowed `BumpScopeGuard(Root)`.
            marker: PhantomData<&'a ()>,
        }
    };
}

maybe_default_allocator!(make_type);

impl<A, S> UnwindSafe for BumpScope<'_, A, S>
where
    A: UnwindSafe,
    S: BumpAllocatorSettings,
{
}

impl<A, S> RefUnwindSafe for BumpScope<'_, A, S>
where
    A: UnwindSafe,
    S: BumpAllocatorSettings,
{
}

impl<A, S> Debug for BumpScope<'_, A, S>
where
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(self.stats()).debug_format("BumpScope", f)
    }
}

unsafe impl<A, S> Allocator for BumpScope<'_, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        allocator_impl::allocate(self, layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { allocator_impl::deallocate(self, ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::grow(self, ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::grow_zeroed(self, ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::shrink(self, ptr, old_layout, new_layout) }
    }
}

/// Methods for a [*guaranteed allocated*](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
impl<'a, A, S> BumpScope<'a, A, S>
where
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &'a A {
        self.stats().current_chunk().allocator()
    }

    /// Calls `f` with a new child scope.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    ///
    /// bump.scoped(|bump| {
    ///     bump.alloc_str("Hello, world!");
    ///     assert_eq!(bump.stats().allocated(), 13);
    /// });
    ///
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    pub fn scoped<R>(&mut self, f: impl FnOnce(BumpScope<A, S>) -> R) -> R {
        let mut guard = self.scope_guard();
        f(guard.scope())
    }

    /// Calls `f` with a new child scope of a new minimum alignment.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::Bump;
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
        f: impl FnOnce(BumpScope<A, S::WithMinimumAlignment<NEW_MIN_ALIGN>>) -> R,
    ) -> R
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        // This guard will reset the bump pointer to the current position, which is aligned to `MIN_ALIGN`.
        let mut guard = self.scope_guard();
        let scope = guard.scope();
        scope.align::<NEW_MIN_ALIGN>();
        f(unsafe { scope.cast() })
    }

    /// Calls `f` with this scope but with a new minimum alignment.
    ///
    /// # Examples
    ///
    /// Increase the minimum alignment:
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_scope();
    ///
    /// // here we're allocating with a `MIN_ALIGN` of `1`
    /// let foo = bump.alloc_str("foo");
    /// assert_eq!(bump.stats().allocated(), 3);
    ///
    /// let bar = bump.aligned::<8, _>(|bump| {
    ///     // in here the bump position has been aligned to `8`
    ///     assert_eq!(bump.stats().allocated(), 8);
    ///     assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    ///
    ///     // make some allocations that benefit from the higher `MIN_ALIGN` of `8`
    ///     let bar = bump.alloc(0u64);
    ///     assert_eq!(bump.stats().allocated(), 16);
    ///  
    ///     // the bump position will stay aligned to `8`
    ///     bump.alloc(0u8);
    ///     assert_eq!(bump.stats().allocated(), 24);
    ///
    ///     bar
    /// });
    ///
    /// assert_eq!(bump.stats().allocated(), 24);
    ///
    /// // continue making allocations with a `MIN_ALIGN` of `1`
    /// let baz = bump.alloc_str("baz");
    /// assert_eq!(bump.stats().allocated(), 24 + 3);
    ///
    /// dbg!(foo, bar, baz);
    /// ```
    ///
    /// Decrease the minimum alignment:
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::{Bump, alloc::Global, settings::{BumpSettings, BumpAllocatorSettings}};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithMinimumAlignment<8>;
    ///
    /// let mut bump: Bump<Global, Settings> = Bump::new();
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
    pub fn aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(BumpScope<'a, A, S::WithMinimumAlignment<NEW_MIN_ALIGN>>) -> R,
    ) -> R
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        if NEW_MIN_ALIGN < S::MIN_ALIGN {
            // This guard will align whatever the future bump position is back to `MIN_ALIGN`.
            let guard = BumpAlignGuard::new(self);
            f(unsafe { guard.scope.clone_unchecked().cast() })
        } else {
            self.align::<NEW_MIN_ALIGN>();
            f(unsafe { self.clone_unchecked().cast() })
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
    ///     bump.alloc_str("Hello, world!");
    ///     assert_eq!(bump.stats().allocated(), 13);
    /// }
    ///
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn scope_guard(&mut self) -> BumpScopeGuard<'_, A, S> {
        BumpScopeGuard::new(self)
    }
}

/// Methods that are always available.
impl<'a, A, S> BumpScope<'a, A, S>
where
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<A, S>) -> Self {
        Self {
            chunk: Cell::new(chunk),
            marker: PhantomData,
        }
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn get_allocator(&self) -> Option<&'a A> {
        self.stats().get_current_chunk().map(|c| c.allocator())
    }

    /// Creates a checkpoint of the current bump position.
    ///
    /// The bump position can be reset to this checkpoint with [`reset_to`].
    ///
    /// [`reset_to`]: Self::reset_to
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.stats().allocated(), 5);
    ///     # _ = hello;
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk.get())
    }

    /// Resets the bump position to a previously created checkpoint.
    /// The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    ///
    /// - the checkpoint must have been created by this bump allocator
    /// - the bump allocator must not have been [`reset`] since creation of this checkpoint
    /// - there must be no references to allocations made since creation of this checkpoint
    /// - the checkpoint must not have been created by an`!GUARANTEED_ALLOCATED` when self is `GUARANTEED_ALLOCATED`
    ///
    /// [`reset`]: crate::Bump::reset
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.stats().allocated(), 5);
    ///     # _ = hello;
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline]
    #[expect(clippy::missing_panics_doc)] // just debug assertions
    pub unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        // If the checkpoint was created when the bump allocator had no allocated chunk
        // then the chunk pointer will point to the unallocated chunk header.
        //
        // In such cases we reset the bump pointer to the very start of the very first chunk.
        //
        // We don't check if the chunk pointer points to the unallocated chunk header
        // if the bump allocator is `GUARANTEED_ALLOCATED`. We are allowed to not do this check
        // because of this safety condition of `reset_to`:
        // > the checkpoint must not have been created by an`!GUARANTEED_ALLOCATED` when self is `GUARANTEED_ALLOCATED`
        if !<S::GuaranteedAllocated as Boolean>::VALUE && checkpoint.chunk == ChunkHeader::unallocated::<S>() {
            if let Some(mut chunk) = self.chunk.get().guaranteed_allocated() {
                while let Some(prev) = chunk.prev() {
                    chunk = prev;
                }

                chunk.reset();

                // SAFETY: casting from guaranteed-allocated to non-guaranteed-allocated is safe
                self.chunk.set(unsafe { chunk.cast() });
            }
        } else {
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
                    chunk.contains_addr_or_end(checkpoint.address.get()),
                    "checkpoint address does not point within its chunk"
                );
            }

            unsafe {
                checkpoint.reset_within_chunk();
                let chunk = RawChunk::from_header(checkpoint.chunk.cast());
                self.chunk.set(chunk);
            }
        }
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, A, S> {
        self.chunk.get().stats()
    }

    #[inline(always)]
    pub(crate) fn align<const ALIGN: usize>(&self)
    where
        MinimumAlignment<ALIGN>: SupportedMinimumAlignment,
    {
        if ALIGN > S::MIN_ALIGN {
            // The UNALLOCATED chunk is always aligned.
            if let Some(chunk) = self.chunk.get().guaranteed_allocated() {
                let pos = chunk.pos().addr();
                let addr = align_pos(S::UP, ALIGN, pos);
                unsafe { chunk.set_pos_addr(addr) };
            }
        }
    }

    #[inline(always)]
    pub(crate) fn align_to<MinimumAlignment>(&self)
    where
        MinimumAlignment: SupportedMinimumAlignment,
    {
        if MinimumAlignment::VALUE > S::MIN_ALIGN {
            // The UNALLOCATED chunk is always aligned.
            if let Some(chunk) = self.chunk.get().guaranteed_allocated() {
                let pos = chunk.pos().addr();
                let addr = align_pos(S::UP, MinimumAlignment::VALUE, pos);
                unsafe { chunk.set_pos_addr(addr) };
            }
        }
    }

    /// Converts this `BumpScope` into a ***not*** [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    #[inline(always)]
    pub fn into_not_guaranteed_allocated(self) -> BumpScope<'a, A, S::WithGuaranteedAllocated<false>> {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute(self) }
    }

    /// Borrows `BumpScope` as a ***not*** [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    ///
    /// Note that it's not possible to mutably borrow as a not guaranteed allocated bump allocator. That's because
    /// a user could `mem::swap` it with an actual unallocated bump allocator which in turn would make `&mut self`
    /// unallocated.
    #[inline(always)]
    pub fn as_not_guaranteed_allocated(&self) -> &BumpScope<'a, A, S::WithGuaranteedAllocated<false>> {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute_ref(self) }
    }

    /// # Safety
    ///
    /// - `self` must not be used until this clone is gone
    #[inline(always)]
    pub(crate) unsafe fn clone_unchecked(&self) -> BumpScope<'a, A, S> {
        unsafe { BumpScope::new_unchecked(self.chunk.get()) }
    }

    /// Converts this `BumpScope` into a raw pointer.
    #[inline]
    #[must_use]
    pub fn into_raw(self) -> NonNull<()> {
        let this = ManuallyDrop::new(self);
        this.chunk.get().header().cast()
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
        Self {
            chunk: Cell::new(unsafe { RawChunk::from_header(ptr.cast()) }),
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast<S2>(self) -> BumpScope<'a, A, S2>
    where
        S2: BumpAllocatorSettings,
    {
        BumpScope {
            chunk: Cell::new(unsafe { self.chunk.get().cast::<S2>() }),
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_mut<S2>(&mut self) -> &mut BumpScope<'a, A, S2>
    where
        S2: BumpAllocatorSettings,
    {
        unsafe { &mut *ptr::from_mut(self).cast::<BumpScope<'a, A, S2>>() }
    }

    /// Will error at compile time if `NEW_MIN_ALIGN < MIN_ALIGN`.
    #[inline(always)]
    pub(crate) fn must_align_more<const NEW_MIN_ALIGN: usize>(&self)
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        const {
            assert!(
                NEW_MIN_ALIGN >= S::MIN_ALIGN,
                "`into_aligned` or `as_mut_aligned` can't decrease the minimum alignment"
            );
        }

        self.align::<NEW_MIN_ALIGN>();
    }

    /// Mutably borrows `BumpScope` with a new minimum alignment.
    ///
    /// **This cannot decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment.
    ///
    /// When decreasing the alignment we need to make sure that the bump position is realigned to the original alignment.
    /// That can only be ensured by having a function that takes a closure, like the methods mentioned above do.
    #[inline(always)]
    pub fn as_mut_aligned<const NEW_MIN_ALIGN: usize>(
        &mut self,
    ) -> &mut BumpScope<'a, A, S::WithMinimumAlignment<NEW_MIN_ALIGN>>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_mut() }
    }

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
    /// **This cannot decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment.
    ///
    /// When decreasing the alignment we need to make sure that the bump position is realigned to the original alignment.
    /// That can only be ensured by having a function that takes a closure, like the methods mentioned above do.
    ///
    /// If this was allowed to decrease the alignment it would break minimum alignment:
    ///
    /// ```ignore
    /// # // We can't `compile_fail,E0080` this doc test because it does not do the compile step
    /// # // that triggers the error.
    /// # use bump_scope::{Bump, alloc::Global};
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
    #[inline(always)]
    pub fn into_aligned<const NEW_MIN_ALIGN: usize>(self) -> BumpScope<'a, A, S::WithMinimumAlignment<NEW_MIN_ALIGN>>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast() }
    }

    /// Sets the bump position and aligns it to the required `MIN_ALIGN`.
    ///
    /// This does nothing if the current chunk is the UNALLOCATED one.
    #[inline(always)]
    pub(crate) unsafe fn set_pos(&self, pos: NonZeroUsize) {
        unsafe {
            let addr = align_pos(S::UP, S::MIN_ALIGN, pos);

            if let Some(chunk) = self.chunk.get().guaranteed_allocated() {
                chunk.set_pos_addr(addr);
            }
        }
    }

    /// A version of [`set_pos`](Self::set_pos) that only aligns the pointer
    /// if it the `pos_align` is smaller than the `MIN_ALIGN`.
    ///
    /// This should only be called when the `pos_align` is statically known so
    /// the branch gets optimized out.
    ///
    /// This does nothing if the current chunk is the UNALLOCATED one.
    #[inline(always)]
    pub(crate) unsafe fn set_aligned_pos(&self, pos: NonZeroUsize, pos_align: usize) {
        debug_assert_eq!(pos.get() % pos_align, 0);

        let addr = if pos_align < S::MIN_ALIGN {
            align_pos(S::UP, S::MIN_ALIGN, pos)
        } else {
            pos.get()
        };

        if let Some(chunk) = self.chunk.get().guaranteed_allocated() {
            unsafe { chunk.set_pos_addr(addr) };
        }
    }

    /// Converts this `BumpScope` into a `BumpScope` with new settings.
    ///
    /// Not all settings can be converted to. This function will fail to compile when:
    /// - the bump direction differs
    /// - the new setting is guaranteed-allocated when the old one isn't
    ///   (use [`into_guaranteed_allocated`](Self::into_guaranteed_allocated) to do this conversion)
    #[inline]
    pub fn with_settings<NewS>(self) -> BumpScope<'a, A, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        const {
            assert!(NewS::UP == S::UP, "can't change `UP` setting");

            assert!(
                NewS::GUARANTEED_ALLOCATED <= S::GUARANTEED_ALLOCATED,
                "can't turn a non-guaranteed-allocated bump allocator into a guaranteed-allocated one"
            );
        }

        self.as_scope().align_to::<NewS::MinimumAlignment>();

        unsafe { transmute(self) }
    }

    /// Borrows this `BumpScope` with new settings.
    ///
    /// Not all settings can be converted to. This function will fail to compile when:
    /// - the bump direction differs
    /// - the new setting is guaranteed-allocated when the old one isn't
    ///   (use [`as_guaranteed_allocated`](Self::as_guaranteed_allocated) to do this conversion)
    /// - the minimum alignment differs
    #[inline]
    pub fn borrow_with_settings<NewS>(&self) -> &BumpScope<'a, A, NewS>
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
        }

        unsafe { transmute_ref(self) }
    }

    /// Borrows this `BumpScope` mutably with new settings.
    ///
    /// Not all settings can be converted to. This function will fail to compile when:
    /// - the bump direction differs
    /// - the guaranteed-allocated property differs
    /// - the new minimum alignment is less than the old one
    #[inline]
    pub fn borrow_mut_with_settings<NewS>(&mut self) -> &mut BumpScope<'a, A, NewS>
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
        }

        self.as_scope().align_to::<NewS::MinimumAlignment>();

        unsafe { transmute_mut(self) }
    }
}

/// Methods that are always available. (but with `A: Allocator`)
impl<'a, A, S> BumpScope<'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    /// Converts this `BumpScope` into a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    ///
    /// See [`Bump::into_guaranteed_allocated`].
    ///
    /// # Panics
    ///
    /// Panics if the closure panics.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn into_guaranteed_allocated(
        self,
        f: impl FnOnce() -> Bump<A, S::WithGuaranteedAllocated<true>>,
    ) -> BumpScope<'a, A, S::WithGuaranteedAllocated<true>> {
        self.ensure_allocated(f);
        unsafe { transmute(self) }
    }

    /// Converts this `BumpScope` into a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    ///
    /// See [`Bump::try_into_guaranteed_allocated`].
    ///
    /// # Errors
    ///
    /// Errors if the closure fails.
    #[inline(always)]
    pub fn try_into_guaranteed_allocated(
        self,
        f: impl FnOnce() -> Result<Bump<A, S::WithGuaranteedAllocated<true>>, AllocError>,
    ) -> Result<BumpScope<'a, A, S::WithGuaranteedAllocated<true>>, AllocError> {
        self.try_ensure_allocated(f)?;
        Ok(unsafe { transmute(self) })
    }

    /// Borrows `BumpScope` as a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    ///
    /// See [`Bump::as_guaranteed_allocated`].
    ///
    /// # Panics
    ///
    /// Panics if the closure panics.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn as_guaranteed_allocated(
        &self,
        f: impl FnOnce() -> Bump<A, S::WithGuaranteedAllocated<true>>,
    ) -> &BumpScope<'a, A, S::WithGuaranteedAllocated<true>> {
        self.ensure_allocated(f);
        unsafe { transmute_ref(self) }
    }

    /// Borrows `BumpScope` as a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    ///
    /// See [`Bump::try_as_guaranteed_allocated`].
    ///
    /// # Errors
    ///
    /// Errors if the closure fails.
    #[inline(always)]
    pub fn try_as_guaranteed_allocated(
        &self,
        f: impl FnOnce() -> Result<Bump<A, S::WithGuaranteedAllocated<true>>, AllocError>,
    ) -> Result<&BumpScope<'a, A, S::WithGuaranteedAllocated<true>>, AllocError> {
        self.try_ensure_allocated(f)?;
        Ok(unsafe { transmute_ref(self) })
    }

    /// Mutably borrows `BumpScope` as a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    ///
    /// See [`Bump::as_mut_guaranteed_allocated`].
    ///
    /// # Panics
    ///
    /// Panics if the closure panics.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn as_mut_guaranteed_allocated(
        &mut self,
        f: impl FnOnce() -> Bump<A, S::WithGuaranteedAllocated<true>>,
    ) -> &mut BumpScope<'a, A, S::WithGuaranteedAllocated<true>> {
        self.ensure_allocated(f);
        unsafe { transmute_mut(self) }
    }

    /// Mutably borrows `BumpScope` as a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    ///
    /// See [`Bump::try_as_mut_guaranteed_allocated`].
    ///
    /// # Errors
    ///
    /// Errors if the closure fails.
    #[inline(always)]
    pub fn try_as_mut_guaranteed_allocated(
        &mut self,
        f: impl FnOnce() -> Result<Bump<A, S::WithGuaranteedAllocated<true>>, AllocError>,
    ) -> Result<&mut BumpScope<'a, A, S::WithGuaranteedAllocated<true>>, AllocError> {
        self.try_ensure_allocated(f)?;
        Ok(unsafe { transmute_mut(self) })
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) fn ensure_allocated(&self, f: impl FnOnce() -> Bump<A, S::WithGuaranteedAllocated<true>>) {
        if self.chunk.get().is_unallocated() {
            unsafe {
                self.chunk.set(RawChunk::from_header(f().into_raw().cast()));
            }
        }
    }

    #[inline(always)]
    pub(crate) fn try_ensure_allocated(
        &self,
        f: impl FnOnce() -> Result<Bump<A, S::WithGuaranteedAllocated<true>>, AllocError>,
    ) -> Result<(), AllocError> {
        if self.chunk.get().is_unallocated() {
            unsafe {
                self.chunk.set(RawChunk::from_header(f()?.into_raw().cast()));
            }
        }

        Ok(())
    }
}

impl<A, S> BumpScope<'_, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    pub(crate) fn generic_allocate_layout<B: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, B> {
        match self.chunk.get().alloc(CustomLayout(layout)) {
            Some(ptr) => Ok(ptr),
            None => self.alloc_in_another_chunk(layout),
        }
    }

    #[inline(always)]
    pub(crate) fn generic_allocate_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        if T::IS_ZST {
            return Ok(NonNull::dangling());
        }

        match self.chunk.get().alloc(SizedLayout::new::<T>()) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.allocate_sized_in_another_chunk::<E, T>() {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline(always)]
    pub(crate) fn generic_allocate_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        if T::IS_ZST {
            return Ok(NonNull::dangling());
        }

        let Ok(layout) = ArrayLayout::array::<T>(len) else {
            return Err(E::capacity_overflow());
        };

        match self.chunk.get().alloc(layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.allocate_slice_in_another_chunk::<E, T>(len) {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline(always)]
    pub(crate) fn generic_allocate_slice_for<E: ErrorBehavior, T>(&self, value: &[T]) -> Result<NonNull<T>, E> {
        if T::IS_ZST {
            return Ok(NonNull::dangling());
        }

        let layout = ArrayLayout::for_value(value);

        match self.chunk.get().alloc(layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.allocate_slice_in_another_chunk::<E, T>(value.len()) {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline(always)]
    pub(crate) fn generic_prepare_slice_allocation<B: ErrorBehavior, T>(&self, min_cap: usize) -> Result<NonNull<[T]>, B> {
        let range = self.prepare_allocation_range::<B, T>(min_cap)?;

        // NB: We can't use `offset_from_unsigned`, because the size is not a multiple of `T`'s.
        let cap = unsafe { non_null::byte_offset_from_unsigned(range.end, range.start) } / T::SIZE;

        let ptr = if S::UP { range.start } else { unsafe { range.end.sub(cap) } };

        Ok(NonNull::slice_from_raw_parts(ptr, cap))
    }

    #[inline(always)]
    pub(crate) fn generic_reserve_bytes<B: ErrorBehavior>(&self, additional: usize) -> Result<(), B> {
        let Ok(layout) = Layout::from_size_align(additional, 1) else {
            return Err(B::capacity_overflow());
        };

        if let Some(mut chunk) = self.chunk.get().guaranteed_allocated() {
            let mut additional = additional;

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
        } else {
            let allocator = A::default_or_panic();
            let new_chunk = RawChunk::new_in(
                ChunkSize::<A, S::Up>::from_capacity(layout).ok_or_else(B::capacity_overflow)?,
                None,
                allocator,
            )?;
            self.chunk.set(new_chunk);
            Ok(())
        }
    }

    #[inline(always)]
    pub(crate) fn generic_prepare_sized_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B> {
        match self.chunk.get().prepare_allocation(SizedLayout::new::<T>()) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.prepare_allocation_in_another_chunk::<B, T>() {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn prepare_allocation_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E> {
        let layout = CustomLayout(Layout::new::<T>());

        unsafe { self.in_another_chunk(layout, RawChunk::prepare_allocation) }
    }

    /// Returns a pointer range.
    /// The start and end pointers are aligned.
    /// But `end - start` is *not* a multiple of `size_of::<T>()`.
    /// So `end.offset_from_unsigned(start)` may not be used!
    #[inline(always)]
    pub(crate) fn prepare_allocation_range<B: ErrorBehavior, T>(&self, cap: usize) -> Result<Range<NonNull<T>>, B> {
        let Ok(layout) = ArrayLayout::array::<T>(cap) else {
            return Err(B::capacity_overflow());
        };

        let range = match self.chunk.get().prepare_allocation_range(layout) {
            Some(ptr) => ptr,
            None => self.prepare_allocation_range_in_another_chunk(layout)?,
        };

        Ok(range.start.cast::<T>()..range.end.cast::<T>())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn prepare_allocation_range_in_another_chunk<E: ErrorBehavior>(
        &self,
        layout: ArrayLayout,
    ) -> Result<Range<NonNull<u8>>, E> {
        unsafe { self.in_another_chunk(layout, RawChunk::prepare_allocation_range) }
    }

    #[inline(always)]
    pub(crate) fn alloc_in_current_chunk(&self, layout: Layout) -> Option<NonNull<u8>> {
        self.chunk.get().alloc(CustomLayout(layout))
    }

    /// Allocation slow path.
    /// The active chunk must *not* have space for `layout`.
    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        // An allocation of size 0 will fail if we currently use the "unallodated" dummy chunk.
        // In this case we don't want to allocate a chunk, just return a dangling pointer.
        if !S::GUARANTEED_ALLOCATED && layout.size() == 0 {
            return Ok(polyfill::layout::dangling(layout));
        }

        unsafe { self.in_another_chunk(CustomLayout(layout), RawChunk::alloc) }
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn allocate_sized_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E> {
        self.alloc_in_another_chunk(Layout::new::<T>())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn allocate_slice_in_another_chunk<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<u8>, E> {
        let Ok(layout) = Layout::array::<T>(len) else {
            return Err(E::capacity_overflow());
        };

        self.alloc_in_another_chunk(layout)
    }

    /// # Safety
    ///
    /// `f` on the new chunk created by `RawChunk::append_for` with the layout `layout` must return `Some`.
    #[inline(always)]
    pub(crate) unsafe fn in_another_chunk<B: ErrorBehavior, R, L: LayoutProps>(
        &self,
        layout: L,
        mut f: impl FnMut(RawChunk<A, S::WithGuaranteedAllocated<true>>, L) -> Option<R>,
    ) -> Result<R, B> {
        unsafe {
            let new_chunk: RawChunk<A, S::WithGuaranteedAllocated<true>> =
                if let Some(mut chunk) = self.chunk.get().guaranteed_allocated() {
                    while let Some(next_chunk) = chunk.next() {
                        chunk = next_chunk;

                        // We don't reset the chunk position when we leave a scope, so we need to do it here.
                        chunk.reset();

                        // SAFETY: casting from guaranteed-allocated to non-guaranteed-allocated is safe
                        self.chunk.set(chunk.cast());

                        if let Some(ptr) = f(chunk, layout) {
                            return Ok(ptr);
                        }
                    }

                    // there is no chunk that fits, we need a new chunk
                    chunk.append_for(*layout)
                } else {
                    // When this bump allocator is unallocated, `A` is guaranteed to implement `Default`,
                    // `default_or_panic` will not panic.
                    let allocator = A::default_or_panic();

                    RawChunk::new_in(
                        ChunkSize::from_capacity(*layout).ok_or_else(B::capacity_overflow)?,
                        None,
                        allocator,
                    )
                }?;

            // SAFETY: casting from guaranteed-allocated to non-guaranteed-allocated is safe
            self.chunk.set(new_chunk.cast());

            match f(new_chunk, layout) {
                Some(ptr) => Ok(ptr),
                _ => {
                    // SAFETY: We just appended a chunk for that specific layout, it must have enough space.
                    // We don't panic here so we don't produce any panic code when using `try_` apis.
                    // We check for that in `test-fallibility`.
                    core::hint::unreachable_unchecked()
                }
            }
        }
    }
}

impl<A, S> NoDrop for BumpScope<'_, A, S> where S: BumpAllocatorSettings {}

/// Methods that forward to traits.
// error docs can be found in the forwarded-to method
#[allow(clippy::missing_errors_doc)]
impl<'a, A, S> BumpScope<'a, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    traits::forward_alloc_methods! {
        self: self
        access: {self}
        access_mut: {self}
        lifetime: 'a
    }
}

/// Additional `alloc` methods that are not from traits.
impl<'a, A, S> BumpScope<'a, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// There is also [`alloc_try_with_mut`](Self::alloc_try_with_mut), optimized for a mutable reference.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    pub fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpBox<'a, T>, E> {
        panic_on_error(self.generic_alloc_try_with(f))
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// There is also [`try_alloc_try_with_mut`](Self::try_alloc_try_with_mut), optimized for a mutable reference.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_try_with<T, E>(
        &self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpBox<'a, T>, E>, AllocError> {
        self.generic_alloc_try_with(f)
    }

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
        let pos = if S::UP { self.chunk.get().pos() } else { ptr.cast() };

        Ok(unsafe {
            non_null::write_with(ptr, f);

            // If `f` made allocations on this bump allocator we can't shrink the allocation.
            let can_shrink = pos == self.chunk.get().pos();

            match non_null::result(ptr) {
                Ok(value) => Ok({
                    if can_shrink {
                        let new_pos = if S::UP {
                            let pos = value.add(1).addr().get();
                            up_align_usize_unchecked(pos, S::MIN_ALIGN)
                        } else {
                            let pos = value.addr().get();
                            down_align_usize(pos, S::MIN_ALIGN)
                        };

                        // The allocation of a non-ZST was successful, so our chunk must be allocated.
                        let chunk = self.chunk.get().guaranteed_allocated_unchecked();
                        chunk.set_pos_addr(new_pos);
                    }

                    BumpBox::from_raw(value)
                }),
                Err(error) => Err({
                    let error = error.read();

                    if can_shrink {
                        self.reset_to(checkpoint_before_alloc);
                    }

                    error
                }),
            }
        })
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// This is just like [`alloc_try_with`](Self::alloc_try_with), but optimized for a mutable reference.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with_mut(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with_mut(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    pub fn alloc_try_with_mut<T, E>(&mut self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpBox<'a, T>, E> {
        panic_on_error(self.generic_alloc_try_with_mut(f))
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// This is just like [`try_alloc_try_with`](Self::try_alloc_try_with), but optimized for a mutable reference.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_try_with_mut<T, E>(
        &mut self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpBox<'a, T>, E>, AllocError> {
        self.generic_alloc_try_with_mut(f)
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
        let ptr = self.generic_prepare_sized_allocation::<B, Result<T, E>>()?;

        Ok(unsafe {
            non_null::write_with(ptr, f);

            // There is no need for `can_shrink` checks, because we have a mutable reference
            // so there's no way anyone else has allocated in `f`.
            match non_null::result(ptr) {
                Ok(value) => Ok({
                    let new_pos = if S::UP {
                        let pos = value.add(1).addr().get();
                        up_align_usize_unchecked(pos, S::MIN_ALIGN)
                    } else {
                        let pos = value.addr().get();
                        down_align_usize(pos, S::MIN_ALIGN)
                    };

                    // The allocation of a non-ZST was successful, so our chunk must be allocated.
                    let chunk = self.chunk.get().guaranteed_allocated_unchecked();
                    chunk.set_pos_addr(new_pos);

                    BumpBox::from_raw(value)
                }),
                Err(error) => Err({
                    let error = error.read();
                    self.reset_to(checkpoint);
                    error
                }),
            }
        })
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_uninit<B: ErrorBehavior, T>(&self) -> Result<BumpBox<'a, MaybeUninit<T>>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::zst(MaybeUninit::uninit()));
        }

        let ptr = self.generic_allocate_sized::<B, T>()?.cast::<MaybeUninit<T>>();
        unsafe { Ok(BumpBox::from_raw(ptr)) }
    }
}
