use crate::{BumpScope, align_pos, settings::BumpAllocatorSettings};

/// Aligns the bump pointer on drop.
///
/// This is useful in unsafe contexts where the alignment is changed and we have to change it back.
/// The `BumpScope` is in an invalid state when the bump pointer alignment does not match `MIN_ALIGN`.
/// So `drop` ***must*** be called to return the bump scope to a valid state.
pub(crate) struct BumpAlignGuard<'b, 'a, A, S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) scope: &'b mut BumpScope<'a, A, S>,
}

impl<A, S> Drop for BumpAlignGuard<'_, '_, A, S>
where
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn drop(&mut self) {
        if let Some(chunk) = self.scope.chunk.get().guaranteed_allocated() {
            let pos = chunk.pos().addr();
            let addr = align_pos(S::UP, S::MIN_ALIGN, pos);
            unsafe { chunk.set_pos_addr(addr) };
        }
    }
}

impl<'b, 'a, A, S> BumpAlignGuard<'b, 'a, A, S>
where
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    pub(crate) fn new(scope: &'b mut BumpScope<'a, A, S>) -> Self {
        Self { scope }
    }
}
