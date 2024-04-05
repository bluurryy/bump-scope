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
    bump_common_methods, chunk_size::ChunkSize, doc_align_cant_decrease, error_behavior_generic_methods, polyfill::pointer,
    BumpScope, BumpScopeGuardRoot, Checkpoint, ErrorBehavior, MinimumAlignment, RawChunk, Stats, SupportedMinimumAlignment,
    WithoutDealloc, WithoutShrink,
};

#[cfg(not(no_global_oom_handling))]
use crate::infallible;

#[cfg(test)]
use crate::WithDrop;

/// A bump allocator.
///
/// Every constructor of `Bump` needs to allocate memory.
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
> where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) chunk: Cell<RawChunk<UP, A>>,
}

// Sending Bumps when nothing is allocated is fine.
// When something is allocated Bump is borrowed and sending is not possible.
unsafe impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> Send for Bump<A, MIN_ALIGN, UP> where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment
{
}

impl<const MIN_ALIGN: usize, const UP: bool, A> UnwindSafe for Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone + UnwindSafe,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, A> RefUnwindSafe for Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone + UnwindSafe,
{
}

impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> Drop for Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn drop(&mut self) {
        unsafe {
            let chunk = self.chunk.get();
            chunk.for_each_prev(|chunk| chunk.deallocate());
            chunk.for_each_next(|chunk| chunk.deallocate());
            chunk.deallocate();
        }
    }
}

impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> Debug for Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.stats().debug_format("Bump", f)
    }
}

#[cfg(not(no_global_oom_handling))]
impl<A: Allocator + Clone + Default, const MIN_ALIGN: usize, const UP: bool> Default for Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn default() -> Self {
        Self::new_in(Default::default())
    }
}

#[cfg(feature = "alloc")]
impl<const MIN_ALIGN: usize, const UP: bool> Bump<Global, MIN_ALIGN, UP>
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

        /// Constructs a new, `Bump` with a chunk of at least `size` bytes.
        impl
        #[must_use]
        for pub fn with_size
        for pub fn try_with_size
        fn generic_with_size(size: usize) -> Self {
            Self::generic_with_size_in(size, Global)
        }

        /// Constructs a new, `Bump` with at least enough space for `layout`.
        impl
        #[must_use]
        for pub fn with_capacity
        for pub fn try_with_capacity
        fn generic_with_capacity(layout: Layout) -> Self {
            Self::generic_with_capacity_in(layout, Global)
        }
    }
}

impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    error_behavior_generic_methods! {
        impl
        /// This is equivalent to `Bump::with_capacity_in(512, allocator)`.
        for pub fn new_in
        /// This is equivalent to `Bump::try_with_capacity_in(512, allocator)`.
        for pub fn try_new_in
        fn generic_new_in(allocator: A) -> Self {
            Self::generic_with_size_in(512, allocator)
        }

        /// Constructs a new, `Bump` with a chunk of at least `size` bytes.
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

        /// Constructs a new, `Bump` with at least enough space for `layout`.
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

    bump_common_methods!(BumpScopeGuardRoot, false);

    /// Returns this `&Bump` as a `&BumpScope`.
    #[inline(always)]
    pub fn as_scope(&self) -> &BumpScope<A, MIN_ALIGN, UP> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &*pointer::from_ref(self).cast() }
    }

    /// Returns this `&mut Bump` as a `&mut BumpScope`.
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut BumpScope<A, MIN_ALIGN, UP> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &mut *pointer::from_mut(self).cast() }
    }

    /// Converts this `Bump` into a `Bump` with a new minimum alignment.
    #[inline(always)]
    pub fn into_aligned<const NEW_MIN_ALIGN: usize>(self) -> Bump<A, NEW_MIN_ALIGN, UP>
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
    pub fn as_aligned_mut<const NEW_MIN_ALIGN: usize>(&mut self) -> &mut Bump<A, NEW_MIN_ALIGN, UP>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.as_scope().must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align_mut() }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align<const NEW_MIN_ALIGN: usize>(self) -> Bump<A, NEW_MIN_ALIGN, UP>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        let chunk = self.chunk.get();
        mem::forget(self);

        Bump { chunk: Cell::new(chunk) }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align_mut<const NEW_MIN_ALIGN: usize>(&mut self) -> &mut Bump<A, NEW_MIN_ALIGN, UP>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        &mut *pointer::from_mut(self).cast::<Bump<A, NEW_MIN_ALIGN, UP>>()
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

impl<'b, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> From<&'b Bump<A, MIN_ALIGN, UP>>
    for &'b BumpScope<'b, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn from(value: &'b Bump<A, MIN_ALIGN, UP>) -> Self {
        value.as_scope()
    }
}

impl<'b, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> From<&'b mut Bump<A, MIN_ALIGN, UP>>
    for &'b mut BumpScope<'b, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn from(value: &'b mut Bump<A, MIN_ALIGN, UP>) -> Self {
        value.as_mut_scope()
    }
}
