use alloc::{boxed::Box, vec::Vec};

use crate::BumpAllocatorScope;

/// Replaces the bump allocator of a box.
pub fn box_replace_allocator<'old, 'new, Old, New, T>(boxed: Box<T, Old>, new: New) -> (Box<T, New>, Old)
where
    T: ?Sized,
    Old: BumpAllocatorScope<'old>,
    New: BumpAllocatorScope<'new>,
    'old: 'new,
{
    let (ptr, old) = Box::into_raw_with_allocator(boxed);
    (unsafe { Box::from_raw_in(ptr, new) }, old)
}

/// Replaces the bump allocator of a vector.
pub fn vec_replace_allocator<'old, 'new, Old, New, T>(vec: Vec<T, Old>, new: New) -> (Vec<T, New>, Old)
where
    Old: BumpAllocatorScope<'old>,
    New: BumpAllocatorScope<'new>,
    'old: 'new,
{
    let (ptr, len, cap, old) = Vec::into_raw_parts_with_allocator(vec);
    (unsafe { Vec::from_raw_parts_in(ptr, len, cap, new) }, old)
}
