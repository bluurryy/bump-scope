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
    BaseAllocator, Bump, BumpBox, BumpScopeGuard, BumpString, BumpVec, Checkpoint, ErrorBehavior, FixedBumpString,
    FixedBumpVec, MinimumAlignment, MutBumpString, MutBumpVec, MutBumpVecRev, NoDrop, RawChunk, SizedTypeProperties,
    SupportedMinimumAlignment, align_pos,
    alloc::{AllocError, Allocator},
    allocator_impl,
    bump_align_guard::BumpAlignGuard,
    chunk_header::ChunkHeader,
    chunk_size::ChunkSize,
    const_param_assert, down_align_usize,
    layout::{ArrayLayout, CustomLayout, LayoutProps, SizedLayout},
    maybe_default_allocator,
    owned_slice::OwnedSlice,
    polyfill::{non_null, transmute_mut, transmute_ref},
    stats::{AnyStats, Stats},
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
            pub(crate) chunk: Cell<RawChunk<A, UP, GUARANTEED_ALLOCATED>>,

            /// Marks the lifetime of the mutably borrowed `BumpScopeGuard(Root)`.
            marker: PhantomData<&'a ()>,
        }
    };
}

maybe_default_allocator!(make_type);

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> UnwindSafe
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: UnwindSafe,
{
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> RefUnwindSafe
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: UnwindSafe,
{
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Debug
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(self.stats()).debug_format("BumpScope", f)
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
#[allow(clippy::needless_lifetimes)]
impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScope<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
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
    pub fn scoped<R>(&mut self, f: impl FnOnce(BumpScope<A, MIN_ALIGN, UP>) -> R) -> R {
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
    /// # use bump_scope::{Bump, alloc::Global};
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
    ///     bump.alloc_str("Hello, world!");
    ///     assert_eq!(bump.stats().allocated(), 13);
    /// }
    ///
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn scope_guard(&mut self) -> BumpScopeGuard<'_, A, MIN_ALIGN, UP> {
        BumpScopeGuard::new(self)
    }
}

/// Methods for a **not** [*guaranteed allocated*](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
#[allow(clippy::needless_lifetimes)]
impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpScope<'a, A, MIN_ALIGN, UP, false>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> Option<&'a A> {
        self.stats().current_chunk().map(|c| c.allocator())
    }
}

/// Methods that are always available.
impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<A, UP, GUARANTEED_ALLOCATED>) -> Self {
        Self {
            chunk: Cell::new(chunk),
            marker: PhantomData,
        }
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
        if !GUARANTEED_ALLOCATED && checkpoint.chunk == ChunkHeader::UNALLOCATED {
            if let Some(mut chunk) = self.chunk.get().guaranteed_allocated() {
                while let Some(prev) = chunk.prev() {
                    chunk = prev;
                }

                chunk.reset();
                self.chunk.set(chunk.coerce_guaranteed_allocated());
            }
        } else {
            debug_assert_ne!(
                checkpoint.chunk,
                ChunkHeader::UNALLOCATED,
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

    /// "Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, A, UP, GUARANTEED_ALLOCATED> {
        self.chunk.get().stats()
    }

    #[inline(always)]
    pub(crate) fn align<const ALIGN: usize>(&self)
    where
        MinimumAlignment<ALIGN>: SupportedMinimumAlignment,
    {
        if ALIGN > MIN_ALIGN {
            // The UNALLOCATED chunk is always aligned.
            if let Some(chunk) = self.chunk.get().guaranteed_allocated() {
                let pos = chunk.pos().addr();
                let addr = align_pos::<ALIGN, UP>(pos);
                unsafe { chunk.set_pos_addr(addr) };
            }
        }
    }

    /// Converts this `BumpScope` into a ***not*** [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    #[inline(always)]
    pub fn into_not_guaranteed_allocated(self) -> BumpScope<'a, A, MIN_ALIGN, UP, false> {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute(self) }
    }

    /// Borrows `BumpScope` as a ***not*** [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `BumpScope`.
    ///
    /// Note that it's not possible to mutably borrow as a not guaranteed allocated bump allocator. That's because
    /// a user could `mem::swap` it with an actual unallocated bump allocator which in turn would make `&mut self`
    /// unallocated.
    #[inline(always)]
    pub fn as_not_guaranteed_allocated(&self) -> &BumpScope<'a, A, MIN_ALIGN, UP, false> {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute_ref(self) }
    }

    /// # Safety
    ///
    /// - `self` must not be used until this clone is gone
    #[inline(always)]
    pub(crate) unsafe fn clone_unchecked(&self) -> BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
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
        unsafe { &mut *ptr::from_mut(self).cast::<BumpScope<'a, A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>() }
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

    /// Mutably borrows `BumpScope` with a new minimum alignment.
    ///
    /// **This cannot decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment.
    ///
    /// When decreasing the alignment we need to make sure that the bump position is realigned to the original alignment.
    /// That can only be ensured by having a function that takes a closure, like the methods mentioned above do.
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
    pub fn into_aligned<const NEW_MIN_ALIGN: usize>(self) -> BumpScope<'a, A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align() }
    }

    #[inline(always)]
    pub(crate) unsafe fn use_prepared_slice_allocation<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe {
            let end = start.add(len);

            if UP {
                self.set_aligned_pos(end.addr(), T::ALIGN);
                NonNull::slice_from_raw_parts(start, len)
            } else {
                let dst_end = start.add(cap);
                let dst = dst_end.sub(len);
                start.copy_to(dst, len);
                self.set_aligned_pos(dst.addr(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            }
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn use_prepared_slice_allocation_rev<T>(
        &self,
        end: NonNull<T>,
        len: usize,
        cap: usize,
    ) -> NonNull<[T]> {
        unsafe {
            if UP {
                let dst = end.sub(cap);
                let dst_end = dst.add(len);

                let src = end.sub(len);

                src.copy_to(dst, len);

                self.set_aligned_pos(dst_end.addr(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            } else {
                let dst = end.sub(len);
                self.set_aligned_pos(dst.addr(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            }
        }
    }

    /// Sets the bump position and aligns it to the required `MIN_ALIGN`.
    ///
    /// This does nothing if the current chunk is the UNALLOCATED one.
    #[inline(always)]
    pub(crate) unsafe fn set_pos(&self, pos: NonZeroUsize) {
        unsafe {
            let addr = align_pos::<MIN_ALIGN, UP>(pos);

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

        let addr = if pos_align < MIN_ALIGN {
            align_pos::<MIN_ALIGN, UP>(pos)
        } else {
            pos.get()
        };

        if let Some(chunk) = self.chunk.get().guaranteed_allocated() {
            unsafe { chunk.set_pos_addr(addr) };
        }
    }
}

/// Methods that are always available. (but with `A: Allocator`)
impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator,
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
    pub fn into_guaranteed_allocated(self, f: impl FnOnce() -> Bump<A, MIN_ALIGN, UP>) -> BumpScope<'a, A, MIN_ALIGN, UP> {
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
        f: impl FnOnce() -> Result<Bump<A, MIN_ALIGN, UP>, AllocError>,
    ) -> Result<BumpScope<'a, A, MIN_ALIGN, UP>, AllocError> {
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
    pub fn as_guaranteed_allocated(&self, f: impl FnOnce() -> Bump<A, MIN_ALIGN, UP>) -> &BumpScope<'a, A, MIN_ALIGN, UP> {
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
        f: impl FnOnce() -> Result<Bump<A, MIN_ALIGN, UP>, AllocError>,
    ) -> Result<&BumpScope<'a, A, MIN_ALIGN, UP>, AllocError> {
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
        f: impl FnOnce() -> Bump<A, MIN_ALIGN, UP>,
    ) -> &mut BumpScope<'a, A, MIN_ALIGN, UP> {
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
        f: impl FnOnce() -> Result<Bump<A, MIN_ALIGN, UP>, AllocError>,
    ) -> Result<&mut BumpScope<'a, A, MIN_ALIGN, UP>, AllocError> {
        self.try_ensure_allocated(f)?;
        Ok(unsafe { transmute_mut(self) })
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub(crate) fn ensure_allocated(&self, f: impl FnOnce() -> Bump<A, MIN_ALIGN, UP>) {
        if self.chunk.get().is_unallocated() {
            unsafe {
                self.chunk.set(RawChunk::from_header(f().into_raw().cast()));
            }
        }
    }

    #[inline(always)]
    pub(crate) fn try_ensure_allocated(
        &self,
        f: impl FnOnce() -> Result<Bump<A, MIN_ALIGN, UP>, AllocError>,
    ) -> Result<(), AllocError> {
        if self.chunk.get().is_unallocated() {
            unsafe {
                self.chunk.set(RawChunk::from_header(f()?.into_raw().cast()));
            }
        }

        Ok(())
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    pub(crate) fn generic_prepare_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B> {
        match self
            .chunk
            .get()
            .prepare_allocation(MinimumAlignment::<MIN_ALIGN>, SizedLayout::new::<T>())
        {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.prepare_allocation_in_another_chunk::<B, T>() {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn prepare_allocation_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        let layout = CustomLayout(Layout::new::<T>());

        unsafe {
            self.in_another_chunk(layout, |chunk, layout| {
                chunk.prepare_allocation(MinimumAlignment::<MIN_ALIGN>, layout)
            })
        }
    }

    pub(crate) fn generic_prepare_slice_allocation<B: ErrorBehavior, T>(&self, min_cap: usize) -> Result<NonNull<[T]>, B> {
        let range = self.prepare_allocation_range::<B, T>(min_cap)?;

        // NB: We can't use `offset_from_unsigned`, because the size is not a multiple of `T`'s.
        let cap = unsafe { non_null::byte_offset_from_unsigned(range.end, range.start) } / T::SIZE;

        let ptr = if UP { range.start } else { unsafe { range.end.sub(cap) } };

        Ok(NonNull::slice_from_raw_parts(ptr, cap))
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

        let range = match self
            .chunk
            .get()
            .prepare_allocation_range(MinimumAlignment::<MIN_ALIGN>, layout)
        {
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
        unsafe {
            self.in_another_chunk(layout, |chunk, layout| {
                chunk.prepare_allocation_range(MinimumAlignment::<MIN_ALIGN>, layout)
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
            self.in_another_chunk(CustomLayout(layout), |chunk, layout| {
                chunk.alloc(MinimumAlignment::<MIN_ALIGN>, layout)
            })
        }
    }

    #[inline(always)]
    pub(crate) fn do_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        match self.chunk.get().alloc(MinimumAlignment::<MIN_ALIGN>, SizedLayout::new::<T>()) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.do_alloc_sized_in_another_chunk::<E, T>() {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
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
        let Ok(layout) = ArrayLayout::array::<T>(len) else {
            return Err(E::capacity_overflow());
        };

        match self.chunk.get().alloc(MinimumAlignment::<MIN_ALIGN>, layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.do_alloc_slice_in_another_chunk::<E, T>(len) {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[inline(always)]
    pub(crate) fn do_alloc_slice_for<E: ErrorBehavior, T>(&self, value: &[T]) -> Result<NonNull<T>, E> {
        let layout = ArrayLayout::for_value(value);

        match self.chunk.get().alloc(MinimumAlignment::<MIN_ALIGN>, layout) {
            Some(ptr) => Ok(ptr.cast()),
            None => match self.do_alloc_slice_in_another_chunk::<E, T>(value.len()) {
                Ok(ptr) => Ok(ptr.cast()),
                Err(err) => Err(err),
            },
        }
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn do_alloc_slice_in_another_chunk<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<u8>, E>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
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
        mut f: impl FnMut(RawChunk<A, UP, true>, L) -> Option<R>,
    ) -> Result<R, B> {
        unsafe {
            let new_chunk: RawChunk<A, UP, true> = if let Some(chunk) = self.chunk.get().guaranteed_allocated() {
                while let Some(chunk) = chunk.next() {
                    // We don't reset the chunk position when we leave a scope, so we need to do it here.
                    chunk.reset();

                    self.chunk.set(chunk.coerce_guaranteed_allocated());

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

            self.chunk.set(new_chunk.coerce_guaranteed_allocated());

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

impl<A, const MIN_ALIGN: usize, const UP: bool> NoDrop for BumpScope<'_, A, MIN_ALIGN, UP> {}

/// Methods to allocate. Available as fallible or infallible.
impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Allocate an object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc(123);
    /// assert_eq!(allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc<T>(&self, value: T) -> BumpBox<'a, T> {
        panic_on_error(self.generic_alloc(value))
    }

    /// Allocate an object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc(123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc<T>(&self, value: T) -> Result<BumpBox<'a, T>, AllocError> {
        self.generic_alloc(value)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc<B: ErrorBehavior, T>(&self, value: T) -> Result<BumpBox<'a, T>, B> {
        self.generic_alloc_with(|| value)
    }

    /// Allocates space for an object, then calls `f` to produce the
    /// value to be put in that place.
    ///
    /// In some cases this could be more performant than `alloc(f())` because it
    /// permits the compiler to directly place `T` in the allocated memory instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_with(|| 123);
    /// assert_eq!(allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> BumpBox<'a, T> {
        panic_on_error(self.generic_alloc_with(f))
    }

    /// Allocates space for an object, then calls `f` to produce the
    /// value to be put in that place.
    ///
    /// In some cases this could be more performant than `try_alloc(f())` because it
    /// permits the compiler to directly place `T` in the allocated memory instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_with(|| 123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<'a, T>, AllocError> {
        self.generic_alloc_with(f)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_with<B: ErrorBehavior, T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<'a, T>, B> {
        Ok(self.generic_alloc_uninit()?.init(f()))
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[alloc_with](Self::alloc_with)(T::default)</code>.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_default::<i32>();
    /// assert_eq!(allocated, 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_default<T: Default>(&self) -> BumpBox<'a, T> {
        panic_on_error(self.generic_alloc_default())
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[try_alloc_with](Self::try_alloc_with)(T::default)</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_default()?;
    /// assert_eq!(allocated, 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_default<T: Default>(&self) -> Result<BumpBox<'a, T>, AllocError> {
        self.generic_alloc_default()
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_default<B: ErrorBehavior, T: Default>(&self) -> Result<BumpBox<'a, T>, B> {
        self.generic_alloc_with(Default::default)
    }

    /// Allocate an object by cloning it.
    ///
    /// Unlike `alloc(value.clone())` this method also works for dynamically-sized types.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// Allocate a `slice`, `str`, `CStr`, `Path`:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    ///
    /// let cloned = bump.alloc_clone(&[1, 2, 3]);
    /// assert_eq!(cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.alloc_clone("foo");
    /// assert_eq!(cloned, "foo");
    ///
    /// let cloned = bump.alloc_clone(c"foo");
    /// assert_eq!(cloned, c"foo");
    ///
    /// let cloned = bump.alloc_clone(Path::new("foo"));
    /// assert_eq!(cloned, Path::new("foo"));
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// # use bump_scope::Bump;
    ///
    /// trait FnClone: Fn() -> String + CloneToUninit {}
    /// impl<T: ?Sized + Fn() -> String + CloneToUninit> FnClone for T {}
    ///
    /// // the closure references a local variable
    /// let reference = &String::from("Hello,");
    ///
    /// // and owns a string that it will have to clone
    /// let value = String::from("world!");
    ///
    /// let closure = move || format!("{reference} {value}");
    /// let object: &dyn FnClone = &closure;
    ///
    /// assert_eq!(object(), "Hello, world!");
    ///
    /// let bump: Bump = Bump::new();
    /// let object_clone = bump.alloc_clone(object);
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// ```
    #[cfg(feature = "nightly-clone-to-uninit")]
    pub fn alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> BumpBox<'a, T> {
        panic_on_error(self.generic_alloc_clone(value))
    }

    /// Allocate an object by cloning it.
    ///
    /// Unlike `alloc(value.clone())` this method also works for dynamically-sized types.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    ///
    /// Allocate a `slice`, `str`, `CStr`, `Path`:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    ///
    /// let cloned = bump.try_alloc_clone(&[1, 2, 3])?;
    /// assert_eq!(cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.try_alloc_clone("foo")?;
    /// assert_eq!(cloned, "foo");
    ///
    /// let cloned = bump.try_alloc_clone(c"foo")?;
    /// assert_eq!(cloned, c"foo");
    ///
    /// let cloned = bump.try_alloc_clone(Path::new("foo"))?;
    /// assert_eq!(cloned, Path::new("foo"));
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// # use bump_scope::Bump;
    ///
    /// trait FnClone: Fn() -> String + CloneToUninit {}
    /// impl<T: ?Sized + Fn() -> String + CloneToUninit> FnClone for T {}
    ///
    /// // the closure references a local variable
    /// let reference = &String::from("Hello,");
    ///
    /// // and owns a string that it will have to clone
    /// let value = String::from("world!");
    ///
    /// let closure = move || format!("{reference} {value}");
    /// let object: &dyn FnClone = &closure;
    ///
    /// assert_eq!(object(), "Hello, world!");
    ///
    /// let bump: Bump = Bump::try_new()?;
    /// let object_clone = bump.try_alloc_clone(object)?;
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[cfg(feature = "nightly-clone-to-uninit")]
    pub fn try_alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> Result<BumpBox<'a, T>, AllocError> {
        self.generic_alloc_clone(value)
    }

    #[cfg(feature = "nightly-clone-to-uninit")]
    pub(crate) fn generic_alloc_clone<B: ErrorBehavior, T: CloneToUninit + ?Sized>(
        &self,
        value: &T,
    ) -> Result<BumpBox<'a, T>, B> {
        let data = self.generic_alloc_layout(Layout::for_value(value))?;
        let metadata = ptr::metadata(value);

        unsafe {
            value.clone_to_uninit(data.as_ptr());
            let ptr = ptr::from_raw_parts_mut(data.as_ptr(), metadata);
            let ptr = NonNull::new_unchecked(ptr);
            Ok(BumpBox::from_raw(ptr))
        }
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// // by value
    /// let a = bump.alloc_slice_move([1, 2]);
    /// let b = bump.alloc_slice_move(vec![3, 4]);
    /// let c = bump.alloc_slice_move(bump.alloc_iter(5..=6));
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.alloc_slice_move(&mut other);
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(a, [1, 2]);
    /// assert_eq!(b, [3, 4]);
    /// assert_eq!(c, [5, 6]);
    /// assert_eq!(d, [7, 8]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> BumpBox<'a, [T]> {
        panic_on_error(self.generic_alloc_slice_move(slice))
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// // by value
    /// let a = bump.try_alloc_slice_move([1, 2])?;
    /// let b = bump.try_alloc_slice_move(vec![3, 4])?;
    /// let c = bump.try_alloc_slice_move(bump.alloc_iter(5..=6))?;
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.try_alloc_slice_move(&mut other)?;
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(a, [1, 2]);
    /// assert_eq!(b, [3, 4]);
    /// assert_eq!(c, [5, 6]);
    /// assert_eq!(d, [7, 8]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> Result<BumpBox<'a, [T]>, AllocError> {
        self.generic_alloc_slice_move(slice)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_move<B: ErrorBehavior, T>(
        &self,
        slice: impl OwnedSlice<Item = T>,
    ) -> Result<BumpBox<'a, [T]>, B> {
        Ok(BumpVec::generic_from_owned_slice_in(slice, self)?.into_boxed_slice())
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_copy(&[1, 2, 3]);
    /// assert_eq!(allocated, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> BumpBox<'a, [T]> {
        panic_on_error(self.generic_alloc_slice_copy(slice))
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_copy(&[1, 2, 3])?;
    /// assert_eq!(allocated, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, AllocError> {
        self.generic_alloc_slice_copy(slice)
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
            Ok(BumpBox::from_raw(NonNull::slice_from_raw_parts(dst, len)))
        }
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_clone(&[String::from("a"), String::from("b")]);
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> BumpBox<'a, [T]> {
        panic_on_error(self.generic_alloc_slice_clone(slice))
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_clone(&[String::from("a"), String::from("b")])?;
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, AllocError> {
        self.generic_alloc_slice_clone(slice)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_slice_clone<B: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice_clone(slice));
        }

        Ok(self.generic_alloc_uninit_slice_for(slice)?.init_clone(slice))
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill(3, "ho");
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> BumpBox<'a, [T]> {
        panic_on_error(self.generic_alloc_slice_fill(len, value))
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_fill(3, "ho")?;
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> Result<BumpBox<'a, [T]>, AllocError> {
        self.generic_alloc_slice_fill(len, value)
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

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`alloc_slice_fill`](Self::alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill_with::<i32>(3, Default::default);
    /// assert_eq!(allocated, [0, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> BumpBox<'a, [T]> {
        panic_on_error(self.generic_alloc_slice_fill_with(len, f))
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`try_alloc_slice_fill`](Self::try_alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_fill_with::<i32>(3, Default::default)?;
    /// assert_eq!(allocated, [0, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<'a, [T]>, AllocError> {
        self.generic_alloc_slice_fill_with(len, f)
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

    /// Allocate a `str`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_str("Hello, world!");
    /// assert_eq!(allocated, "Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_str(&self, src: &str) -> BumpBox<'a, str> {
        panic_on_error(self.generic_alloc_str(src))
    }

    /// Allocate a `str`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_str("Hello, world!")?;
    /// assert_eq!(allocated, "Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_str(&self, src: &str) -> Result<BumpBox<'a, str>, AllocError> {
        self.generic_alloc_str(src)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_str<B: ErrorBehavior>(&self, src: &str) -> Result<BumpBox<'a, str>, B> {
        let slice = self.generic_alloc_slice_copy(src.as_bytes())?;

        // SAFETY: input is `str` so this is too
        Ok(unsafe { BumpBox::from_utf8_unchecked(slice) })
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`alloc_fmt_mut`](Self::alloc_fmt_mut)
    /// instead for better performance.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fmt(&self, args: fmt::Arguments) -> BumpBox<'a, str> {
        panic_on_error(self.generic_alloc_fmt(args))
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_fmt_mut`](Self::try_alloc_fmt_mut)
    /// instead for better performance.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_fmt(&self, args: fmt::Arguments) -> Result<BumpBox<'a, str>, AllocError> {
        self.generic_alloc_fmt(args)
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

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`alloc_fmt`](Self::alloc_fmt).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fmt_mut(&mut self, args: fmt::Arguments) -> BumpBox<'a, str> {
        panic_on_error(self.generic_alloc_fmt_mut(args))
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_fmt`](Self::try_alloc_fmt).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_fmt_mut(&mut self, args: fmt::Arguments) -> Result<BumpBox<'a, str>, AllocError> {
        self.generic_alloc_fmt_mut(args)
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

    /// Allocate a `CStr`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr(c"Hello, world!");
    /// assert_eq!(allocated, c"Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr(&self, src: &CStr) -> &'a CStr {
        panic_on_error(self.generic_alloc_cstr(src))
    }

    /// Allocate a `CStr`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_cstr(c"Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr(&self, src: &CStr) -> Result<&'a CStr, AllocError> {
        self.generic_alloc_cstr(src)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_cstr<B: ErrorBehavior>(&self, src: &CStr) -> Result<&'a CStr, B> {
        let slice = self.generic_alloc_slice_copy(src.to_bytes_with_nul())?.into_ref();

        // SAFETY: input is `CStr` so this is too
        Ok(unsafe { CStr::from_bytes_with_nul_unchecked(slice) })
    }

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr_from_str("Hello, world!");
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.alloc_cstr_from_str("abc\0def");
    /// assert_eq!(allocated, c"abc");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_from_str(&self, src: &str) -> &'a CStr {
        panic_on_error(self.generic_alloc_cstr_from_str(src))
    }

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_cstr_from_str("Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.try_alloc_cstr_from_str("abc\0def")?;
    /// assert_eq!(allocated, c"abc");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr_from_str(&self, src: &str) -> Result<&'a CStr, AllocError> {
        self.generic_alloc_cstr_from_str(src)
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

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`alloc_cstr_fmt_mut`](Self::alloc_cstr_fmt_mut)
    /// instead for better performance.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_fmt(&self, args: fmt::Arguments) -> &'a CStr {
        panic_on_error(self.generic_alloc_cstr_fmt(args))
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_cstr_fmt_mut`](Self::try_alloc_cstr_fmt_mut)
    /// instead for better performance.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr_fmt(&self, args: fmt::Arguments) -> Result<&'a CStr, AllocError> {
        self.generic_alloc_cstr_fmt(args)
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

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`alloc_cstr_fmt`](Self::alloc_cstr_fmt).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt_mut(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> &'a CStr {
        panic_on_error(self.generic_alloc_cstr_fmt_mut(args))
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_cstr_fmt`](Self::try_alloc_cstr_fmt).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt_mut(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> Result<&'a CStr, AllocError> {
        self.generic_alloc_cstr_fmt_mut(args)
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

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`alloc_iter_mut`].
    ///
    /// [`alloc_iter_exact`]: Self::alloc_iter_exact
    /// [`alloc_iter_mut`]: Self::alloc_iter_mut
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]> {
        panic_on_error(self.generic_alloc_iter(iter))
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`try_alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`try_alloc_iter_mut`].
    ///
    /// [`try_alloc_iter_exact`]: Self::try_alloc_iter_exact
    /// [`try_alloc_iter_mut`]: Self::try_alloc_iter_mut
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<'a, [T]>, AllocError> {
        self.generic_alloc_iter(iter)
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

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_exact([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter_exact<T, I>(&self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> BumpBox<'a, [T]>
    where
        I: ExactSizeIterator<Item = T>,
    {
        panic_on_error(self.generic_alloc_iter_exact(iter))
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_exact([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_iter_exact<T, I>(
        &self,
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> Result<BumpBox<'a, [T]>, AllocError>
    where
        I: ExactSizeIterator<Item = T>,
    {
        self.generic_alloc_iter_exact(iter)
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

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`alloc_iter`](Self::alloc_iter).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`](Self::alloc_iter_mut_rev) instead.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter_mut<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]> {
        panic_on_error(self.generic_alloc_iter_mut(iter))
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_iter`](Self::try_alloc_iter).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`](Self::alloc_iter_mut_rev) instead.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_mut([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_iter_mut<T>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<'a, [T]>, AllocError> {
        self.generic_alloc_iter_mut(iter)
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

    /// Allocate elements of an iterator into a slice in reverse order.
    ///
    /// Compared to [`alloc_iter_mut`] this function is more performant
    /// for downwards bumping allocators as the allocation for the vector can be shrunk in place
    /// without any `ptr::copy`.
    ///
    /// The reverse is true when upwards allocating. In that case it's better to use [`alloc_iter_mut`] to prevent
    /// the `ptr::copy`.
    ///
    /// [`alloc_iter_mut`]: Self::alloc_iter_mut
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut_rev([1, 2, 3]);
    /// assert_eq!(slice, [3, 2, 1]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter_mut_rev<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]> {
        panic_on_error(self.generic_alloc_iter_mut_rev(iter))
    }

    /// Allocate elements of an iterator into a slice in reverse order.
    ///
    /// Compared to [`try_alloc_iter_mut`] this function is more performant
    /// for downwards bumping allocators as the allocation for the vector can be shrunk in place
    /// without any `ptr::copy`.
    ///
    /// The reverse is true when upwards allocating. In that case it's better to use [`try_alloc_iter_mut`] to prevent
    /// the `ptr::copy`.
    ///
    /// [`try_alloc_iter_mut`]: Self::try_alloc_iter_mut
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_mut_rev([1, 2, 3])?;
    /// assert_eq!(slice, [3, 2, 1]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_iter_mut_rev<T>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<'a, [T]>, AllocError> {
        self.generic_alloc_iter_mut_rev(iter)
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

    /// Allocate an unitialized object.
    ///
    /// You can safely initialize the object with [`init`](BumpBox::init) or unsafely with [`assume_init`](BumpBox::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit();
    ///
    /// let five = uninit.init(5);
    ///
    /// assert_eq!(*five, 5)
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.alloc_uninit();
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_uninit<T>(&self) -> BumpBox<'a, MaybeUninit<T>> {
        panic_on_error(self.generic_alloc_uninit())
    }

    /// Allocate an unitialized object.
    ///
    /// You can safely initialize the object with [`init`](BumpBox::init) or unsafely with [`assume_init`](BumpBox::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.try_alloc_uninit()?;
    ///
    /// let five = uninit.init(5);
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut uninit = bump.try_alloc_uninit()?;
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_uninit<T>(&self) -> Result<BumpBox<'a, MaybeUninit<T>>, AllocError> {
        self.generic_alloc_uninit()
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_uninit<B: ErrorBehavior, T>(&self) -> Result<BumpBox<'a, MaybeUninit<T>>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::zst(MaybeUninit::uninit()));
        }

        let ptr = self.do_alloc_sized::<B, T>()?.cast::<MaybeUninit<T>>();
        unsafe { Ok(BumpBox::from_raw(ptr)) }
    }

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit_slice(3);
    ///
    /// let values = uninit.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(values, [1, 2, 3])
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.alloc_uninit_slice(3);
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_uninit_slice<T>(&self, len: usize) -> BumpBox<'a, [MaybeUninit<T>]> {
        panic_on_error(self.generic_alloc_uninit_slice(len))
    }

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.try_alloc_uninit_slice(3)?;
    ///
    /// let values = uninit.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut uninit = bump.try_alloc_uninit_slice(3)?;
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_uninit_slice<T>(&self, len: usize) -> Result<BumpBox<'a, [MaybeUninit<T>]>, AllocError> {
        self.generic_alloc_uninit_slice(len)
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
            let ptr = NonNull::slice_from_raw_parts(ptr, len);
            Ok(BumpBox::from_raw(ptr))
        }
    }

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// This is just like [`alloc_uninit_slice`](Self::alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = &[1, 2, 3];
    /// let uninit_slice = bump.alloc_uninit_slice_for(slice);
    /// assert_eq!(uninit_slice.len(), 3);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_uninit_slice_for<T>(&self, slice: &[T]) -> BumpBox<'a, [MaybeUninit<T>]> {
        panic_on_error(self.generic_alloc_uninit_slice_for(slice))
    }

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// This is just like [`try_alloc_uninit_slice`](Self::try_alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let slice = &[1, 2, 3];
    /// let uninit_slice = bump.try_alloc_uninit_slice_for(slice)?;
    /// assert_eq!(uninit_slice.len(), 3);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_uninit_slice_for<T>(&self, slice: &[T]) -> Result<BumpBox<'a, [MaybeUninit<T>]>, AllocError> {
        self.generic_alloc_uninit_slice_for(slice)
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
            let ptr = NonNull::slice_from_raw_parts(ptr, slice.len());
            Ok(BumpBox::from_raw(ptr))
        }
    }

    /// Allocate a [`FixedBumpVec`] with the given `capacity`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// # #[allow(deprecated)]
    /// let mut values = bump.alloc_fixed_vec(3);
    /// values.push(1);
    /// values.push(2);
    /// values.push(3);
    /// assert_eq!(values, [1, 2, 3])
    /// ```
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpVec::with_capacity_in()` instead"]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fixed_vec<T>(&self, capacity: usize) -> FixedBumpVec<'a, T> {
        panic_on_error(self.generic_alloc_fixed_vec(capacity))
    }

    /// Allocate a [`FixedBumpVec`] with the given `capacity`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// # #[allow(deprecated)]
    /// let mut values = bump.try_alloc_fixed_vec(3)?;
    /// values.push(1);
    /// values.push(2);
    /// values.push(3);
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpVec::try_with_capacity_in()` instead"]
    #[inline(always)]
    pub fn try_alloc_fixed_vec<T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, AllocError> {
        self.generic_alloc_fixed_vec(capacity)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_fixed_vec<B: ErrorBehavior, T>(&self, capacity: usize) -> Result<FixedBumpVec<'a, T>, B> {
        Ok(FixedBumpVec::from_uninit(self.generic_alloc_uninit_slice(capacity)?))
    }

    /// Allocate a [`FixedBumpString`] with the given `capacity` in bytes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// # #[allow(deprecated)]
    /// let mut string = bump.alloc_fixed_string(13);
    /// string.push_str("Hello,");
    /// string.push_str(" world!");
    /// assert_eq!(string, "Hello, world!");
    /// ```
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpString::with_capacity_in()` instead"]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fixed_string(&self, capacity: usize) -> FixedBumpString<'a> {
        panic_on_error(self.generic_alloc_fixed_string(capacity))
    }

    /// Allocate a [`FixedBumpString`] with the given `capacity` in bytes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// # #[allow(deprecated)]
    /// let mut string = bump.try_alloc_fixed_string(13)?;
    /// string.push_str("Hello,");
    /// string.push_str(" world!");
    /// assert_eq!(string, "Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpString::try_with_capacity_in()` instead"]
    #[inline(always)]
    pub fn try_alloc_fixed_string(&self, capacity: usize) -> Result<FixedBumpString<'a>, AllocError> {
        self.generic_alloc_fixed_string(capacity)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_fixed_string<B: ErrorBehavior>(&self, capacity: usize) -> Result<FixedBumpString<'a>, B> {
        Ok(FixedBumpString::from_uninit(self.generic_alloc_uninit_slice(capacity)?))
    }

    /// Allocates memory as described by the given `Layout`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_layout(&self, layout: Layout) -> NonNull<u8> {
        panic_on_error(self.generic_alloc_layout(layout))
    }

    /// Allocates memory as described by the given `Layout`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn try_alloc_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        self.generic_alloc_layout(layout)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_layout<B: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, B> {
        match self.chunk.get().alloc(MinimumAlignment::<MIN_ALIGN>, CustomLayout(layout)) {
            Some(ptr) => Ok(ptr),
            None => self.alloc_in_another_chunk(layout),
        }
    }

    /// Drops an allocated value and attempts to free its memory.
    ///
    /// The memory can only be freed if this is the last allocation.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let boxed = bump.alloc(3i32);
    /// assert_eq!(bump.stats().allocated(), 4);
    /// bump.dealloc(boxed);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    pub fn dealloc<T: ?Sized>(&self, boxed: BumpBox<T>) {
        let layout = Layout::for_value::<T>(&boxed);
        let ptr = boxed.into_raw();

        unsafe {
            ptr.drop_in_place();
            self.deallocate(ptr.cast(), layout);
        }
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_bytes`, <code>self.[stats](Self::stats)().[remaining](Stats::remaining)()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that these additional bytes are not necessarily in one contiguous region but
    /// might be spread out among many chunks.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve_bytes(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve_bytes(&self, additional: usize) {
        panic_on_error(self.generic_reserve_bytes(additional));
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_bytes`, <code>self.[stats](Self::stats)().[remaining](Stats::remaining)()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::try_new()?;
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.try_reserve_bytes(4096)?;
    /// assert!(bump.stats().capacity() >= 4096);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        self.generic_reserve_bytes(additional)
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
                ChunkSize::from_capacity(layout).ok_or_else(B::capacity_overflow)?,
                None,
                allocator,
            )?;
            self.chunk.set(new_chunk.coerce_guaranteed_allocated());
            Ok(())
        }
    }

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
    #[allow(clippy::missing_errors_doc)]
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
        let pos = if UP { self.chunk.get().pos() } else { ptr.cast() };

        Ok(unsafe {
            non_null::write_with(ptr, f);

            // If `f` made allocations on this bump allocator we can't shrink the allocation.
            let can_shrink = pos == self.chunk.get().pos();

            match non_null::result(ptr) {
                Ok(value) => Ok({
                    if can_shrink {
                        let new_pos = if UP {
                            let pos = value.add(1).addr().get();
                            up_align_usize_unchecked(pos, MIN_ALIGN)
                        } else {
                            let pos = value.addr().get();
                            down_align_usize(pos, MIN_ALIGN)
                        };

                        // The allocation of a non-ZST was successful, so our chunk must be allocated.
                        let chunk = self.chunk.get().guaranteed_allocated_unchecked();
                        chunk.set_pos_addr(new_pos);
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
    #[allow(clippy::missing_errors_doc)]
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
        let ptr = self.generic_prepare_allocation::<B, Result<T, E>>()?;

        Ok(unsafe {
            non_null::write_with(ptr, f);

            // There is no need for `can_shrink` checks, because we have a mutable reference
            // so there's no way anyone else has allocated in `f`.
            match non_null::result(ptr) {
                Ok(value) => Ok({
                    let new_pos = if UP {
                        let pos = value.add(1).addr().get();
                        up_align_usize_unchecked(pos, MIN_ALIGN)
                    } else {
                        let pos = value.addr().get();
                        down_align_usize(pos, MIN_ALIGN)
                    };

                    // The allocation of a non-ZST was successful, so our chunk must be allocated.
                    let chunk = self.chunk.get().guaranteed_allocated_unchecked();
                    chunk.set_pos_addr(new_pos);

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
