#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
//! Some sanity checks for chunk size calculation.
//! Chunk size also has a fuzz target called `chunk_size`.

mod common;

use std::{
    alloc::{AllocError, Allocator, Global, Layout},
    ptr::NonNull,
};

use bump_scope::Bump;

use common::up_and_down;

type AssumedMallocOverhead = [usize; 2];

macro_rules! create_mock_allocator {
    (
        $ident:ident $size_and_align:literal
    ) => {
        #[expect(dead_code)]
        #[repr(align($size_and_align))]
        #[derive(Clone)]
        struct $ident([u8; $size_and_align]);

        impl $ident {
            fn new() -> Self {
                Self([0; $size_and_align])
            }
        }

        unsafe impl Allocator for $ident {
            fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
                Global.allocate(layout)
            }

            unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
                unsafe { Global.deallocate(ptr, layout) };
            }
        }
    };
}
const OVERHEAD: usize = size_of::<AssumedMallocOverhead>();

up_and_down! {
    const C: BumpConfig = DEFAULT_C;
    const C_MIN_SIZE_1: BumpConfig = BumpConfigBuilder(C).min_chunk_size(1).build();
    const C_MIN_SIZE_1024: BumpConfig = BumpConfigBuilder(C).min_chunk_size(1024).build();

    #[test]
    fn zst() {
        // four pointers, + overhead, next power of two, minus overhead
        let bump = Bump::<Global, C_MIN_SIZE_1>::with_size(0);
        assert_eq!(bump.stats().size(), size_of::<[usize; 8]>() - OVERHEAD);

        let bump = Bump::<Global, C_MIN_SIZE_1>::with_size(512 - 1);
        assert_eq!(bump.stats().size(), 512 - OVERHEAD);

        let bump = Bump::<Global, C_MIN_SIZE_1>::with_size(512);
        assert_eq!(bump.stats().size(), 512 - OVERHEAD);

        let bump = Bump::<Global, C_MIN_SIZE_1>::with_size(0x1000 - 1);
        assert_eq!(bump.stats().size(), 0x1000 - OVERHEAD);

        let bump = Bump::<Global, C_MIN_SIZE_1>::with_size(0x1000);
        assert_eq!(bump.stats().size(), 0x1000 - OVERHEAD);

        let bump = Bump::<Global, C_MIN_SIZE_1>::with_size(0x2000 - 1);
        assert_eq!(bump.stats().size(), 0x2000 - OVERHEAD);

        let bump = Bump::<Global, C_MIN_SIZE_1>::with_size(0x2000);
        assert_eq!(bump.stats().size(), 0x2000 - OVERHEAD);

        // same as `with_size(0)`
        let bump = Bump::<Global, C_MIN_SIZE_1>::with_capacity(Layout::array::<u8>(0).unwrap());
        assert_eq!(bump.stats().size(), size_of::<[usize; 8]>() - OVERHEAD);
    }

    #[test]
    fn aligned_allocator_issue_32() {
        create_mock_allocator! {
            BigAllocator 32
        }

        let _: Bump<BigAllocator, C_MIN_SIZE_1> = Bump::with_size_in(0x2000, BigAllocator::new());
    }

    #[test]
    fn giant_base_allocator() {
        create_mock_allocator! {
            MyAllocator 4096
        }

        let bump: Bump<MyAllocator, C_MIN_SIZE_1> = Bump::with_size_in(0x2000, MyAllocator::new());
        assert_eq!(bump.stats().allocated(), 0);
        bump.alloc_str("hey");
        assert_eq!(bump.stats().allocated(), 3);
    }

    #[test]
    fn minimum_size() {
        let bump = Bump::<Global, C_MIN_SIZE_1024>::new();
        assert_eq!(bump.stats().size(), 1024 - size_of::<[usize; 2]>());
    }

    #[test]
    fn minimum_size_smaller_hint() {
        let bump = Bump::<Global, C_MIN_SIZE_1024>::with_size(0);
        assert_eq!(bump.stats().size(), 1024 - size_of::<[usize; 2]>());
    }

    #[test]
    fn minimum_size_bigger_hint() {
        let bump = Bump::<Global, C_MIN_SIZE_1024>::with_size(2048);
        assert_eq!(bump.stats().size(), 2048 - size_of::<[usize; 2]>());
    }
}
