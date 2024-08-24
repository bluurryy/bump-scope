use crate::{BaseAllocator, BumpScope, MinimumAlignment, SupportedMinimumAlignment};

/// Aligns the bump pointer on drop.
///
/// This is useful in unsafe contexts where the alignment is changed and we have to change it back.
/// The `BumpScope` is in an invalid state when the bump pointer alignment does not match `MIN_ALIGN`.
/// So `drop` ***must*** be called to return the bump scope to a valid state.
pub(crate) struct BumpAlignGuard<'b, 'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    pub(crate) scope: &'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>,
}

impl<'b, 'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for BumpAlignGuard<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.scope.chunk.get().align_pos_to::<MIN_ALIGN>();
    }
}

impl<'b, 'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpAlignGuard<'b, 'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    pub(crate) fn new(scope: &'b mut BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        Self { scope }
    }
}
