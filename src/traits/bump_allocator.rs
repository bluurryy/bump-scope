use core::ptr;

use crate::{
    BaseAllocator, Bump, BumpScope, BumpScopeGuard,
    alloc::AllocError,
    polyfill::transmute_mut,
    settings::{BumpAllocatorSettings, MinimumAlignment, SupportedMinimumAlignment},
    traits::MutBumpAllocatorTyped,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// A bump allocator, generic over [`Bump`] and [`BumpScope`].
///
/// Many useful methods are only available for a [`BumpAllocatorScope`].
/// You can access them by converting to a `BumpScope` using [`as_scope`] and [`as_mut_scope`].
///
/// [`BumpAllocatorScope`]: crate::traits::BumpAllocatorScope
/// [`as_scope`]: BumpAllocator::as_scope
/// [`as_mut_scope`]: BumpAllocator::as_mut_scope
pub trait BumpAllocator: MutBumpAllocatorTyped + Sized {
    /// The base allocator.
    type Allocator: BaseAllocator<<Self::Settings as BumpAllocatorSettings>::GuaranteedAllocated>;

    /// The bump allocator settings.
    type Settings: BumpAllocatorSettings;

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
    /// # Panics
    /// Panics if the allocation fails.
    /// Allocation only occurs if the bump allocator has not yet allocated a chunk.
    ///
    /// Panics if the bump allocator is currently [claimed].
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
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    #[must_use]
    #[cfg(feature = "panic-on-alloc")]
    fn scope_guard(&mut self) -> BumpScopeGuard<'_, Self::Allocator, Self::Settings> {
        panic_on_error(self.as_mut_scope().generic_scope_guard())
    }

    /// Creates a new [`BumpScopeGuard`].
    ///
    /// This allows for creation of child scopes.
    ///
    /// # Panics
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    /// Allocation only occurs if the bump allocator has not yet allocated a chunk.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    ///
    /// {
    ///     let mut guard = bump.try_scope_guard()?;
    ///     let bump = guard.scope();
    ///     bump.alloc_str("Hello, world!");
    ///     assert_eq!(bump.stats().allocated(), 13);
    /// }
    ///
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    fn try_scope_guard(&mut self) -> Result<BumpScopeGuard<'_, Self::Allocator, Self::Settings>, AllocError> {
        self.as_mut_scope().generic_scope_guard()
    }

    /// Calls `f` with a new child scope.
    ///
    /// # Panics
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// Panics if the allocation fails.
    /// Allocation only occurs if the bump allocator has not yet allocated a chunk.
    ///
    /// # Examples
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
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn scoped<R>(&mut self, f: impl FnOnce(BumpScope<'_, Self::Allocator, Self::Settings>) -> R) -> R {
        let mut guard = self.scope_guard();
        f(guard.scope())
    }

    /// Calls `f` with a new child scope.
    ///
    /// # Panics
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    /// Allocation only occurs if the bump allocator has not yet allocated a chunk.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    ///
    /// bump.try_scoped(|bump| {
    ///     bump.alloc_str("Hello, world!");
    ///     assert_eq!(bump.stats().allocated(), 13);
    /// })?;
    ///
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    #[inline(always)]
    fn try_scoped<R>(
        &mut self,
        f: impl FnOnce(BumpScope<'_, Self::Allocator, Self::Settings>) -> R,
    ) -> Result<R, AllocError> {
        let mut guard = self.try_scope_guard()?;
        Ok(f(guard.scope()))
    }

    /// Calls `f` with a new child scope of a new minimum alignment.
    ///
    /// # Panics
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// Panics if the allocation fails.
    /// Allocation only occurs if the bump allocator has not yet allocated a chunk.
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
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn scoped_aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(
            BumpScope<'_, Self::Allocator, <Self::Settings as BumpAllocatorSettings>::WithMinimumAlignment<NEW_MIN_ALIGN>>,
        ) -> R,
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

    /// Calls `f` with a new child scope of a new minimum alignment.
    ///
    /// # Panics
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    /// Allocation only occurs if the bump allocator has not yet allocated a chunk.
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
    /// bump.try_scoped_aligned::<8, ()>(|bump| {
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
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    #[inline(always)]
    fn try_scoped_aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(
            BumpScope<'_, Self::Allocator, <Self::Settings as BumpAllocatorSettings>::WithMinimumAlignment<NEW_MIN_ALIGN>>,
        ) -> R,
    ) -> Result<R, AllocError>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        // This guard will reset the bump pointer to the current position, which is aligned to `MIN_ALIGN`.
        let mut guard = self.try_scope_guard()?;
        let scope = guard.scope();
        scope.align::<NEW_MIN_ALIGN>();
        Ok(f(unsafe { scope.cast() }))
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

    // Overwriting this implementation because `BumpScopeGuardRoot` is slightly more efficient.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn scoped<R>(&mut self, f: impl FnOnce(BumpScope<'_, Self::Allocator, Self::Settings>) -> R) -> R {
        let mut guard = self.scope_guard();
        f(guard.scope())
    }

    // Overwriting this implementation because `BumpScopeGuardRoot` is slightly more efficient.
    #[inline(always)]
    fn try_scoped<R>(
        &mut self,
        f: impl FnOnce(BumpScope<'_, Self::Allocator, Self::Settings>) -> R,
    ) -> Result<R, AllocError> {
        let mut guard = self.try_scope_guard()?;
        Ok(f(guard.scope()))
    }

    // Overwriting this implementation because `BumpScopeGuardRoot` is slightly more efficient.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn scoped_aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(
            BumpScope<'_, Self::Allocator, <Self::Settings as BumpAllocatorSettings>::WithMinimumAlignment<NEW_MIN_ALIGN>>,
        ) -> R,
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

    // Overwriting this implementation because `BumpScopeGuardRoot` is slightly more efficient.
    #[inline(always)]
    fn try_scoped_aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(
            BumpScope<'_, Self::Allocator, <Self::Settings as BumpAllocatorSettings>::WithMinimumAlignment<NEW_MIN_ALIGN>>,
        ) -> R,
    ) -> Result<R, AllocError>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        // This guard will reset the bump pointer to the current position, which is aligned to `MIN_ALIGN`.
        let mut guard = self.try_scope_guard()?;
        let scope = guard.scope();
        scope.align::<NEW_MIN_ALIGN>();
        Ok(f(unsafe { scope.cast() }))
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
