use core::{
    alloc::Allocator,
    fmt,
    ops::{Deref, DerefMut},
};

use crate::{BumpScope, Checkpoint, config::BumpConfig};

/// Returned from [`BumpScope::scope_guard`].
///
/// [`BumpScope::scope_guard`]: crate::BumpScope::scope_guard
#[must_use]
pub struct BumpScopeGuard<'a, const C: BumpConfig, A: Allocator> {
    scope: &'a mut BumpScope<C, A>,
    checkpoint: Checkpoint,
}

impl<const C: BumpConfig, A: Allocator> Drop for BumpScopeGuard<'_, C, A> {
    #[inline(always)]
    fn drop(&mut self) {
        self.reset();
    }
}

impl<const C: BumpConfig, A: Allocator> Deref for BumpScopeGuard<'_, C, A> {
    type Target = BumpScope<C, A>;

    fn deref(&self) -> &Self::Target {
        self.scope
    }
}

impl<const C: BumpConfig, A: Allocator> DerefMut for BumpScopeGuard<'_, C, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.scope
    }
}

impl<const C: BumpConfig, A: Allocator> fmt::Debug for BumpScopeGuard<'_, C, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.scope.fmt(f)
    }
}

impl<'a, const C: BumpConfig, A: Allocator> BumpScopeGuard<'a, C, A> {
    #[inline(always)]
    pub(crate) fn new(scope: &'a mut BumpScope<C, A>) -> Self {
        let checkpoint = scope.checkpoint();
        Self { scope, checkpoint }
    }

    /// Frees the memory taken up by allocations made since creation of this bump scope guard.
    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe { self.scope.reset_to(self.checkpoint) }
    }
}
