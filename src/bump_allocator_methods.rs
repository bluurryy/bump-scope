use crate::{BaseAllocator, BumpScope, BumpScopeGuard, MinimumAlignment, MutBumpAllocatorScope, SupportedMinimumAlignment};

///
/// # Safety
///
/// Just don't implement it lmao.
pub unsafe trait BumpAllocatorMethods<'a>: MutBumpAllocatorScope<'a> + Sized {
    /// Some `BumpScope`.
    type Scope<'scope>;

    /// This is returned from [`scope_guard`](Self::scope_guard).
    type ScopeGuard;

    /// Creates a new bump scope guard.
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
    fn scope_guard(&mut self) -> Self::ScopeGuard
    where
        Self: CanCreateScopes;

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
    fn scoped<R>(&mut self, f: impl FnOnce(Self::Scope<'_>) -> R) -> R
    where
        Self: CanCreateScopes;
}

// TODO: add if not implemented lint or whatever it's called
pub unsafe trait CanCreateScopes {}

unsafe impl<'scope, A, const MIN_ALIGN: usize, const UP: bool> CanCreateScopes for BumpScope<'scope, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
}

unsafe impl<'scope, A, const MIN_ALIGN: usize, const UP: bool> CanCreateScopes for &BumpScope<'scope, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
}

unsafe impl<'scope, A, const MIN_ALIGN: usize, const UP: bool> CanCreateScopes for &mut BumpScope<'scope, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
{
}

unsafe impl<'scope, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> BumpAllocatorMethods<'scope>
    for BumpScope<'scope, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Scope<'child_scope> = BumpScope<'child_scope, A, MIN_ALIGN, UP>;

    type ScopeGuard = BumpScopeGuard<'scope, A, MIN_ALIGN, UP>;

    fn scope_guard(&mut self) -> Self::ScopeGuard
    where
        Self: CanCreateScopes,
    {
        unsafe { BumpScopeGuard::new_unchecked(self.chunk.get()) }
    }

    fn scoped<R>(&mut self, f: impl FnOnce(Self::Scope<'_>) -> R) -> R
    where
        Self: CanCreateScopes,
    {
        let mut guard = self.scope_guard();
        f(guard.scope())
    }
}
