use crate::{BumpAllocatorScope, Unmanaged, extra::replace_allocator::ReplaceAllocator};

pub trait DetachAllocator<'a, A>: ReplaceAllocator<'a, 'a, A, Unmanaged<'a>> + Sized
where
    A: BumpAllocatorScope<'a>,
{
    fn detach_allocator(self) -> (Self::Output, A) {
        ReplaceAllocator::replace_allocator(self, Unmanaged::VALUE)
    }
}
