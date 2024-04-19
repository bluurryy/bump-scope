use allocator_api2::alloc::Allocator;

use crate::{BumpScope, MinimumAlignment, SupportedMinimumAlignment};

/// Aligns the bump pointer on drop.
///
/// This is useful in unsafe contexts where the alignment is changed and we have to change it back.
/// The `BumpScope` is in an invalid state when the bump pointer alignment does not match `MIN_ALIGN`.
/// So `drop` ***must*** be called to return the bump scope to a valid state.
pub(crate) struct BumpAlignGuard<'b, 'a, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool, const CONST_NEW: bool>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) scope: &'b mut BumpScope<'a, A, MIN_ALIGN, UP, CONST_NEW>,
}

impl<'b, 'a, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool, const CONST_NEW: bool> Drop
    for BumpAlignGuard<'b, 'a, A, MIN_ALIGN, UP, CONST_NEW>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.scope.chunk.get().align_pos_to::<MIN_ALIGN>();
    }
}

impl<'b, 'a, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool, const CONST_NEW: bool>
    BumpAlignGuard<'b, 'a, A, MIN_ALIGN, UP, CONST_NEW>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    pub fn new(scope: &'b mut BumpScope<'a, A, MIN_ALIGN, UP, CONST_NEW>) -> Self {
        Self { scope }
    }
}
