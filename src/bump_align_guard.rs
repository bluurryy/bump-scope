use allocator_api2::alloc::Allocator;

use crate::{BumpScope, MinimumAlignment, SupportedMinimumAlignment};

/// Aligns the bump pointer on drop.
///
/// This can't be a safe public api. Bump validity relies on drop being called. We can not enforce drop being called if this were public.
pub(crate) struct BumpAlignGuard<'b, 'a, const MIN_ALIGN: usize, const UP: bool, A: Allocator + Clone>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pub(crate) scope: &'b mut BumpScope<'a, MIN_ALIGN, UP, A>,
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, A: Allocator + Clone> Drop for BumpAlignGuard<'b, 'a, MIN_ALIGN, UP, A>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn drop(&mut self) {
        self.scope.force_align::<MIN_ALIGN>();
    }
}

impl<'b, 'a, const MIN_ALIGN: usize, const UP: bool, A: Allocator + Clone> BumpAlignGuard<'b, 'a, MIN_ALIGN, UP, A>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    pub fn new(scope: &'b mut BumpScope<'a, MIN_ALIGN, UP, A>) -> Self {
        Self { scope }
    }
}
