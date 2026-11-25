use core::alloc::Allocator;

use crate::{
    BumpScope,
    config::{BumpConfig, config},
    util::align_pos,
};

/// Aligns the bump pointer on drop.
///
/// This is useful in unsafe contexts where the alignment is changed and we have to change it back.
/// The `BumpScope` is in an invalid state when the bump pointer alignment does not match `MIN_ALIGN`.
/// So `drop` ***must*** be called to return the bump scope to a valid state.
pub(crate) struct BumpAlignGuard<'a, const C: BumpConfig, A: Allocator> {
    pub(crate) scope: &'a mut BumpScope<C, A>,
}

impl<const C: BumpConfig, A: Allocator> Drop for BumpAlignGuard<'_, C, A> {
    #[inline(always)]
    fn drop(&mut self) {
        if self.scope.stats().current_chunk().is_none() {
            return;
        }

        let pos = self.scope.pos().addr().get();
        let addr = align_pos(config!(upwards), config!(min_align), pos);
        unsafe { self.scope.set_pos(addr) };
    }
}

impl<'a, const C: BumpConfig, A: Allocator> BumpAlignGuard<'a, C, A> {
    #[inline(always)]
    pub(crate) fn new(scope: &'a mut BumpScope<C, A>) -> Self {
        Self { scope }
    }
}
