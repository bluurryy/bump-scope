use core::{
    alloc::Layout,
    cell::Cell,
    fmt::{self, Debug},
    mem::{self, ManuallyDrop},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

use allocator_api2::alloc::{AllocError, Allocator};

#[cfg(feature = "alloc")]
use allocator_api2::alloc::Global;

use crate::{
    bump_common_methods, bump_scope_methods, chunk_size::ChunkSize, doc_align_cant_decrease, error_behavior_generic_methods,
    polyfill::pointer, BumpScope, BumpScopeGuardRoot, Checkpoint, ErrorBehavior, GuaranteedAllocatedStats, MinimumAlignment,
    RawChunk, Stats, SupportedMinimumAlignment, WithoutDealloc, WithoutShrink, DEFAULT_START_CHUNK_SIZE,
};

#[cfg(not(no_global_oom_handling))]
use crate::infallible;

#[cfg(feature = "alloc")]
use crate::{empty_chunk_header, polyfill::cfg_const};

#[cfg(test)]
use crate::WithDrop;

/// A bump allocator.
///
/// Most of `Bump`'s constructors allocate memory.
/// The exception is [`Bump::unallocated`]. A bump allocator created by this function has its [`GUARANTEED_ALLOCATED` parameter set to `false`](crate#guaranteed_allocated-parameter).
/// Such a `Bump` is unable to create a scope with `scoped` or `scope_guard`.
/// It can be converted into a guaranteed allocated `Bump` with [`into_guaranteed_allocated`](Bump::into_guaranteed_allocated) or [`as_guaranteed_allocated_mut`](Bump::as_guaranteed_allocated_mut).
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
    #[cfg(feature = "alloc")] A: Allocator + Clone = Global,
    #[cfg(not(feature = "alloc"))] A: Allocator + Clone,
    const MIN_ALIGN: usize = 1,
    const UP: bool = true,
    const GUARANTEED_ALLOCATED: bool = true,
> where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) chunk: Cell<RawChunk<UP, A>>,
}

// Sending Bumps when nothing is allocated is fine.
// When something is allocated Bump is borrowed and sending is not possible.
unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Send
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> UnwindSafe
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone + UnwindSafe,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> RefUnwindSafe
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone + UnwindSafe,
{
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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
    A: Allocator + Clone,
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
    A: Allocator + Clone + Default,
{
    #[inline(always)]
    fn default() -> Self {
        Self::new_in(Default::default())
    }
}

#[cfg(feature = "alloc")]
impl<const MIN_ALIGN: usize, const UP: bool> Bump<Global, MIN_ALIGN, UP, false>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    cfg_const! {
        #[cfg_const(feature = "nightly-const-refs-to-static")]
        /// Constructs a new `Bump` without doing any allocations.
        ///
        /// **This is `const` when the `nightly-const-refs-to-static` is enabled.**
        #[must_use]
        pub fn unallocated() -> Self {
            Self { chunk: Cell::new(unsafe { RawChunk::from_header(empty_chunk_header().cast()) }) }
        }
    }
}

#[cfg(feature = "alloc")]
impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<Global, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    error_behavior_generic_methods! {
        impl
        /// This is equivalent to `Bump::with_capacity(512)`.
        #[must_use]
        for pub fn new
        /// This is equivalent to `Bump::try_with_capacity(512)`.
        for pub fn try_new
        fn generic_new() -> Self {
            Self::generic_new_in(Global)
        }

        /// Constructs a new `Bump` with a chunk of at least `size` bytes.
        impl
        #[must_use]
        for pub fn with_size
        for pub fn try_with_size
        fn generic_with_size(size: usize) -> Self {
            Self::generic_with_size_in(size, Global)
        }

        /// Constructs a new `Bump` with at least enough space for `layout`.
        impl
        #[must_use]
        for pub fn with_capacity
        for pub fn try_with_capacity
        fn generic_with_capacity(layout: Layout) -> Self {
            Self::generic_with_capacity_in(layout, Global)
        }
    }
}

/// These functions are only available if the `Bump` is [guaranteed allocated](crate#guaranteed_allocated-parameter).
impl<A, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
    bump_scope_methods!(BumpScopeGuardRoot, false);
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
    error_behavior_generic_methods! {
        impl
        /// This is equivalent to `Bump::with_size_in(512, allocator)`.
        for pub fn new_in
        /// This is equivalent to `Bump::try_with_size_in(512, allocator)`.
        for pub fn try_new_in
        fn generic_new_in(allocator: A) -> Self {
            Self::generic_with_size_in(DEFAULT_START_CHUNK_SIZE, allocator)
        }

        /// Constructs a new `Bump` with a chunk of at least `size` bytes with the provided allocator.
        impl
        for pub fn with_size_in
        for pub fn try_with_size_in
        fn generic_with_size_in(size: usize, allocator: A) -> Self {
            Ok(Self {
                chunk: Cell::new(RawChunk::new_in(
                    ChunkSize::new(size)?,
                    None,
                    allocator,
                )?),
            })
        }

        /// Constructs a new `Bump` with at least enough space for `layout` with the provided allocator.
        impl
        for pub fn with_capacity_in
        for pub fn try_with_capacity_in
        fn generic_with_capacity_in(layout: Layout, allocator: A) -> Self {
            Ok(Self {
                chunk: Cell::new(RawChunk::new_in(
                    ChunkSize::for_capacity(layout)?,
                    None,
                    allocator,
                )?),
            })
        }
    }

    // This needs `&mut self` to make sure that no allocations are alive.
    #[doc = crate::doc_fn_reset!()]
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
    #[doc = doc_align_cant_decrease!()]
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
    pub fn into_guaranteed_allocated(self) -> Bump<A, MIN_ALIGN, UP> {
        infallible(self.generic_into_guaranteed_allocated())
    }

    /// Converts this `Bump` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    pub fn try_into_guaranteed_allocated(self) -> Result<Bump<A, MIN_ALIGN, UP>, AllocError> {
        self.generic_into_guaranteed_allocated()
    }

    fn generic_into_guaranteed_allocated<E: ErrorBehavior>(self) -> Result<Bump<A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { self.cast_allocated() })
    }

    /// Mutably borrows `Bump` in a [guaranteed allocated](crate#guaranteed_allocated-parameter) state.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[cfg(not(no_global_oom_handling))]
    pub fn as_guaranteed_allocated_mut(&mut self) -> &mut Bump<A, MIN_ALIGN, UP> {
        infallible(self.generic_as_guaranteed_allocated_mut())
    }

    /// Mutably borrows `Bump` in an [guaranteed allocated](crate#guaranteed_allocated-parameter) state.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    pub fn try_as_guaranteed_allocated_mut(&mut self) -> Result<&mut Bump<A, MIN_ALIGN, UP>, AllocError> {
        self.generic_as_guaranteed_allocated_mut()
    }

    fn generic_as_guaranteed_allocated_mut<E: ErrorBehavior>(&mut self) -> Result<&mut Bump<A, MIN_ALIGN, UP>, E> {
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

impl<'b, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    From<&'b Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for &'b BumpScope<'b, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn from(value: &'b Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        value.as_scope()
    }
}

impl<'b, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    From<&'b mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>
    for &'b mut BumpScope<'b, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn from(value: &'b mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        value.as_mut_scope()
    }
}
