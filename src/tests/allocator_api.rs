use std::{alloc::Layout, eprintln, ptr::NonNull};

use crate::{
    BaseAllocator,
    alloc::{Allocator, Global},
    polyfill::non_null,
    settings::{BumpAllocatorSettings, BumpSettings},
    tests::Bump,
    traits::{BumpAllocator, BumpAllocatorCore, BumpAllocatorScope, BumpAllocatorTyped},
};

use super::either_way;

either_way! {
  grow

  grow_last_in_place

  grow_last_out_of_place

  allocate_zst_returns_dangling

  allocate_zst_returns_dangling_unallocated
}

fn layout(size: usize) -> Layout {
    Layout::from_size_align(size, 4).unwrap()
}

fn assert_aligned_to(ptr: NonNull<[u8]>) {
    assert!(ptr.cast::<u8>().addr().get() % 4 == 0);
}

fn grow<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    let ptr = bump.allocate(layout(1)).unwrap();
    assert_aligned_to(ptr);

    assert_ne!(ptr, bump.allocate(layout(1)).unwrap());

    let new = bump.allocate(layout(2048)).unwrap();
    assert_aligned_to(new);

    assert_ne!(ptr.cast::<u8>(), new.cast::<u8>());
}

fn grow_last_in_place<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    unsafe {
        let ptr = bump.allocate(layout(1)).unwrap();
        assert_aligned_to(ptr);

        let new = bump.grow(ptr.cast(), layout(1), layout(2)).unwrap();
        assert_aligned_to(new);

        if UP {
            assert_eq!(ptr.cast::<u8>(), new.cast::<u8>());
        }
    }
}

fn grow_last_out_of_place<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    unsafe {
        let ptr = bump.allocate(layout(1)).unwrap();
        assert_aligned_to(ptr);

        let new = bump.grow(ptr.cast(), layout(1), layout(2048)).unwrap();
        assert_aligned_to(new);

        assert_ne!(ptr.cast::<u8>(), new.cast::<u8>());
    }
}

fn allocate_zst_returns_dangling<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
    let dangling_addr = NonNull::<()>::dangling().addr();

    // make sure there is no capacity left on its chunk
    assert_eq!(bump.stats().count(), 1);
    bump.allocate(Layout::array::<u8>(bump.stats().remaining()).unwrap()).unwrap();
    assert_eq!(bump.stats().remaining(), 0);
    assert_eq!(bump.stats().count(), 1);

    macro_rules! must_dangle {
        ($($expr:expr,)*) => {
            $(assert_eq!(dangling_addr, $expr.addr(), stringify!($expr));)*
        };
    }

    macro_rules! mustnt_dangle {
        ($($expr:expr,)*) => {
            $(assert_ne!(dangling_addr, $expr.addr(), stringify!($expr));)*
        };
    }

    // typed allocation functions should return dangling pointers for zsts
    // and not go through the allocation machinery
    must_dangle! {
        bump.alloc::<()>(()).into_raw(),
        bump.try_alloc::<()>(()).unwrap().into_raw(),

        bump.alloc_uninit::<()>().into_raw(),
        bump.try_alloc_uninit::<()>().unwrap().into_raw(),

        bump.allocate_sized::<()>(),
        bump.try_allocate_sized::<()>().unwrap(),

        bump.allocate_slice::<()>(123),
        bump.try_allocate_slice::<()>(123).unwrap(),

        bump.allocate_slice_for::<()>(&[]),
        bump.try_allocate_slice_for::<()>(&[]).unwrap(),

        (&bump as &dyn BumpAllocatorCore).allocate_sized::<()>(),
        (&bump as &dyn BumpAllocatorCore).try_allocate_sized::<()>().unwrap(),

        (&bump as &dyn BumpAllocatorCore).allocate_slice::<()>(123),
        (&bump as &dyn BumpAllocatorCore).try_allocate_slice::<()>(123).unwrap(),

        (&bump as &dyn BumpAllocatorCore).allocate_slice_for::<()>(&[]),
        (&bump as &dyn BumpAllocatorCore).try_allocate_slice_for::<()>(&[]).unwrap(),

    }

    // it'd be fine for those to return dangling pointers, they currently don't though
    mustnt_dangle! {
        bump.allocate_layout(Layout::new::<()>()),
        bump.try_allocate_layout(Layout::new::<()>()).unwrap(),

        bump.allocate(Layout::new::<()>()).unwrap(),
        bump.allocate_zeroed(Layout::new::<()>()).unwrap(),
    }

    // this mustn't have allocated another chunk
    assert_eq!(bump.stats().count(), 1);
}

fn allocate_zst_returns_dangling_unallocated<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP, false>> = Bump::unallocated();
    let dangling_addr = NonNull::<()>::dangling().addr();

    macro_rules! must_dangle {
            ($($expr:expr,)*) => {
                $(assert_eq!(dangling_addr, $expr.addr(), stringify!($expr));)*
            };
        }

    must_dangle! {
        bump.alloc::<()>(()).into_raw(),
        bump.try_alloc::<()>(()).unwrap().into_raw(),

        bump.alloc_uninit::<()>().into_raw(),
        bump.try_alloc_uninit::<()>().unwrap().into_raw(),

        bump.allocate_layout(Layout::new::<()>()),
        bump.try_allocate_layout(Layout::new::<()>()).unwrap(),

        bump.allocate_sized::<()>(),
        bump.try_allocate_sized::<()>().unwrap(),

        bump.allocate_slice::<()>(123),
        bump.try_allocate_slice::<()>(123).unwrap(),

        bump.allocate_slice_for::<()>(&[]),
        bump.try_allocate_slice_for::<()>(&[]).unwrap(),

        (&bump as &dyn BumpAllocatorCore).allocate_sized::<()>(),
        (&bump as &dyn BumpAllocatorCore).try_allocate_sized::<()>().unwrap(),

        (&bump as &dyn BumpAllocatorCore).allocate_slice::<()>(123),
        (&bump as &dyn BumpAllocatorCore).try_allocate_slice::<()>(123).unwrap(),

        (&bump as &dyn BumpAllocatorCore).allocate_slice_for::<()>(&[]),
        (&bump as &dyn BumpAllocatorCore).try_allocate_slice_for::<()>(&[]).unwrap(),

        bump.allocate(Layout::new::<()>()).unwrap(),
        bump.allocate_zeroed(Layout::new::<()>()).unwrap(),
    }

    // this mustn't have allocated a chunk
    assert_eq!(bump.stats().count(), 0);
}
