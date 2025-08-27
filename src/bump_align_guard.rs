use crate::{BumpScope, MinimumAlignment, SupportedMinimumAlignment, align_pos};

/// Aligns the bump pointer on drop.
///
/// This is useful in unsafe contexts where the alignment is changed and we have to change it back.
/// The `BumpScope` is in an invalid state when the bump pointer alignment does not match `MIN_ALIGN`.
/// So `drop` ***must*** be called to return the bump scope to a valid state.
pub(crate) struct BumpAlignGuard<
    'b,
    'a,
    A,
    const MIN_ALIGN: usize,
    const UP: bool,
    const GUARANTEED_ALLOCATED: bool,
    const DEALLOCATES: bool,
> where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) scope: &'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED, DEALLOCATES>,
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, const DEALLOCATES: bool> Drop
    for BumpAlignGuard<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn drop(&mut self) {
        if let Some(chunk) = self.scope.chunk.get().guaranteed_allocated() {
            let pos = chunk.pos().addr();
            let addr = align_pos::<MIN_ALIGN, UP>(pos);
            unsafe { chunk.set_pos_addr(addr) };
        }
    }
}

impl<'b, 'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, const DEALLOCATES: bool>
    BumpAlignGuard<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED, DEALLOCATES>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    pub(crate) fn new(scope: &'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED, DEALLOCATES>) -> Self {
        Self { scope }
    }
}
