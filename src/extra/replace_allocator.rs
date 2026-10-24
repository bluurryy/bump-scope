use alloc::{boxed::Box, vec::Vec};

use crate::BumpAllocatorScope;

/// TODO
pub fn replace_allocator<'old, 'new, Old, New, BoxOrVec>(box_or_vec: BoxOrVec, new: New) -> (BoxOrVec::Output, Old)
where
    BoxOrVec: ReplaceAllocator<'old, 'new, Old, New>,
    Old: BumpAllocatorScope<'old>,
    New: BumpAllocatorScope<'new>,
    'old: 'new,
{
    ReplaceAllocator::replace_allocator(box_or_vec, new)
}

pub trait ReplaceAllocator<'old, 'new, Old, New>
where
    Old: BumpAllocatorScope<'old>,
    New: BumpAllocatorScope<'new>,
    'old: 'new,
{
    type Output;

    fn replace_allocator(self, new: New) -> (Self::Output, Old);
}

impl<'old, 'new, Old, New, T> ReplaceAllocator<'old, 'new, Old, New> for Box<T, Old>
where
    Old: BumpAllocatorScope<'old>,
    New: BumpAllocatorScope<'new>,
    'old: 'new,
    T: ?Sized,
{
    type Output = Box<T, New>;

    fn replace_allocator(self, new: New) -> (Self::Output, Old) {
        let (ptr, old) = Box::into_raw_with_allocator(self);
        (unsafe { Box::from_raw_in(ptr, new) }, old)
    }
}

impl<'old, 'new, Old, New, T> ReplaceAllocator<'old, 'new, Old, New> for Vec<T, Old>
where
    Old: BumpAllocatorScope<'old>,
    New: BumpAllocatorScope<'new>,
    'old: 'new,
{
    type Output = Vec<T, New>;

    fn replace_allocator(self, new: New) -> (Self::Output, Old) {
        let (ptr, len, cap, old) = Vec::into_raw_parts_with_allocator(self);
        (unsafe { Vec::from_raw_parts_in(ptr, len, cap, new) }, old)
    }
}
