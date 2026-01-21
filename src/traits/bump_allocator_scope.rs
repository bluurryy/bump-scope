use crate::{
    BaseAllocator, BumpScope,
    bump_align_guard::BumpAlignGuard,
    polyfill::transmute_mut,
    settings::{BumpAllocatorSettings, MinimumAlignment, SupportedMinimumAlignment, True},
    stats::Stats,
    traits::{BumpAllocator, MutBumpAllocatorCoreScope},
};

/// A bump allocator scope.
pub trait BumpAllocatorScope<'a>: BumpAllocator + MutBumpAllocatorCoreScope<'a> {
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    fn stats(&self) -> Stats<'a, Self::Allocator, Self::Settings>;

    /// Forwards to [`BumpScope::with_settings`].
    fn with_settings<NewS>(self) -> BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings;

    /// Forwards to [`BumpScope::borrow_with_settings`].
    fn borrow_with_settings<NewS>(&self) -> &BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings;

    /// Forwards to [`BumpScope::borrow_mut_with_settings`].
    fn borrow_mut_with_settings<NewS>(&mut self) -> &mut BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings;

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
    fn aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(
            &mut BumpScope<
                'a,
                Self::Allocator,
                <Self::Settings as BumpAllocatorSettings>::WithMinimumAlignment<NEW_MIN_ALIGN>,
            >,
        ) -> R,
    ) -> R
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment;

    /// Returns a reference to the base allocator.
    ///
    /// This is only available if the bump allocator is guaranteed-allocated.
    /// You can always get a reference to the base allocator using [`get_allocator`].
    ///
    /// [`get_allocator`]: BumpAllocatorScope::get_allocator
    #[must_use]
    #[inline(always)]
    fn allocator(&self) -> &'a Self::Allocator
    where
        Self::Settings: BumpAllocatorSettings<GuaranteedAllocated = True>,
    {
        self.stats().current_chunk().allocator()
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    fn get_allocator(&self) -> Option<&'a Self::Allocator> {
        self.stats().get_current_chunk().map(|c| c.allocator())
    }
}

impl<'a, A, S> BumpAllocatorScope<'a> for BumpScope<'a, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline]
    fn stats(&self) -> Stats<'a, Self::Allocator, Self::Settings> {
        self.chunk.get().stats()
    }

    #[inline]
    fn with_settings<NewS>(self) -> BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        BumpScope::with_settings(self)
    }

    #[inline]
    fn borrow_with_settings<NewS>(&self) -> &BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        BumpScope::borrow_with_settings(self)
    }

    #[inline]
    fn borrow_mut_with_settings<NewS>(&mut self) -> &mut BumpScope<'a, Self::Allocator, NewS>
    where
        NewS: BumpAllocatorSettings,
        Self: Sized,
    {
        BumpScope::borrow_mut_with_settings(self)
    }

    #[inline]
    fn aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(
            &mut BumpScope<
                'a,
                Self::Allocator,
                <Self::Settings as BumpAllocatorSettings>::WithMinimumAlignment<NEW_MIN_ALIGN>,
            >,
        ) -> R,
    ) -> R
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        if NEW_MIN_ALIGN < S::MIN_ALIGN {
            let guard = BumpAlignGuard::new(self);

            // SAFETY: bump is already aligned to `NEW_MIN_ALIGN` and the guard will ensure
            // that the bump pointer will again be aligned to `MIN_ALIGN` once it drops
            let bump = unsafe { transmute_mut(guard.scope) };

            f(bump)
        } else {
            self.align::<NEW_MIN_ALIGN>();

            // SAFETY: we aligned the bump pointer
            let bump = unsafe { transmute_mut(self) };

            f(bump)
        }
    }
}
