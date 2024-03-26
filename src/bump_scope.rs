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

use allocator_api2::alloc::Allocator;

#[cfg(feature = "alloc")]
use allocator_api2::alloc::Global;

use crate::{
    bump_align_guard::BumpAlignGuard,
    bump_common_methods, const_param_assert, doc_align_cant_decrease,
    polyfill::{nonnull, pointer},
    ArrayLayout, BumpScopeGuard, Checkpoint, ErrorBehavior, LayoutTrait, MinimumAlignment, RawChunk, SizedTypeProperties,
    Stats, SupportedMinimumAlignment, WithoutDealloc, WithoutShrink,
};

#[cfg(test)]
use crate::WithDrop;

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
    #[cfg(feature = "alloc")] A = Global,
    #[cfg(not(feature = "alloc"))] A,
    const MIN_ALIGN: usize = 1,
    const UP: bool = true,
> {
    pub(crate) chunk: Cell<RawChunk<UP, A>>,

    /// Marks the lifetime of the mutably borrowed `BumpScopeGuard(Root)`.
    marker: PhantomData<&'a ()>,
}

impl<const MIN_ALIGN: usize, const UP: bool, A> UnwindSafe for BumpScope<'_, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone + UnwindSafe,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, A> RefUnwindSafe for BumpScope<'_, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone + UnwindSafe,
{
}

impl<A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> Debug for BumpScope<'_, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.stats().debug_format("BumpScope", f)
    }
}

impl<'a, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> BumpScope<'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    pub(crate) unsafe fn new_unchecked(chunk: RawChunk<UP, A>) -> Self {
        Self {
            chunk: Cell::new(chunk),
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn consolidate_greed<T>(&self, mut start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        let end = nonnull::add(start, len);

        if UP {
            self.set_pos(nonnull::addr(end), T::ALIGN);
            nonnull::slice_from_raw_parts(start, len)
        } else {
            {
                let dst_end = nonnull::add(start, cap);
                let dst = nonnull::sub(dst_end, len);

                // We only copy if we can do so nonoverlappingly.
                if dst >= end {
                    nonnull::copy_nonoverlapping(start, dst, len);
                    start = dst;
                }
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

                // We only copy if we can do so nonoverlappingly.
                if dst_end <= start {
                    nonnull::copy_nonoverlapping(start, dst, len);
                    start = dst;
                    end = dst_end;
                }
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
        debug_assert_eq!(nonnull::addr(chunk.pos()).get() % current_align, 0);

        unsafe { chunk.set_pos_addr(pos.get()) }

        if current_align < MIN_ALIGN {
            chunk.align_pos_to::<MIN_ALIGN>();
        }
    }

    #[inline(always)]
    pub(crate) fn alloc_greedy<B: ErrorBehavior, T>(&self, cap: usize) -> Result<(NonNull<T>, usize), B> {
        let Range { start, end } = self.alloc_greedy_range::<B, T>(cap)?;

        // NB: We can't use `sub_ptr`, because the size is not a multiple of `T`'s.
        let capacity = unsafe { nonnull::byte_sub_ptr(end, start) } / T::SIZE;

        Ok((start, capacity))
    }

    #[inline(always)]
    pub(crate) fn alloc_greedy_rev<B: ErrorBehavior, T>(&self, cap: usize) -> Result<(NonNull<T>, usize), B> {
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
    fn alloc_greedy_range<B: ErrorBehavior, T>(&self, cap: usize) -> Result<Range<NonNull<T>>, B> {
        let layout = match ArrayLayout::array::<T>(cap) {
            Ok(ok) => ok,
            Err(_) => return Err(B::capacity_overflow()),
        };

        let range = match self.chunk.get().alloc_greedy::<MIN_ALIGN, true>(layout) {
            Some(ptr) => ptr,
            None => self.alloc_greedy_in_another_chunk(layout)?,
        };

        Ok(range.start.cast::<T>()..range.end.cast::<T>())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_greedy_in_another_chunk<E: ErrorBehavior>(
        &self,
        layout: ArrayLayout,
    ) -> Result<Range<NonNull<u8>>, E> {
        unsafe { self.do_custom_alloc_in_another_chunk(layout, RawChunk::alloc_greedy::<MIN_ALIGN, false>) }
    }

    #[inline(always)]
    pub(crate) fn alloc_in_current_chunk(&self, layout: Layout) -> Option<NonNull<u8>> {
        self.chunk.get().alloc::<MIN_ALIGN, false, false, _>(layout)
    }

    /// Allocation slow path.
    /// The active chunk must *not* have space for `layout`.
    #[cold]
    #[inline(never)]
    pub(crate) fn alloc_in_another_chunk<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        unsafe { self.do_custom_alloc_in_another_chunk(layout, RawChunk::alloc::<MIN_ALIGN, false, false, _>) }
    }

    #[inline(always)]
    pub(crate) fn do_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        let result = match self.chunk.get().alloc::<MIN_ALIGN, true, true, _>(ArrayLayout::new::<T>()) {
            Some(ptr) => Ok(ptr),
            None => self.do_alloc_sized_in_another_chunk::<E, T>(),
        };

        match result {
            Ok(ptr) => Ok(ptr.cast()),
            Err(error) => Err(error),
        }
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn do_alloc_sized_in_another_chunk<E: ErrorBehavior, T>(&self) -> Result<NonNull<u8>, E>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        let layout = Layout::new::<T>();
        self.alloc_in_another_chunk(layout)
    }

    #[inline(always)]
    pub(crate) fn do_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<[T]>, E> {
        let layout = match ArrayLayout::array::<T>(len) {
            Ok(layout) => layout,
            Err(_) => return Err(E::capacity_overflow()),
        };

        let result = match self.chunk.get().alloc::<MIN_ALIGN, false, true, _>(layout) {
            Some(ptr) => Ok(ptr),
            None => unsafe { self.do_alloc_slice_in_another_chunk::<E, T>(len) },
        };

        match result {
            Ok(ptr) => Ok(nonnull::slice_from_raw_parts(ptr.cast(), len)),
            Err(error) => Err(error),
        }
    }

    #[inline(always)]
    pub(crate) fn do_alloc_slice_for<E: ErrorBehavior, T>(&self, value: &[T]) -> Result<NonNull<[T]>, E> {
        let layout = ArrayLayout::for_value(value);

        let result = match self.chunk.get().alloc::<MIN_ALIGN, false, true, _>(layout) {
            Some(ptr) => Ok(ptr),
            None => unsafe { self.do_alloc_slice_in_another_chunk::<E, T>(value.len()) },
        };

        match result {
            Ok(ptr) => Ok(nonnull::slice_from_raw_parts(ptr.cast(), value.len())),
            Err(error) => Err(error),
        }
    }

    #[cold]
    #[inline(never)]
    pub(crate) unsafe fn do_alloc_slice_in_another_chunk<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<u8>, E>
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        let layout = Layout::from_size_align_unchecked(len * T::SIZE, T::ALIGN);
        self.alloc_in_another_chunk(layout)
    }

    #[inline(always)]
    pub(crate) fn force_align<const ALIGN: usize>(&self)
    where
        MinimumAlignment<ALIGN>: SupportedMinimumAlignment,
    {
        self.chunk.get().align_pos_to::<ALIGN>();
    }

    #[inline(always)]
    pub(crate) fn force_align_more<const NEW_MIN_ALIGN: usize>(&self)
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        const_param_assert! {
            (const MIN_ALIGN: usize, const NEW_MIN_ALIGN: usize) => MIN_ALIGN <= NEW_MIN_ALIGN, "`into_aligned` or `as_aligned_mut` can't decrease the minimum alignment"
        }

        if NEW_MIN_ALIGN > MIN_ALIGN {
            self.force_align::<NEW_MIN_ALIGN>();
        }
    }

    pub(crate) fn do_alloc_no_bump_for<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        let result = match self.chunk.get().alloc_no_bump_for::<MIN_ALIGN, T>() {
            Some(ptr) => Ok(ptr),
            None => unsafe {
                self.do_custom_alloc_in_another_chunk(Layout::new::<T>(), |chunk, _| {
                    chunk.alloc_no_bump_for::<MIN_ALIGN, T>()
                })
            },
        };

        match result {
            Ok(ptr) => Ok(ptr.cast()),
            Err(error) => Err(error),
        }
    }

    /// # Safety
    ///
    /// `allocate` on the new chunk created by `RawChunk::append_for` with the layout `layout` must return `Some`.
    #[inline(always)]
    pub(crate) unsafe fn do_custom_alloc_in_another_chunk<E: ErrorBehavior, L: LayoutTrait, R>(
        &self,
        layout: L,
        mut allocate: impl FnMut(RawChunk<UP, A>, L) -> Option<R>,
    ) -> Result<R, E> {
        while let Some(chunk) = self.chunk.get().next() {
            // We don't reset the chunk position when we leave a scope, so we need to do it here.
            chunk.reset();

            self.chunk.set(chunk);

            if let Some(ptr) = allocate(chunk, layout) {
                return Ok(ptr);
            }
        }

        // there is no chunk that fits, we need a new chunk
        let new_chunk = self.chunk.get().append_for(layout.layout())?;
        self.chunk.set(new_chunk);

        if let Some(ptr) = allocate(new_chunk, layout) {
            Ok(ptr)
        } else {
            // SAFETY: We just appended a chunk for that specific layout, it must have enough space.
            unsafe { core::hint::unreachable_unchecked() }
        }
    }

    bump_common_methods!(BumpScopeGuard, true);

    /// Returns `&self` as is. This is used in for macros that support both `Bump` and `BumpScope`, like [`bump_vec!`](crate::bump_vec!).
    #[inline(always)]
    pub fn as_scope(&self) -> &Self {
        self
    }

    /// Returns `&mut self` as is. This is useful for macros that support both `Bump` and `BumpScope`, like [`bump_vec!`](crate::bump_vec!).
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut Self {
        self
    }

    /// Converts this `BumpScope` into a `BumpScope` with a new minimum alignment.
    ///
    #[doc = doc_align_cant_decrease!()]
    #[inline(always)]
    pub fn into_aligned<const NEW_MIN_ALIGN: usize>(self) -> BumpScope<'a, A, NEW_MIN_ALIGN, UP>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.force_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align() }
    }

    /// Mutably borrows `BumpScope` with a new minimum alignment.
    ///
    #[doc = doc_align_cant_decrease!()]
    #[inline(always)]
    pub fn as_aligned_mut<const NEW_MIN_ALIGN: usize>(&mut self) -> &mut BumpScope<'a, A, NEW_MIN_ALIGN, UP>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.force_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align_mut() }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align<const NEW_MIN_ALIGN: usize>(self) -> BumpScope<'a, A, NEW_MIN_ALIGN, UP>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        BumpScope {
            chunk: self.chunk,
            marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align_mut<const NEW_MIN_ALIGN: usize>(&mut self) -> &mut BumpScope<'a, A, NEW_MIN_ALIGN, UP>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        unsafe { &mut *pointer::from_mut(self).cast::<BumpScope<'a, A, NEW_MIN_ALIGN, UP>>() }
    }

    /// # Safety
    ///
    /// - `self` must not be used until this clone is gone
    #[inline(always)]
    pub(crate) unsafe fn clone_unchecked(&self) -> BumpScope<'a, A, MIN_ALIGN, UP> {
        BumpScope::new_unchecked(self.chunk.get())
    }

    /// Converts this `BumpScope` into a raw pointer.
    #[inline]
    #[must_use]
    pub fn into_raw(self) -> NonNull<()> {
        let this = ManuallyDrop::new(self);
        this.chunk.get().header_ptr().cast()
    }

    /// Converts the raw pointer that was created with [`into_raw`](BumpScope::into_raw) back into a `BumpScope`.
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
