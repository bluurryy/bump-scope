use core::{
    alloc::Allocator,
    cell::Cell,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit, offset_of},
    ops::Deref,
    ptr::NonNull,
};

use crate::{
    SizedTypeProperties,
    chunk::{AllocatedChunk, AllocatedChunkHeader, ChunkSize, NonZstFirstAllocatedChunkHeader},
    error_behavior::ErrorBehavior,
    polyfill::transmute_ref,
    settings::BumpAllocatorSettings,
};

/// The first [`AllocatedChunk`], it stores the allocator and `prev` is always `None`.
#[repr(transparent)]
pub(crate) struct FirstAllocatedChunk<A, S> {
    header: NonNull<AllocatedChunkHeader>,
    marker: PhantomData<fn() -> (A, S)>,
}

impl<A, S> Copy for FirstAllocatedChunk<A, S> {}

impl<A, S> Clone for FirstAllocatedChunk<A, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S> FirstAllocatedChunk<A, S>
where
    S: BumpAllocatorSettings,
{
    pub(crate) unsafe fn from_unchecked(chunk: AllocatedChunk<A, S>) -> Self {
        debug_assert!(chunk.prev().is_none());
        Self {
            header: chunk.header().cast(),
            marker: PhantomData,
        }
    }

    pub(crate) fn new<E>(chunk_size: impl ChunkSize, allocator: A) -> Result<Self, E>
    where
        A: Allocator,
        E: ErrorBehavior,
    {
        let (ptr, size) = AllocatedChunk::<A, S>::allocate(chunk_size, &allocator)?;
        unsafe { Ok(FirstAllocatedChunk::init(ptr, size, allocator)) }
    }

    pub(crate) unsafe fn init(ptr: NonNull<u8>, size: usize, allocator: A) -> FirstAllocatedChunk<A, S> {
        unsafe {
            if A::IS_ZST {
                let header = if S::UP {
                    ptr.cast::<AllocatedChunkHeader>()
                } else {
                    ptr.add(size).cast::<AllocatedChunkHeader>().sub(1)
                };

                let pos = Cell::new(if S::UP { header.add(1).cast() } else { header.cast() });
                let end = if S::UP { ptr.add(size) } else { ptr };

                header.write(AllocatedChunkHeader {
                    pos,
                    end,
                    prev: Cell::new(None),
                    next: Cell::new(None),
                });

                // We logically "store" the allocator in the chunk, which for a zst
                // just means that we don't drop it.
                _ = ManuallyDrop::new(allocator);

                FirstAllocatedChunk {
                    header,
                    marker: PhantomData,
                }
            } else {
                let header = if S::UP {
                    ptr.cast::<NonZstFirstAllocatedChunkHeader<A>>()
                } else {
                    ptr.add(size).cast::<NonZstFirstAllocatedChunkHeader<A>>().sub(1)
                };

                let allocator_ptr = header
                    .byte_add(offset_of!(NonZstFirstAllocatedChunkHeader<A>, allocator))
                    .cast();
                let pos = Cell::new(if S::UP { header.add(1).cast() } else { header.cast() });
                let end = if S::UP { ptr.add(size) } else { ptr };

                header
                    .cast::<NonZstFirstAllocatedChunkHeader<A>>()
                    .write(NonZstFirstAllocatedChunkHeader {
                        pos,
                        end,
                        prev: Cell::new(None),
                        next: Cell::new(None),
                        allocator_ptr,
                        allocator,
                    });

                FirstAllocatedChunk {
                    header: header.cast(),
                    marker: PhantomData,
                }
            }
        }
    }

    pub(crate) fn read_allocator(self) -> A {
        if A::IS_ZST {
            // SAFETY: a zst does not have an invalid value; we logically "stored"
            // an instance of `A` in this chunk which we now "read"
            #[allow(clippy::uninit_assumed_init)]
            unsafe {
                MaybeUninit::<A>::uninit().assume_init()
            }
        } else {
            unsafe {
                self.header
                    .byte_add(offset_of!(NonZstFirstAllocatedChunkHeader<A>, allocator))
                    .cast::<A>()
                    .read()
            }
        }
    }
}

impl<A, S> Deref for FirstAllocatedChunk<A, S> {
    type Target = AllocatedChunk<A, S>;

    fn deref(&self) -> &Self::Target {
        unsafe { transmute_ref(self) }
    }
}
