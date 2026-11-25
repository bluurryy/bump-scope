use core::{
    alloc::Allocator,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use alloc::alloc::Global;

use bump_scope_core::{
    BumpClaimGuard as CoreBumpClaimGuard,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG},
};

use crate::BumpScope;

// For docs.
#[allow(unused_imports)]
use crate::traits::*;

/// Returned from [`BumpScope::claim`].
#[must_use]
pub struct BumpClaimGuard<'b, 'a, const C: BumpConfig = DEFAULT_BUMP_CONFIG, A: Allocator = Global> {
    pub(crate) core: CoreBumpClaimGuard<'b, C, A>,
    marker: PhantomData<&'a u8>,
}

impl<'a, const C: BumpConfig, A: Allocator> Deref for BumpClaimGuard<'_, 'a, C, A> {
    type Target = BumpScope<'a, C, A>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: we use the valid 'a lifetime we used for construction
        unsafe { BumpScope::ref_from_core(&self.core) }
    }
}

impl<const C: BumpConfig, A: Allocator> DerefMut for BumpClaimGuard<'_, '_, C, A> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: we use the valid 'a lifetime we used for construction
        unsafe { BumpScope::mut_from_core(&mut self.core) }
    }
}

impl<'b, 'a, const C: BumpConfig, A: Allocator> BumpClaimGuard<'b, 'a, C, A> {
    #[inline(always)]
    pub(crate) fn new(original: &'b BumpScope<'a, C, A>) -> Self {
        Self {
            core: original.as_ref().claim(),
            marker: PhantomData,
        }
    }
}
