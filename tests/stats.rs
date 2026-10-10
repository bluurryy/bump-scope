#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{
    alloc::{AllocError, Allocator, Global, Layout},
    ptr::NonNull,
};

use bump_scope::{settings::BumpSettings, traits::BumpAllocatorCore};

use common::either_way;

use crate::common::{AssumedMallocOverhead, ChunkHeader};

either_way! {
    zst_allocator
    ptr_allocator
    fat_allocator
}

type Bump<const UP: bool, A = Global> = bump_scope::Bump<A, BumpSettings<1, UP, false, true>>;

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

fn zst_allocator<const UP: bool>() {
    allocator_with_layout!(ZstAllocator, 0, 1);
    let bump = <Bump<UP, ZstAllocator>>::new();
    bump.alloc(1u8);
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(bump.stats().size(), 512 - size_of::<AssumedMallocOverhead>());
    assert_eq!(bump.stats().allocated(), 1);
    assert_eq!(
        bump.stats().capacity(),
        512 - size_of::<AssumedMallocOverhead>() - size_of::<ChunkHeader<ZstAllocator>>()
    );

    assert_eq!(bump.stats().size(), bump.any_stats().size());
    assert_eq!(bump.stats().allocated(), bump.any_stats().allocated());
    assert_eq!(bump.stats().capacity(), bump.any_stats().capacity());
    drop(bump);
}

fn ptr_allocator<const UP: bool>() {
    allocator_with_layout!(PtrAllocator: *const u8);
    let bump = <Bump<UP, PtrAllocator>>::new();
    bump.alloc(1u8);
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(bump.stats().size(), 512 - size_of::<AssumedMallocOverhead>());
    assert_eq!(bump.stats().allocated(), 1);
    assert_eq!(
        bump.stats().capacity(),
        512 - size_of::<AssumedMallocOverhead>() - size_of::<ChunkHeader<PtrAllocator>>()
    );

    assert_eq!(bump.stats().size(), bump.any_stats().size());
    assert_eq!(bump.stats().allocated(), bump.any_stats().allocated());
    assert_eq!(bump.stats().capacity(), bump.any_stats().capacity());
    drop(bump);
}

fn fat_allocator<const UP: bool>() {
    allocator_with_layout!(FatAllocator, 1024, 1);
    let bump = <Bump<UP, FatAllocator>>::new();
    bump.alloc(1u8);
    assert_eq!(bump.stats().count(), 1);
    assert_eq!(bump.stats().size(), 2048 - size_of::<AssumedMallocOverhead>());
    assert_eq!(bump.stats().allocated(), 1);
    assert_eq!(
        bump.stats().capacity(),
        2048 - size_of::<AssumedMallocOverhead>() - size_of::<ChunkHeader<FatAllocator>>()
    );

    assert_eq!(bump.stats().size(), bump.any_stats().size());
    assert_eq!(bump.stats().allocated(), bump.any_stats().allocated());
    assert_eq!(bump.stats().capacity(), bump.any_stats().capacity());
    drop(bump);
}
