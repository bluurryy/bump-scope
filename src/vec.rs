//! Simple `Vec`-like for the purpose of `alloc_iter`

use core::{
    alloc::Layout,
    ops::{Deref, DerefMut},
    panic::{RefUnwindSafe, UnwindSafe},
};

use allocator_api2::alloc::Allocator;

use crate::{
    BumpBox, BumpScope, ErrorBehavior, FixedBumpVec, MinimumAlignment, SizedTypeProperties, SupportedMinimumAlignment,
};

pub(crate) struct Vec<'b, 'a: 'b, T, A, const MIN_ALIGN: usize = 1, const UP: bool = true> {
    fixed: FixedBumpVec<'a, T>,
    bump: &'b BumpScope<'a, A, MIN_ALIGN, UP>,
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool> UnwindSafe for Vec<'b, 'a, T, A, MIN_ALIGN, UP>
where
    T: UnwindSafe,
    A: UnwindSafe,
{
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool> RefUnwindSafe for Vec<'b, 'a, T, A, MIN_ALIGN, UP>
where
    T: RefUnwindSafe,
    A: RefUnwindSafe,
{
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool> Deref for Vec<'b, 'a, T, A, MIN_ALIGN, UP> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.fixed
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool> DerefMut for Vec<'b, 'a, T, A, MIN_ALIGN, UP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fixed
    }
}

impl<'b, 'a: 'b, T, A, const MIN_ALIGN: usize, const UP: bool> Vec<'b, 'a, T, A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
{
    #[inline(always)]
    pub fn generic_with_capacity_in<B: ErrorBehavior>(
        capacity: usize,
        bump: &'b BumpScope<'a, A, MIN_ALIGN, UP>,
    ) -> Result<Self, B> {
        if T::IS_ZST {
            return Ok(Self {
                fixed: FixedBumpVec {
                    initialized: BumpBox::EMPTY,
                    capacity: usize::MAX,
                },
                bump,
            });
        }

        if capacity == 0 {
            return Ok(Self {
                fixed: FixedBumpVec {
                    initialized: BumpBox::EMPTY,
                    capacity: 0,
                },
                bump,
            });
        }

        Ok(Self {
            fixed: bump.generic_alloc_fixed_vec(capacity)?,
            bump,
        })
    }

    #[inline(always)]
    pub fn generic_push<B: ErrorBehavior>(&mut self, value: T) -> Result<(), B> {
        self.generic_reserve_one()?;

        unsafe {
            self.fixed.unchecked_push(value);
        }

        Ok(())
    }

    pub fn into_slice(self) -> BumpBox<'a, [T]> {
        self.fixed.into_boxed_slice()
    }

    fn generic_reserve_one<B: ErrorBehavior>(&mut self) -> Result<(), B> {
        if self.fixed.is_full() {
            self.generic_grow_cold()
        } else {
            Ok(())
        }
    }

    #[cold]
    #[inline(never)]
    fn generic_grow_cold<B: ErrorBehavior>(&mut self) -> Result<(), B> {
        if T::IS_ZST {
            return Ok(());
        }

        if self.fixed.capacity == 0 {
            self.fixed = self.bump.generic_alloc_fixed_vec::<B, T>(1)?;
            return Ok(());
        }

        let old_ptr = self.fixed.as_non_null_ptr().cast();
        let old_layout = self.fixed.layout();

        let new_capacity = match self.fixed.capacity.checked_mul(2) {
            Some(capacity) => capacity,
            None => return Err(B::capacity_overflow()),
        };

        let new_layout = match Layout::array::<T>(new_capacity) {
            Ok(layout) => layout,
            Err(_) => return Err(B::capacity_overflow()),
        };

        let new_ptr = match unsafe { self.bump.grow(old_ptr, old_layout, new_layout) } {
            Ok(ptr) => ptr,
            Err(_) => return Err(B::allocation(new_layout)),
        };

        unsafe {
            self.fixed.initialized.set_ptr(new_ptr.cast());
        };

        self.fixed.capacity = new_capacity;
        Ok(())
    }
}
