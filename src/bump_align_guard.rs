use allocator_api2::alloc::Allocator;

use crate::{BumpScope, MinimumAlignment, SupportedMinimumAlignment};

/// Aligns the bump pointer on drop.
///
/// This can't be a safe public api. Bump validity relies on drop being called. We can not enforce drop being called if this were public.
pub(crate) struct BumpAlignGuard<'b, 'a, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) scope: &'b mut BumpScope<'a, A, MIN_ALIGN, UP>,
}

impl<'b, 'a, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> Drop for BumpAlignGuard<'b, 'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.scope.force_align::<MIN_ALIGN>();
    }
}

impl<'b, 'a, A: Allocator + Clone, const MIN_ALIGN: usize, const UP: bool> BumpAlignGuard<'b, 'a, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    pub fn new(scope: &'b mut BumpScope<'a, A, MIN_ALIGN, UP>) -> Self {
        Self { scope }
    }
}
