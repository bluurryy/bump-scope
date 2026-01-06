use core::ptr;

use crate::{
    BaseAllocator, Bump, BumpScope, BumpScopeGuard,
    alloc::Allocator,
    polyfill::transmute_mut,
    settings::{BumpAllocatorSettings, MinimumAlignment, SupportedMinimumAlignment, True},
    traits::MutBumpAllocatorTyped,
};

/// A bump allocator, generic over [`Bump`] and [`BumpScope`].
pub trait BumpAllocator: MutBumpAllocatorTyped {
    /// The base allocator.
    type Allocator: Allocator + Clone;

    /// The bump allocator settings.
    type Settings: BumpAllocatorSettings;

    // TODO: check that must_use on the trait method works, or if we have to add it to the impls
    /// Returns this bump allocator as a `&BumpScope`.
    #[must_use]
    fn as_scope(&self) -> &BumpScope<'_, Self::Allocator, Self::Settings>;

    /// Returns this bump allocator as a `&mut BumpScope`.
    #[must_use]
    fn as_mut_scope(&mut self) -> &mut BumpScope<'_, Self::Allocator, Self::Settings>;

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
    fn scope_guard(&mut self) -> BumpScopeGuard<'_, Self::Allocator, Self::Settings>
    where
        Self::Settings: BumpAllocatorSettings<GuaranteedAllocated = True>,
    {
        BumpScopeGuard::new(self.as_mut_scope())
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
    fn scoped<R>(&mut self, f: impl FnOnce(BumpScope<'_, Self::Allocator, Self::Settings>) -> R) -> R
    where
        Self::Settings: BumpAllocatorSettings<GuaranteedAllocated = True>,
    {
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
    fn scoped_aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(
            BumpScope<'_, Self::Allocator, <Self::Settings as BumpAllocatorSettings>::WithMinimumAlignment<NEW_MIN_ALIGN>>,
        ) -> R,
    ) -> R
    where
        Self::Settings: BumpAllocatorSettings<GuaranteedAllocated = True>,
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        // This guard will reset the bump pointer to the current position, which is aligned to `MIN_ALIGN`.
        let mut guard = self.scope_guard();
        let scope = guard.scope();
        scope.align::<NEW_MIN_ALIGN>();
        f(unsafe { scope.cast() })
    }
}

impl<A, S> BumpAllocator for Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    type Allocator = A;
    type Settings = S;

    #[inline(always)]
    fn as_scope(&self) -> &BumpScope<'_, Self::Allocator, Self::Settings> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &*ptr::from_ref(self).cast() }
    }

    #[inline(always)]
    fn as_mut_scope(&mut self) -> &mut BumpScope<'_, Self::Allocator, Self::Settings> {
        // SAFETY: we shorten the lifetime that allocations will have which is sound
        unsafe { transmute_mut(self) }
    }
}

impl<A, S> BumpAllocator for BumpScope<'_, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    type Allocator = A;
    type Settings = S;

    #[inline(always)]
    fn as_scope(&self) -> &BumpScope<'_, Self::Allocator, Self::Settings> {
        self
    }

    #[inline(always)]
    fn as_mut_scope(&mut self) -> &mut BumpScope<'_, Self::Allocator, Self::Settings> {
        // SAFETY: we shorten the lifetime that allocations will have which is sound
        unsafe { transmute_mut(self) }
    }
}
