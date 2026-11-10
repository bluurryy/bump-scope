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
up_and_down! {
    const C: BumpConfig = DEFAULT_C;
    const C_SIZE_1: BumpConfig = BumpConfigBuilder(C).default_initial_chunk_size(1).build();
    const C_SIZE_1024: BumpConfig = BumpConfigBuilder(C).default_initial_chunk_size(1024).build();

    #[test]
    fn zst() {
        let bump = Bump::<C_SIZE_1>::with_size(0);
        assert_eq!(bump.stats().size(), size_of::<[usize; 4]>());

        let bump = Bump::<C_SIZE_1>::with_size(512 - 1);
        assert_eq!(bump.stats().size(), 512);

        let bump = Bump::<C_SIZE_1>::with_size(512);
        assert_eq!(bump.stats().size(), 512);

        let bump = Bump::<C_SIZE_1>::with_size(0x1000 - 1);
        assert_eq!(bump.stats().size(), 0x1000);

        let bump = Bump::<C_SIZE_1>::with_size(0x1000);
        assert_eq!(bump.stats().size(), 0x1000);

        let bump = Bump::<C_SIZE_1>::with_size(0x2000 - 1);
        assert_eq!(bump.stats().size(), 0x2000);

        let bump = Bump::<C_SIZE_1>::with_size(0x2000);
        assert_eq!(bump.stats().size(), 0x2000);

        let bump = Bump::<C_SIZE_1>::with_capacity(Layout::array::<u8>(0).unwrap());
        assert_eq!(bump.stats().size(), size_of::<[usize; 4]>());
    }

    #[test]
    fn aligned_allocator_issue_32() {
        create_mock_allocator! {
            BigAllocator 32
        }

        let _: Bump<C_SIZE_1, BigAllocator> = Bump::with_size_in(0x2000, BigAllocator::new());
    }

    #[test]
    fn giant_base_allocator() {
        create_mock_allocator! {
            MyAllocator 4096
        }

        let bump: Bump<C_SIZE_1, MyAllocator> = Bump::with_size_in(0x2000, MyAllocator::new());
        assert_eq!(bump.allocated(), 0);
        bump.alloc_str("hey").into_boxu();
        assert_eq!(bump.allocated(), 3);
    }

    #[test]
    fn minimum_size() {
        let bump = Bump::<C_SIZE_1024>::new();
        assert_eq!(bump.stats().size(), 1024);
    }

    #[test]
    fn minimum_size_smaller_hint() {
        let bump = Bump::<C_SIZE_1024>::with_size(0);
        assert_eq!(bump.stats().size(), size_of::<[usize;4]>());
    }

    #[test]
    fn minimum_size_bigger_hint() {
        let bump = Bump::<C_SIZE_1024>::with_size(2048);
        assert_eq!(bump.stats().size(), 2048);
    }
}
