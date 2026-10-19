#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{
    alloc::{Allocator, Global, Layout},
    ptr::NonNull,
};

use bump_scope::{
    Bump, config,
    traits::{BumpAllocator, BumpAllocatorCore},
};

use crate::common::{DisaligningAllocator, up_and_down};

macro_rules! assert_eq_ne {
    ($which:expr, $lhs:expr, $rhs:expr) => {
        if $which {
            assert_eq!($lhs, $rhs);
        } else {
            assert_ne!($lhs, $rhs);
        }
    };
}

up_and_down! {
    const C: BumpConfig = BumpConfigBuilder(DEFAULT_C).const_constructible(true).build();

    #[test]
    fn allocate() {
        let bump: Bump<Global, C> = Bump::new();

        let layout = Layout::from_size_align(1, 4).unwrap();
        let ptr = bump.allocate(layout).unwrap();
        assert_eq!(ptr.cast::<u8>().addr().get() % 4, 0);

        assert_ne!(ptr, bump.allocate(Layout::from_size_align(1, 1).unwrap()).unwrap());

        let layout = Layout::from_size_align(2048, 4).unwrap();
        let new = bump.allocate(layout).unwrap();
        assert_eq!(ptr.cast::<u8>().addr().get() % 4, 0);

        assert_ne!(ptr.cast::<u8>(), new.cast::<u8>());
    }

    #[test]
    fn allocate_zst_returns_dangling() {
        let dangling_addr = NonNull::<()>::dangling().addr();

        // create a bump allocator with a chunk with no capacity
        let bump: Bump<Global, C> = Bump::new();
        assert_eq!(bump.stats().count(), 1);
        bump.allocate(Layout::array::<u8>(bump.stats().remaining()).unwrap()).unwrap();
        assert_eq!(bump.stats().remaining(), 0);
        assert_eq!(bump.stats().count(), 1);
        let start_allocated = bump.stats().allocated();

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

        // the alloc api must return dangling pointers
        must_dangle! {
            bump.alloc::<()>(()).into_raw(),
            bump.try_alloc::<()>(()).unwrap().into_raw(),

            bump.alloc_uninit::<()>().into_raw(),
            bump.try_alloc_uninit::<()>().unwrap().into_raw(),
        }

        // the allocate api must not return dangling pointers
        mustnt_dangle! {
            bump.allocate(Layout::new::<()>()).unwrap(),
            bump.allocate_zeroed(Layout::new::<()>()).unwrap(),

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
        }

        // this mustn't have allocated another chunk
        assert_eq!(bump.stats().count(), 1);
        assert_eq!(bump.stats().allocated(), start_allocated);
    }

    #[test]
    fn allocate_zst_returns_dangling_unallocated() {
        let bump: Bump<Global, C> = Bump::const_new();
        let dangling_addr = NonNull::<()>::dangling().addr();

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

        // the alloc api must return dangling pointers
        must_dangle! {
            bump.alloc::<()>(()).into_raw(),
            bump.try_alloc::<()>(()).unwrap().into_raw(),

            bump.alloc_uninit::<()>().into_raw(),
            bump.try_alloc_uninit::<()>().unwrap().into_raw(),
        }

        // this mustn't have allocated a chunk
        assert_eq!(bump.stats().count(), 0);

        // the allocate api must not return dangling pointers
        mustnt_dangle! {
            bump.allocate(Layout::new::<()>()).unwrap(),
            bump.allocate_zeroed(Layout::new::<()>()).unwrap(),

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
        }

        // this must have allocate a chunk (but takes up no capacity)
        assert_eq!(bump.stats().count(), 1);
        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn grow() {
        let bump: Bump<Global, C> = Bump::new();

        unsafe {
            let new_layout = Layout::from_size_align(1, 4).unwrap();
            let new_ptr = bump.allocate(new_layout).unwrap();
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 1 } else { 4 });
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // grow in place with same align (same size)
            let new_layout = Layout::from_size_align(1, 4).unwrap();
            let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 1 } else { 4 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_eq!(old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // grow in place with same align
            let new_layout = Layout::from_size_align(2, 4).unwrap();
            let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 2 } else { 8 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_eq_ne!(config::read::upwards(C), old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // grow in place with greater align (same size)
            let new_layout = Layout::from_size_align(2, 8).unwrap();
            let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 2 } else { 8 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 8, 0);
            assert_eq!(old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // grow in place with greater align
            let new_layout = Layout::from_size_align(3, 16).unwrap();
            let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 3 } else { 16 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 8, 0);
            assert_eq_ne!(config::read::upwards(C), old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // grow in place with smaller align (same size)
            let new_layout = Layout::from_size_align(3, 8).unwrap();
            let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 3 } else { 16 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_eq!(old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // grow in place with smaller align
            let new_layout = Layout::from_size_align(4, 4).unwrap();
            let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 4 } else { 20 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_eq_ne!(config::read::upwards(C), old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // allocate so the next grow will be out of place
            bump.allocate(Layout::new::<u8>()).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 5 } else { 21 });

            // grow out of place with same align
            let new_layout = Layout::from_size_align(4, 4).unwrap();
            let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 12 } else { 28 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_ne!(old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            assert_eq!(bump.stats().count(), 1);

            // grow in new chunk
            let new_layout = Layout::from_size_align(1024, 4).unwrap();
            let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(
                bump.stats().allocated(),
                bump.stats().small_to_big().next().unwrap().capacity() + 1024
            );
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_ne!(old_ptr.addr(), new_ptr.addr());

            assert_eq!(bump.stats().count(), 2);
        }
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn shrink() {
        let base = DisaligningAllocator::new(4096, Global);
        let bump: Bump<_, C> = Bump::new_in(&base);

        unsafe {
            let new_layout = Layout::from_size_align(15, 4).unwrap();
            let new_ptr = bump.allocate(new_layout).unwrap();
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 15 } else { 16 });
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // shrink in place with same align (same size)
            let new_layout = Layout::from_size_align(15, 4).unwrap();
            let new_ptr = bump.shrink(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 15 } else { 16 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_eq!(old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // shrink in place with same align
            let new_layout = Layout::from_size_align(11, 4).unwrap();
            let new_ptr = bump.shrink(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 11 } else { 12 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
            assert_eq_ne!(config::read::upwards(C), old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // shrink in place with smaller align (same size)
            let new_layout = Layout::from_size_align(11, 2).unwrap();
            let new_ptr = bump.shrink(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 11 } else { 12 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 2, 0);
            assert_eq!(old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // shrink in place with smaller align
            let new_layout = Layout::from_size_align(10, 1).unwrap();
            let new_ptr = bump.shrink(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 10 } else { 11 });
            assert_eq_ne!(config::read::upwards(C), old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // shrink in place with greater align (same size)
            let new_layout = Layout::from_size_align(10, 8).unwrap();
            let new_ptr = bump.shrink(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 10 } else { 16 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 8, 0);
            assert_eq_ne!(config::read::upwards(C), old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            // shrink in place with greater align
            let new_layout = Layout::from_size_align(9, 16).unwrap();
            let new_ptr = bump.shrink(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().allocated(), if config::read::upwards(C) { 9 } else { 16 });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 16, 0);
            assert_eq!(old_ptr.addr(), new_ptr.addr());
            let old_layout = new_layout;
            let old_ptr = new_ptr;

            assert_eq!(bump.stats().count(), 1);

            // shrink out of place (same size)
            let new_layout = Layout::from_size_align(9, 1024).unwrap();
            let new_ptr = bump.shrink(old_ptr.cast(), old_layout, new_layout).unwrap();
            assert_eq!(bump.stats().count(), 2);
            dbg!(&bump);
            dbg!(&bump.stats().small_to_big().collect::<Vec<_>>());
            assert_eq!(bump.stats().allocated(), {
                let disalignment_offset = 16;
                let header_size = 8 * 6;
                let chunks = bump.stats().small_to_big().collect::<Vec<_>>();

                chunks[0].capacity()
                    + if config::read::upwards(C) {
                        let padding_until_aligned = 1024 - disalignment_offset - header_size;
                        padding_until_aligned + 9
                    } else {
                        chunks[1].capacity() - (1024 - disalignment_offset)
                    }
            });
            assert_eq!(new_ptr.cast::<u8>().addr().get() % 1024, 0);
            assert_ne!(old_ptr.addr(), new_ptr.addr());
        }
    }
}
