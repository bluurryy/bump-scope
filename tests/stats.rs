#![cfg(all(feature = "std", feature = "panic-on-alloc",))]

mod common;

use std::{
    alloc::{AllocError, Allocator, Global, Layout},
    ptr::NonNull,
};

use bump_scope::{Bump, BumpAllocator as _, config::BumpConfig};

use common::up_and_down;

use crate::common::AssumedMallocOverhead;

macro_rules! allocator_with_layout {
    ($name:ident, $size:literal, $align:literal) => {
        #[repr(align($align))]
        #[derive(Clone)]
        struct $name {
            #[expect(dead_code)]
            field: [u8; $size],
        }

        impl Default for $name {
            fn default() -> Self {
                Self { field: [0; _] }
            }
        }

        unsafe impl Allocator for $name {
            fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
                Global.allocate(layout)
            }

            unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
                unsafe { Global.deallocate(ptr, layout) };
            }
        }
    };
    ($name:ident: $field:ty) => {
        #[derive(Default, Clone)]
        struct $name {
            #[expect(dead_code)]
            field: $field,
        }

        unsafe impl Allocator for $name {
            fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
                Global.allocate(layout)
            }

            unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
                unsafe { Global.deallocate(ptr, layout) };
            }
        }
    };
}

up_and_down! {
    const C: BumpConfig = BumpConfigBuilder(DEFAULT_C).const_constructible(true).build();

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn unallocated() {
        allocator_with_layout!(TestBaseAllocator, 2048, 8);
        let bump = Bump::<TestBaseAllocator, C>::const_new();
        assert_eq!(bump.stats().count(), 0);
        assert_eq!(bump.stats().size(), 0);
        assert_eq!(bump.stats().capacity(), 0);
        assert_eq!(bump.allocated(), 0);
        assert_eq!(bump.stats().remaining(), 0);
        test_base_allocator(
            Bump::<TestBaseAllocator, C>::new(),
            4096,
            8 * 6 + 2048, // rounded up from 5 to 6 due to align
            8 * 6,        // rounded up from 5 to 6 due to align
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn zst_allocator() {
        allocator_with_layout!(TestBaseAllocator, 0, 1);
        test_base_allocator(
            Bump::<TestBaseAllocator, C>::new(),
            512,
            ZST_ALLOCATED_CHUNK_HEADER_SIZE,
            ZST_ALLOCATED_CHUNK_HEADER_SIZE,
        );
    }

    // asserts same as the `zst_allocator`, alignment shouldn't affect anything
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn zst_allocator_high_align() {
        allocator_with_layout!(TestBaseAllocator, 0, 2048);
        test_base_allocator(
            Bump::<TestBaseAllocator, C>::new(),
            512,
            ZST_ALLOCATED_CHUNK_HEADER_SIZE,
            ZST_ALLOCATED_CHUNK_HEADER_SIZE,
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn ptr_allocator() {
        allocator_with_layout!(TestBaseAllocator, 8, 8);
        test_base_allocator(
            Bump::<TestBaseAllocator, C>::new(),
            512,
            8 * 6,
            8 * 6, // rounded up from 5 to 6 due to align
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn two_ptr_allocator() {
        allocator_with_layout!(TestBaseAllocator, 16, 8);
        test_base_allocator(
            Bump::<TestBaseAllocator, C>::new(),
            512,
            8 * 8, // rounded up from 7 to 8 due to align
            8 * 6, // rounded up from 5 to 6 due to align
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn high_align_allocator() {
        if !bump_scope::config::read::upwards(C) {
            // FIXME: size allocation is weird as heck for downwards, fix it!
            return;
        }

        allocator_with_layout!(TestBaseAllocator, 32, 32);
        test_base_allocator(
            Bump::<TestBaseAllocator, C>::new(),
            512,
            8 * 5 + 8 * 3 + 32, // 8 * 3 is padding
            8 * 6,              // rounded up from 5 to 6 due to align
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn fat_allocator() {
        allocator_with_layout!(TestBaseAllocator, 1024, 1);
        test_base_allocator(
            Bump::<TestBaseAllocator, C>::new(),
            2048,
            8 * 6 + 1024, // rounded up from 5 to 6 due to align
            8 * 6,        // rounded up from 5 to 6 due to align
        );
    }
}

fn allocate_another_chunk<A, const C: BumpConfig>(bump: &Bump<A, C>, dummy_size: usize)
where
    A: Allocator,
{
    let start_chunks = bump.stats().count();

    // fill the existing chunk if there is one
    let remaining = bump.stats().remaining();
    if remaining != 0 {
        bump.allocate(Layout::from_size_align(remaining, 1).unwrap()).unwrap();
    }
    assert_eq!(bump.stats().count(), start_chunks);

    bump.allocate(Layout::from_size_align(dummy_size, 1).unwrap()).unwrap();
    assert_eq!(bump.stats().count(), start_chunks + 1);
    assert_eq!(bump.stats().current_chunk().unwrap().allocated(), dummy_size);
}

const ZST_ALLOCATED_CHUNK_HEADER_SIZE: usize = 8 * 4;
const MALLOC_OVERHEAD_SIZE: usize = size_of::<AssumedMallocOverhead>();

fn test_base_allocator<A, const C: BumpConfig>(
    bump: Bump<A, C>,
    first_chunk_size_with_malloc_overhead: usize,
    first_allocated_chunk_header_size: usize,
    non_first_allocated_chunk_header_size: usize,
) where
    A: Allocator + Default,
{
    bump.alloc(1u8).into_boxu();
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(
        bump.stats().size(),
        first_chunk_size_with_malloc_overhead - MALLOC_OVERHEAD_SIZE
    );
    assert_eq!(
        bump.stats().capacity(),
        first_chunk_size_with_malloc_overhead - MALLOC_OVERHEAD_SIZE - first_allocated_chunk_header_size
    );
    assert_eq!(bump.allocated(), 1);
    assert_eq!(bump.stats().remaining(), bump.stats().capacity() - bump.allocated());

    allocate_another_chunk(&bump, 1);
    assert_eq!(
        bump.stats().current_chunk().unwrap().size(),
        first_chunk_size_with_malloc_overhead * 2 - MALLOC_OVERHEAD_SIZE
    );
    assert_eq!(
        bump.stats().current_chunk().unwrap().capacity(),
        first_chunk_size_with_malloc_overhead * 2 - MALLOC_OVERHEAD_SIZE - non_first_allocated_chunk_header_size
    );
    assert_eq!(bump.stats().current_chunk().unwrap().allocated(), 1);
    assert_eq!(
        bump.stats().current_chunk().unwrap().remaining(),
        bump.stats().current_chunk().unwrap().capacity() - bump.stats().current_chunk().unwrap().allocated()
    );
}
