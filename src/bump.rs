#[cfg(not(no_global_oom_handling))]
use crate::infallible;
use crate::{
    bump_common_methods, bump_scope_methods,
    chunk_size::ChunkSize,
    error_behavior_generic_methods_allocation_failure,
    polyfill::{cfg_const, pointer, transmute_ref},
    unallocated_chunk_header, BaseAllocator, BumpScope, BumpScopeGuardRoot, Checkpoint, ErrorBehavior, MinimumAlignment,
    RawChunk, Stats, SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};
use allocator_api2::alloc::AllocError;
use core::{
    alloc::Layout,
    cell::Cell,
    fmt::{self, Debug},
    mem::{self, transmute, ManuallyDrop},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

macro_rules! bump_declaration {
    ($($allocator_parameter:tt)*) => {
        /// A bump allocator.
        ///
        /// Most of `Bump`'s constructors allocate memory.
        /// The exception is [`Bump::unallocated`]. A bump allocator created by this function has its [`GUARANTEED_ALLOCATED` parameter set to `false`](crate#guaranteed_allocated-parameter).
        /// Such a `Bump` is unable to create a scope with `scoped` or `scope_guard`.
        /// It can be converted into a guaranteed allocated `Bump` with [`guaranteed_allocated`](Bump::guaranteed_allocated) or [`guaranteed_allocated_mut`](Bump::guaranteed_allocated_mut).
        ///
        /// # Gotchas
        ///
        /// Allocating directly on a `Bump` is not compatible with entering bump scopes at the same time:
        ///
        /// ```compile_fail,E0502
        /// # use bump_scope::Bump;
        /// let mut bump: Bump = Bump::new();
        ///
        /// let one = bump.alloc(1);
        ///
        /// bump.scoped(|bump| {
        ///     // whatever
        /// });
        /// ```
        /// Instead convert it to a [`BumpScope`] first:
        /// ```
        /// # use bump_scope::Bump;
        /// let mut bump: Bump = Bump::new();
        /// let bump = bump.as_mut_scope();
        ///
        /// let one = bump.alloc(1);
        ///
        /// bump.scoped(|bump| {
        ///     // whatever
        /// });
        /// ```
        #[repr(transparent)]
        pub struct Bump<
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        > where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
            pub(crate) chunk: Cell<RawChunk<UP, A>>,
        }
    };
}

crate::maybe_default_allocator!(bump_declaration);

// Sending Bumps when nothing is allocated is fine.
// When something is allocated Bump is borrowed and sending is not possible.
unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Send
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> UnwindSafe
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + UnwindSafe,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> RefUnwindSafe
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + UnwindSafe,
{
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn drop(&mut self) {
        if self.is_unallocated() {
            return;
        }

        unsafe {
            let chunk = self.chunk.get();
            chunk.for_each_prev(|chunk| chunk.deallocate());
            chunk.for_each_next(|chunk| chunk.deallocate());
            chunk.deallocate();
        }
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Debug
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.stats().debug_format("Bump", f)
    }
}

#[cfg(not(no_global_oom_handling))]
impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Default
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + Default,
{
    #[inline(always)]
    fn default() -> Self {
        Self::new_in(Default::default())
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP, false>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<false>,
{
    cfg_const! {
        #[cfg_const(feature = "nightly-const-refs-to-static")]
        /// Constructs a new `Bump` without doing any allocations.
        ///
        /// **This is `const` when the `nightly-const-refs-to-static` feature is enabled.**
        #[must_use]
        pub fn unallocated() -> Self {
            Self { chunk: Cell::new(unsafe { RawChunk::from_header(unallocated_chunk_header().cast()) }) }
        }
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + Default,
{
    error_behavior_generic_methods_allocation_failure! {
        impl
        /// This is equivalent to <code>[with_size](Bump::with_size)(512)</code>.
        #[must_use]
        for fn new
        /// This is equivalent to <code>[try_with_size](Bump::try_with_size)(512)</code>.
        for fn try_new
        #[inline]
        use fn generic_new() -> Self {
            Self::generic_new_in(Default::default())
        }

        impl
        #[must_use]
        /// Constructs a new `Bump` with a size hint for the first chunk.
        ///
        /// If you want to ensure a specific capacity use <code>[with_capacity](Bump::with_capacity)</code> instead.
        ///
        /// The actual size that will be requested from the base allocator may be bigger or smaller.
        /// (The size of  `[usize;2]` will be subtracted to make it friendlier towards its base allocator that may store its own header information along with it.)
        for fn with_size
        /// Constructs a new `Bump` with a size hint for the first chunk.
        ///
        /// If you want to ensure a specific capacity use <code>[try_with_capacity](Bump::try_with_capacity)</code> instead.
        ///
        /// The actual size that will be requested from the base allocator may be bigger or smaller.
        /// (The size of  `[usize;2]` will be subtracted to make it friendlier towards its base allocator that may store its own header information along with it.)
        for fn try_with_size
        #[inline]
        use fn generic_with_size(size: usize) -> Self {
            Self::generic_with_size_in(size, Default::default())
        }

        impl
        #[must_use]
        /// Constructs a new `Bump` with at least enough space for `layout`.
        ///
        /// To construct a `Bump` with some rough size estimate like `1 << 16` use <code>[with_size](Bump::with_size)</code> instead.
        for fn with_capacity
        /// Constructs a new `Bump` with at least enough space for `layout`.
        ///
        /// To construct a `Bump` with some rough size estimate like `1 << 16` use <code>[try_with_size](Bump::try_with_size)</code> instead.
        for fn try_with_capacity
        #[inline]
        use fn generic_with_capacity(layout: Layout) -> Self {
            Self::generic_with_capacity_in(layout, Default::default())
        }
    }
}

/// These functions are only available if the `Bump` is [guaranteed allocated](crate#guaranteed_allocated-parameter).
impl<A, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
    bump_scope_methods!(BumpScopeGuardRoot, false);
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    error_behavior_generic_methods_allocation_failure! {
        impl
        /// This is equivalent to <code>[with_size_in](Bump::with_size_in)(512, allocator)</code>.
        for fn new_in
        /// This is equivalent to <code>[try_with_size_in](Bump::try_with_size_in)(512, allocator)</code>.
        for fn try_new_in
        #[inline]
        use fn generic_new_in(allocator: A) -> Self {
            Ok(Self {
                chunk: Cell::new(RawChunk::new_in(
                    ChunkSize::DEFAULT_START,
                    None,
                    allocator,
                )?),
            })
        }

        impl
        /// Constructs a new `Bump` with a size hint for the first chunk.
        ///
        /// If you want to ensure a specific capacity use <code>[with_capacity_in](Bump::with_capacity_in)</code> instead.
        ///
        /// The actual size that will be requested from the base allocator may be bigger or smaller.
        /// (The size of  `[usize;2]` will be subtracted to make it friendlier towards its base allocator that may store its own header information along with it.)
        for fn with_size_in
        /// Constructs a new `Bump` with a size hint for the first chunk.
        ///
        /// If you want to ensure a specific capacity use <code>[try_with_capacity_in](Bump::try_with_capacity_in)</code> instead.
        ///
        /// The actual size that will be requested from the base allocator may be bigger or smaller.
        /// (The size of  `[usize;2]` will be subtracted to make it friendlier towards its base allocator that may store its own header information along with it.)
        for fn try_with_size_in
        #[inline]
        use fn generic_with_size_in(size: usize, allocator: A) -> Self {
            Ok(Self {
                chunk: Cell::new(RawChunk::new_in(
                    ChunkSize::new(size).ok_or_else(B::capacity_overflow)?,
                    None,
                    allocator,
                )?),
            })
        }

        impl
        /// Constructs a new `Bump` with at least enough space for `layout`.
        ///
        /// To construct a `Bump` with some rough size estimate like `1 << 16` use <code>[with_size_in](Bump::with_size_in)</code> instead.
        for fn with_capacity_in
        /// Constructs a new `Bump` with at least enough space for `layout`.
        ///
        /// To construct a `Bump` with some rough size estimate like `1 << 16` use <code>[try_with_size_in](Bump::try_with_size_in)</code> instead.
        for fn try_with_capacity_in
        #[inline]
        use fn generic_with_capacity_in(layout: Layout, allocator: A) -> Self {
            Ok(Self {
                chunk: Cell::new(RawChunk::new_in(
                    ChunkSize::for_capacity(layout).ok_or_else(B::capacity_overflow)?,
                    None,
                    allocator,
                )?),
            })
        }
    }

    // This needs `&mut self` to make sure that no allocations are alive.
    /// Deallocates every chunk but the newest, which is also the biggest.
    #[inline(always)]
    pub fn reset(&mut self) {
        let mut chunk = self.chunk.get();

        unsafe {
            chunk.for_each_prev(|chunk| chunk.deallocate());

            while let Some(next) = chunk.next() {
                chunk.deallocate();
                chunk = next;
            }
        }

        chunk.set_prev(None);
        chunk.reset();
        self.chunk.set(chunk);
    }

    bump_common_methods!(false);

    /// Returns this `&Bump` as a `&BumpScope`.
    #[inline(always)]
    pub fn as_scope(&self) -> &BumpScope<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &*pointer::from_ref(self).cast() }
    }

    /// Returns this `&mut Bump` as a `&mut BumpScope`.
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut BumpScope<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &mut *pointer::from_mut(self).cast() }
    }

    /// Converts this `Bump` into a `Bump` with a new minimum alignment.
    #[inline(always)]
    pub fn into_aligned<const NEW_MIN_ALIGN: usize>(self) -> Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.as_scope().align::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align() }
    }

    /// Mutably borrows `Bump` with a new minimum alignment.
    ///
    /// **This can not decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment."
    ///
    /// To decrease alignment we need to ensure that we return to our original alignment.
    /// That can only be guaranteed by a function taking a closure like the ones mentioned above.
    #[inline(always)]
    pub fn as_aligned_mut<const NEW_MIN_ALIGN: usize>(&mut self) -> &mut Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.as_scope().must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align_mut() }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align<const NEW_MIN_ALIGN: usize>(self) -> Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        let chunk = self.chunk.get();
        mem::forget(self);

        Bump { chunk: Cell::new(chunk) }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align_mut<const NEW_MIN_ALIGN: usize>(
        &mut self,
    ) -> &mut Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        &mut *pointer::from_mut(self).cast::<Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>()
    }

    /// Converts this `Bump` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[cfg(not(no_global_oom_handling))]
    pub fn guaranteed_allocated(self) -> Bump<A, MIN_ALIGN, UP> {
        infallible(self.generic_guaranteed_allocated())
    }

    /// Converts this `Bump` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    pub fn try_guaranteed_allocated(self) -> Result<Bump<A, MIN_ALIGN, UP>, AllocError> {
        self.generic_guaranteed_allocated()
    }

    fn generic_guaranteed_allocated<E: ErrorBehavior>(self) -> Result<Bump<A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { self.cast_allocated() })
    }

    /// Mutably borrows `Bump` in a [guaranteed allocated](crate#guaranteed_allocated-parameter) state.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[cfg(not(no_global_oom_handling))]
    pub fn guaranteed_allocated_mut(&mut self) -> &mut Bump<A, MIN_ALIGN, UP> {
        infallible(self.generic_guaranteed_allocated_mut())
    }

    /// Mutably borrows `Bump` in an [guaranteed allocated](crate#guaranteed_allocated-parameter) state.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    pub fn try_guaranteed_allocated_mut(&mut self) -> Result<&mut Bump<A, MIN_ALIGN, UP>, AllocError> {
        self.generic_guaranteed_allocated_mut()
    }

    fn generic_guaranteed_allocated_mut<E: ErrorBehavior>(&mut self) -> Result<&mut Bump<A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { self.cast_allocated_mut() })
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_allocated(self) -> Bump<A, MIN_ALIGN, UP> {
        let chunk = self.chunk.get();
        mem::forget(self);

        Bump { chunk: Cell::new(chunk) }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_allocated_mut(&mut self) -> &mut Bump<A, MIN_ALIGN, UP> {
        &mut *pointer::from_mut(self).cast::<Bump<A, MIN_ALIGN, UP>>()
    }

    /// Converts this `BumpScope` into a ***not*** [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    pub fn not_guaranteed_allocated(self) -> Bump<A, MIN_ALIGN, UP, false>
    where
        A: Default,
    {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute(self) }
    }

    /// Borrows `Bump` in a ***not*** [guaranteed allocated](crate#guaranteed_allocated-parameter) state.
    ///
    /// Note that it's not possible to mutably borrow as a not guaranteed allocated bump allocator. That's because
    /// a user could `mem::swap` it with an actual unallocated bump allocator which in turn would make `&mut self` be
    /// unallocated.
    pub fn not_guaranteed_allocated_ref(&self) -> &Bump<A, MIN_ALIGN, UP, false>
    where
        A: Default,
    {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute_ref(self) }
    }

    /// Converts this `Bump` into a raw pointer.
    ///
    /// ```
    /// use bump_scope::Bump;
    ///
    /// let bump: Bump = Bump::new();
    /// let ptr = bump.into_raw();
    /// let bump: Bump = unsafe { Bump::from_raw(ptr) };
    ///
    /// bump.alloc_str("Why did i do this?");
    /// ```
    #[inline]
    #[must_use]
    pub fn into_raw(self) -> NonNull<()> {
        let this = ManuallyDrop::new(self);
        this.chunk.get().header_ptr().cast()
    }

    /// Converts the raw pointer that was created with [`into_raw`](Bump::into_raw) back into a `Bump`.
    ///
    /// # Safety
    /// - `ptr` must have been created with `Self::into_raw`.
    /// - This function must only be called once with this `ptr`.
    #[inline]
    #[must_use]
    pub unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        let chunk = Cell::new(RawChunk::from_header(ptr.cast()));
        Self { chunk }
    }
}

impl<'b, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    From<&'b Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for &'b BumpScope<'b, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn from(value: &'b Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        value.as_scope()
    }
}

impl<'b, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    From<&'b mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>
    for &'b mut BumpScope<'b, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn from(value: &'b mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        value.as_mut_scope()
    }
}
