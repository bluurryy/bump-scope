use core::{alloc::Allocator, fmt::Debug};

use alloc::alloc::Global;

use bump_scope_core::{
    BumpScopeGuard as CoreBumpScopeGuard,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG},
};

use crate::BumpScope;

/// Returned from [`BumpScope::scope_guard`].
///
/// [`BumpScope::scope_guard`]: crate::BumpScope::scope_guard
#[must_use]
pub struct BumpScopeGuard<'a, const C: BumpConfig = DEFAULT_BUMP_CONFIG, A: Allocator = Global> {
    core: CoreBumpScopeGuard<'a, C, A>,
}

impl<const C: BumpConfig, A: Allocator> Debug for BumpScopeGuard<'_, C, A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.core.fmt(f)
    }
}

impl<const C: BumpConfig, A: Allocator> Drop for BumpScopeGuard<'_, C, A> {
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<'a, const C: BumpConfig, A: Allocator> BumpScopeGuard<'a, C, A> {
    #[inline(always)]
    pub(crate) fn new(scope: &'a mut BumpScope<'_, C, A>) -> Self {
        Self {
            core: scope.as_mut().scope_guard(),
        }
    }

    /// Returns a new `BumpScope`.
    #[inline(always)]
    pub fn scope(&mut self) -> &mut BumpScope<'_, C, A> {
        // SAFETY: the allocation lifetime is our lifetime
        unsafe { BumpScope::mut_from_core(&mut self.core) }
    }

    /// Frees the memory taken up by allocations made since creation of this bump scope guard.
    #[inline(always)]
    pub fn reset(&mut self) {
        self.core.reset();
    }
}
