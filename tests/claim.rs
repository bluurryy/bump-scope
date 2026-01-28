#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::{
    alloc::Layout,
    panic,
    panic::{AssertUnwindSafe, UnwindSafe},
    ptr::NonNull,
    string::String,
};

use bump_scope::{
    BumpBox, BumpVec,
    alloc::{AllocError, Allocator, Global},
    bump_vec,
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
    api_fails_on_claimed_bump

    scoped

    scope_guard

    alloc_mut

    double_drop
}

type Bump<const UP: bool, A = Global> = bump_scope::Bump<A, BumpSettings<1, UP>>;

fn api_fails_on_claimed_bump<const UP: bool>() {
    trait Whatever {}
    impl<T: ?Sized> Whatever for T {}

    fn expect_err(result: Result<impl Whatever, AllocError>) {
        assert!(result.is_err());
    }

    fn expect_panic<R>(f: impl FnMut() -> R) {
        match catch(AssertUnwindSafe(f)) {
            Ok(_) => panic!("expected panic"),
            Err(err) => {
                if err != "bump allocator is claimed" && err != "bump allocator is already claimed" {
                    panic!("wrong panic message: {err}");
                }
            }
        }
    }

    let bump = <Bump<UP>>::new();
    let boxed = bump.alloc::<[u8; 2]>([1, 2]);
    let ptr = BumpBox::as_raw(&boxed).cast::<u8>();

    let original_allocated = 2;
    assert_eq!(bump.stats().allocated(), original_allocated);

    let guard = bump.claim();

    // Allocator
    {
        // allocate
        expect_err(bump.allocate(Layout::new::<()>()));
        expect_err(bump.allocate(Layout::new::<u8>()));

        // allocate_zeroed
        expect_err(bump.allocate_zeroed(Layout::new::<()>()));
        expect_err(bump.allocate_zeroed(Layout::new::<u8>()));

        // grow
        expect_err(unsafe { bump.grow(ptr, Layout::new::<[u8; 2]>(), Layout::new::<[u8; 3]>()) });

        // grow_zeroed
        expect_err(unsafe { bump.grow_zeroed(ptr, Layout::new::<[u8; 2]>(), Layout::new::<[u8; 3]>()) });

        // shrink
        // doesn't error, but will just return the old ptr and len
        let new_ptr = unsafe { bump.shrink(ptr, Layout::new::<[u8; 2]>(), Layout::new::<[u8; 1]>()).unwrap() };
        assert_eq!(new_ptr.len(), 2);
        assert_eq!(ptr, new_ptr.cast());
        assert_eq!(guard.stats().allocated(), original_allocated);

        // deallocate
        // doesn't do anything
        unsafe { bump.deallocate(ptr, Layout::new::<[u8; 2]>()) };
        assert_eq!(guard.stats().allocated(), original_allocated);
    }

    // BumpAllocatorCore
    {
        // prepare_allocation
        expect_err(bump.prepare_allocation(Layout::new::<()>()));
    }

    // BumpAllocatorCoreScope
    {
        // doesn't add any new api
    }

    // BumpAllocatorTyped
    {
        // allocate_layout
        expect_panic(|| bump.allocate_layout(Layout::new::<()>()));
        expect_err(bump.try_allocate_layout(Layout::new::<()>()));
        expect_panic(|| bump.allocate_layout(Layout::new::<u8>()));
        expect_err(bump.try_allocate_layout(Layout::new::<u8>()));

        // allocate_sized
        expect_panic(|| bump.allocate_sized::<()>());
        expect_err(bump.try_allocate_sized::<()>());
        expect_panic(|| bump.allocate_sized::<u8>());
        expect_err(bump.try_allocate_sized::<u8>());

        // allocate_slice
        expect_panic(|| bump.allocate_slice::<()>(123));
        expect_err(bump.try_allocate_slice::<()>(123));
        expect_panic(|| bump.allocate_slice::<u8>(0));
        expect_err(bump.try_allocate_slice::<u8>(0));
        expect_panic(|| bump.allocate_slice::<u8>(1));
        expect_err(bump.try_allocate_slice::<u8>(1));

        // allocate_slice_for
        expect_panic(|| bump.allocate_slice_for::<()>(&[(), (), ()]));
        expect_err(bump.try_allocate_slice_for::<()>(&[(), (), ()]));
        expect_panic(|| bump.allocate_slice_for::<u8>(&[]));
        expect_err(bump.try_allocate_slice_for::<u8>(&[]));
        expect_panic(|| bump.allocate_slice_for::<u8>(&[1]));
        expect_err(bump.try_allocate_slice_for::<u8>(&[1]));

        // shrink_slice
        assert!(unsafe { bump.shrink_slice(ptr, 2, 1) }.is_none());

        // prepare_slice_allocation
        expect_panic(|| bump.prepare_slice_allocation::<()>(123));
        expect_err(bump.try_prepare_slice_allocation::<()>(123));
        expect_panic(|| bump.prepare_slice_allocation::<u8>(123));
        expect_err(bump.try_prepare_slice_allocation::<u8>(123));

        // dealloc
        bump.dealloc(boxed);
        assert_eq!(guard.stats().allocated(), original_allocated);

        // reserve_bytes
        expect_panic(|| bump.reserve_bytes(123));
        expect_err(bump.try_reserve_bytes(123));
    }

    // BumpAllocatorTypedScope
    {
        // api is entirely implemented on top of BumpAllocatorTyped
        // so no need to test it here
    }

    // BumpAllocator
    {
        // scope api is not available because it takes
        // `&mut self` and `claim` borrows the allocator
    }

    // BumpAllocatorScope
    {
        expect_panic(|| bump.claim());
    }

    drop(guard);
    _ = bump.alloc_str("okey dokey");
    drop(bump);
}

fn catch<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> Result<R, String> {
    match std::panic::catch_unwind(f) {
        Ok(r) => Ok(r),
        Err(err) => {
            if let Some(&err) = err.downcast_ref::<&str>() {
                return Err(err.into());
            }

            if let Some(err) = err.downcast_ref::<String>() {
                return Err(err.into());
            }

            Err("panicked".into())
        }
    }
}

fn scoped<const UP: bool>() {
    let bump = <Bump<UP>>::new();

    let vec1: BumpVec<u8, _> = bump_vec![in &bump; 1, 2, 3];
    let vec2: BumpVec<u8, _> = bump_vec![in &bump; 4, 5, 6];

    assert_eq!(bump.stats().allocated(), 6);

    bump.claim().scoped(|bump| {
        let mut both = BumpVec::new_in(&bump);
        both.extend(vec1.iter().copied());
        both.extend(vec2.iter().copied());
        assert_eq!(both, [1, 2, 3, 4, 5, 6]);
        assert_eq!(bump.stats().allocated(), 6 + both.capacity());
    });

    assert_eq!(bump.stats().allocated(), 6);
}

fn scope_guard<const UP: bool>() {
    let bump = <Bump<UP>>::new();

    let vec1: BumpVec<u8, _> = bump_vec![in &bump; 1, 2, 3];
    let vec2: BumpVec<u8, _> = bump_vec![in &bump; 4, 5, 6];

    assert_eq!(bump.stats().allocated(), 6);

    {
        let mut guard = bump.claim();
        let mut guard = guard.scope_guard();
        let bump = guard.scope();

        let mut both = BumpVec::new_in(&bump);
        both.extend(vec1.iter().copied());
        both.extend(vec2.iter().copied());
        assert_eq!(both, [1, 2, 3, 4, 5, 6]);
        assert_eq!(bump.stats().allocated(), 6 + both.capacity());
    }

    assert_eq!(bump.stats().allocated(), 6);
}

fn alloc_mut<const UP: bool>() {
    let bump = <Bump<UP>>::new();

    let uno_dos_tres = bump.claim().alloc_iter_mut([1, 2, 3]);
    assert_eq!(uno_dos_tres, [1, 2, 3]);
}

fn double_drop<const UP: bool>() {
    struct CountDrops<'a>(&'a mut usize);

    impl Drop for CountDrops<'_> {
        fn drop(&mut self) {
            *self.0 += 1;
        }
    }

    unsafe impl Allocator for CountDrops<'_> {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            Global.allocate(layout)
        }

        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            unsafe { Global.deallocate(ptr, layout) };
        }
    }

    fn check<const UP: bool>(f: fn(CountDrops) -> Bump<UP, CountDrops>) {
        let mut drop_count = 0;
        let bump = f(CountDrops(&mut drop_count));

        bump.claim();
        bump.claim();
        bump.claim();

        assert_eq!(*bump.allocator().0, 0);

        drop(bump);

        assert_eq!(drop_count, 1);
    }

    check::<UP>(|a| Bump::new_in(a));
    check::<UP>(|a| Bump::with_size_in(512, a));
}
