use core::mem::{self, ManuallyDrop};
use std::sync::Mutex;

use allocator_api2::alloc::Allocator;

#[cfg(feature = "alloc")]
use allocator_api2::alloc::Global;

use crate::{Bump, BumpScope, MinimumAlignment, SupportedMinimumAlignment};

#[derive(Debug)]
pub struct BumpPool<
    #[cfg(feature = "alloc")] A = Global,
    #[cfg(not(feature = "alloc"))] A,
    const MIN_ALIGN: usize = 1,
    const UP: bool = true,
> where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    bumps: Mutex<Vec<Bump<A, MIN_ALIGN, UP>>>,
    allocator: A,
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Default for BumpPool<A, MIN_ALIGN, UP>
where
    A: Allocator + Clone + Default,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn default() -> Self {
        Self {
            bumps: Mutex::default(),
            allocator: Default::default(),
        }
    }
}

#[cfg(feature = "alloc")]
impl<const MIN_ALIGN: usize, const UP: bool> BumpPool<Global, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[must_use]
    pub fn new() -> Self {
        Self::new_in(Global)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> BumpPool<A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[must_use]
    pub fn new_in(allocator: A) -> Self {
        Self {
            bumps: Default::default(),
            allocator,
        }
    }

    pub fn reset(&mut self) {
        for bump in self.bumps.get_mut().unwrap().iter_mut() {
            bump.reset();
        }
    }

    #[must_use]
    pub fn get(&self) -> BumpPoolGuard<A, MIN_ALIGN, UP> {
        let mut bumps = self.bumps.lock().unwrap();
        let bump = bumps.pop().unwrap_or_else(|| Bump::new_in(self.allocator.clone()));

        BumpPoolGuard {
            pool: self,
            bump: ManuallyDrop::new(bump),
        }
    }
}

#[derive(Debug)]
pub struct BumpPoolGuard<'a, A, const MIN_ALIGN: usize, const UP: bool>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    pool: &'a BumpPool<A, MIN_ALIGN, UP>,
    bump: ManuallyDrop<Bump<A, MIN_ALIGN, UP>>,
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Drop for BumpPoolGuard<'a, A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn drop(&mut self) {
        let mut bumps = self.pool.bumps.lock().unwrap();
        let bump = unsafe { ManuallyDrop::take(&mut self.bump) };
        bumps.push(bump);
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> BumpPoolGuard<'a, A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[must_use]
    pub fn scope(&mut self) -> &mut BumpScope<'a, A, MIN_ALIGN, UP> {
        unsafe { mem::transmute(self.bump.as_mut_scope()) }
    }
}
