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
pub(crate) struct BumpAlignGuard<'b, 'a, const C: BumpConfig, A> {
    pub(crate) scope: &'b mut BumpScope<'a, C, A>,
}

impl<const C: BumpConfig, A> Drop for BumpAlignGuard<'_, '_, C, A> {
    #[inline(always)]
    fn drop(&mut self) {
        if let Some(chunk) = self.scope.raw.chunk.get().as_allocated() {
            let pos = chunk.pos().addr().get();
            let addr = align_pos(config!(upwards), config!(min_align), pos);
            unsafe { chunk.set_pos_addr(addr) };
        }
    }
}

impl<'b, 'a, const C: BumpConfig, A> BumpAlignGuard<'b, 'a, C, A> {
    #[inline(always)]
    pub(crate) fn new(scope: &'b mut BumpScope<'a, C, A>) -> Self {
        Self { scope }
    }
}
