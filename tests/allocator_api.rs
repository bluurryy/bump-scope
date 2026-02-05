#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::{alloc::Layout, ptr::NonNull};

use bump_scope::{
    Bump,
    alloc::{Allocator, Global},
    settings::BumpSettings,
    traits::{BumpAllocatorCore, BumpAllocatorTyped},
};

macro_rules! either_way {
    ($($(#[$attr:meta])* $ident:ident)*) => {
        $(
            mod $ident {
                #[test]
                $(#[$attr])*
                fn up() {
                    std::eprintln!("`UP` is `true`");
                    super::$ident::<true>();
                }

                #[test]
                $(#[$attr])*
                fn down() {
                    std::eprintln!("`UP` is `false`");
                    super::$ident::<false>();
                }
            }
        )*
    };
}

either_way! {
  allocate

  allocate_zst_returns_dangling

  allocate_zst_returns_dangling_unallocated

  grow
}

macro_rules! assert_eq_ne {
    ($which:expr, $lhs:expr, $rhs:expr) => {
        if $which {
            assert_eq!($lhs, $rhs);
        } else {
            assert_ne!($lhs, $rhs);
        }
    };
}

fn allocate<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    let layout = Layout::from_size_align(1, 4).unwrap();
    let ptr = bump.allocate(layout).unwrap();
    assert_eq!(ptr.cast::<u8>().addr().get() % 4, 0);

    assert_ne!(ptr, bump.allocate(Layout::from_size_align(1, 1).unwrap()).unwrap());

    let layout = Layout::from_size_align(2048, 4).unwrap();
    let new = bump.allocate(layout).unwrap();
    assert_eq!(ptr.cast::<u8>().addr().get() % 4, 0);

    assert_ne!(ptr.cast::<u8>(), new.cast::<u8>());
}

fn allocate_zst_returns_dangling<const UP: bool>() {
    let dangling_addr = NonNull::<()>::dangling().addr();

    // create a bump allocator with a chunk with no capacity
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();
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

fn allocate_zst_returns_dangling_unallocated<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP, false>> = Bump::unallocated();
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

fn grow<const UP: bool>() {
    let bump: Bump<Global, BumpSettings<1, UP>> = Bump::new();

    unsafe {
        let new_layout = Layout::from_size_align(1, 4).unwrap();
        let new_ptr = bump.allocate(new_layout).unwrap();
        assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
        assert_eq!(bump.stats().allocated(), if UP { 1 } else { 4 });
        let old_layout = new_layout;
        let old_ptr = new_ptr;

        // grow in place with same align (same size)
        let new_layout = Layout::from_size_align(1, 4).unwrap();
        let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
        assert_eq!(bump.stats().allocated(), if UP { 1 } else { 4 });
        assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
        assert_eq!(old_ptr.addr(), new_ptr.addr());
        let old_layout = new_layout;
        let old_ptr = new_ptr;

        // grow in place with same align
        let new_layout = Layout::from_size_align(2, 4).unwrap();
        let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
        assert_eq!(bump.stats().allocated(), if UP { 2 } else { 8 });
        assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
        assert_eq_ne!(UP, old_ptr.addr(), new_ptr.addr());
        let old_layout = new_layout;
        let old_ptr = new_ptr;

        // grow in place with greater align (same size)
        let new_layout = Layout::from_size_align(2, 8).unwrap();
        let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
        assert_eq!(bump.stats().allocated(), if UP { 2 } else { 8 });
        assert_eq!(new_ptr.cast::<u8>().addr().get() % 8, 0);
        assert_eq!(old_ptr.addr(), new_ptr.addr());
        let old_layout = new_layout;
        let old_ptr = new_ptr;

        // grow in place with greater align
        let new_layout = Layout::from_size_align(3, 16).unwrap();
        let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
        assert_eq!(bump.stats().allocated(), if UP { 3 } else { 16 });
        assert_eq!(new_ptr.cast::<u8>().addr().get() % 8, 0);
        assert_eq_ne!(UP, old_ptr.addr(), new_ptr.addr());
        let old_layout = new_layout;
        let old_ptr = new_ptr;

        // grow in place with smaller align (same size)
        let new_layout = Layout::from_size_align(3, 8).unwrap();
        let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
        assert_eq!(bump.stats().allocated(), if UP { 3 } else { 16 });
        assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
        assert_eq!(old_ptr.addr(), new_ptr.addr());
        let old_layout = new_layout;
        let old_ptr = new_ptr;

        // grow in place with smaller align
        let new_layout = Layout::from_size_align(4, 4).unwrap();
        let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
        assert_eq!(bump.stats().allocated(), if UP { 4 } else { 20 });
        assert_eq!(new_ptr.cast::<u8>().addr().get() % 4, 0);
        assert_eq_ne!(UP, old_ptr.addr(), new_ptr.addr());
        let old_layout = new_layout;
        let old_ptr = new_ptr;

        // allocate so the next grow will be out of place
        bump.allocate(Layout::new::<u8>()).unwrap();
        assert_eq!(bump.stats().allocated(), if UP { 5 } else { 21 });

        // grow out of place with same align
        let new_layout = Layout::from_size_align(4, 4).unwrap();
        let new_ptr = bump.grow(old_ptr.cast(), old_layout, new_layout).unwrap();
        assert_eq!(bump.stats().allocated(), if UP { 12 } else { 28 });
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
