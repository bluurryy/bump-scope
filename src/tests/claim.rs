#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::{
    alloc::{AllocError, Allocator, Global, Layout},
    panic::{AssertUnwindSafe, UnwindSafe},
    ptr::NonNull,
    string::String,
};

use crate::{
    Bump,
    tests::up_and_down,
    traits::{BumpAllocator, Sealed},
};

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn api_fails_on_claimed_bump() {
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

        let bump = Bump::<Global, C>::new();
        let mut boxed = bump.alloc::<[u8; 2]>([1, 2]).into_boxu();
        let ptr = NonNull::new(boxed.as_mut_ptr()).unwrap().cast::<u8>();

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

        // traits::Sealed
        {
            // allocate_layout
            expect_panic(|| bump.spec_alloc::<!>(Layout::new::<()>()));
            expect_panic(|| bump.spec_alloc::<!>(Layout::new::<u8>()));
            expect_err(bump.spec_alloc::<AllocError>(Layout::new::<()>()));
            expect_err(bump.spec_alloc::<AllocError>(Layout::new::<u8>()));

            // allocate sized
            expect_panic(|| bump.spec_alloc_sized::<!, ()>());
            expect_panic(|| bump.spec_alloc_sized::<!, u8>());
            expect_err(bump.spec_alloc_sized::<AllocError, ()>());
            expect_err(bump.spec_alloc_sized::<AllocError, u8>());

            // allocate slice
            expect_panic(|| bump.spec_alloc_slice::<!, ()>(123));
            expect_panic(|| bump.spec_alloc_slice::<!, u8>(0));
            expect_panic(|| bump.spec_alloc_slice::<!, u8>(1));
            expect_err(bump.spec_alloc_slice::<AllocError, ()>(123));
            expect_err(bump.spec_alloc_slice::<AllocError, u8>(0));
            expect_err(bump.spec_alloc_slice::<AllocError, u8>(1));

            // allocate slice for
            expect_panic(|| bump.spec_alloc_slice_for::<!, ()>(&[(), (), ()]));
            expect_panic(|| bump.spec_alloc_slice_for::<!, u8>(&[]));
            expect_panic(|| bump.spec_alloc_slice_for::<!, u8>(&[1]));
            expect_err(bump.spec_alloc_slice_for::<AllocError, ()>(&[(), (), ()]));
            expect_err(bump.spec_alloc_slice_for::<AllocError, u8>(&[]));
            expect_err(bump.spec_alloc_slice_for::<AllocError, u8>(&[1]));

            // shrink slice
            assert!(unsafe { bump.spec_shrink_slice(ptr, 2, 1) }.is_none());

            // prepare slice allocation
            expect_panic(|| bump.spec_prepare_slice_allocation::<!, ()>(123));
            expect_panic(|| bump.spec_prepare_slice_allocation::<!, u8>(123));
            expect_err(bump.spec_prepare_slice_allocation::<AllocError, ()>(123));
            expect_err(bump.spec_prepare_slice_allocation::<AllocError, u8>(123));

            // dealloc
            drop(extra::box_replace_allocator(boxed, &bump));
            assert_eq!(guard.stats().allocated(), original_allocated);

            // reserve
            expect_panic(|| bump.reserve(123));
            expect_err(bump.try_reserve(123));
        }

        // BumpAllocatorScope
        {
            // api is entirely implemented on top of BumpAllocator
            // so no need to test it here
        }

        // Bump(Scope)
        {
            // scope api is not available because it takes
            // `&mut self` and `claim` borrows the allocator
            expect_panic(|| bump.claim());
        }

        drop(guard);
        _ = bump.alloc_str("okey dokey");
        drop(bump);
    }
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
