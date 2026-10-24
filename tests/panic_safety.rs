#![cfg(all(feature = "std", feature = "panic-on-alloc", feature = "nightly"))]
#![feature(allocator_ext)]

mod common;

use std::{
    alloc::{AllocError, Allocator, Global, Layout},
    cell::Cell,
    mem::ManuallyDrop,
    panic::{RefUnwindSafe, catch_unwind},
    ptr::NonNull,
};

use bump_scope::{Bump, extra, vec};

macro_rules! zst_or_not {
    (
        $(
            $name:ident
        )*
    ) => {
        $(
            mod $name {
                #[test]
                #[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
                fn non_zst() {
                    super::$name::<super::helper::Wrap<i32>>();
                }

                #[test]
                #[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
                fn zst() {
                    super::$name::<super::helper::Wrap<()>>();
                }
            }
        )*
    };
}

zst_or_not! {
    init_clone

    init_fill

    init_fill_with

    vec_from_elem_in
}

fn init_clone<T: Testable>() {
    expected_drops(3).panic_on_clone(3).run(|| {
        let original = ManuallyDrop::new(T::array::<5>());
        let bump: Bump = Bump::new();
        let uninit = bump.alloc_uninit_slice::<T>(5).into_boxu();
        let _init = extra::box_init_clone(uninit, &*original);
    });
}

fn init_fill<T: Testable>() {
    expected_drops(4).panic_on_clone(3).run(|| {
        let bump: Bump = Bump::new();
        let uninit = bump.alloc_uninit_slice::<T>(5).into_boxu();
        let _init = extra::box_init_fill(uninit, T::default());
    });
}

fn init_fill_with<T: Testable>() {
    expected_drops(3).panic_on_clone(3).run(|| {
        let original = ManuallyDrop::new(T::default());
        let bump: Bump = Bump::new();
        let uninit = bump.alloc_uninit_slice::<T>(5).into_boxu();
        let _init = extra::box_init_fill_with(uninit, || (*original).clone());
    });
}

fn vec_from_elem_in<T: Testable>() {
    expected_drops(4).panic_on_clone(3).run(|| {
        let bump: Bump = Bump::new();
        let _vec = vec![in &bump; T::default(); 5];
    });
}

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn test_shrink_unfit_in_another_chunk() {
        // Here we make sure to create a first chunk that is only aligned to the required chunk
        // alignment and no more, so when a shrink is done with a high alignment it can't
        // fit in the first chunk.

        #[derive(Default)]
        struct A {
            allocator: DisaligningAllocator<Global>,
            allocation_count: Cell<usize>,
        }

        impl RefUnwindSafe for A {}

        unsafe impl Allocator for A {
            fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
                let count = self.allocation_count.get();
                self.allocation_count.set(count + 1);

                if count == 0 {
                    self.allocator.allocate(layout)
                } else {
                    panic!("intentional panic when shrinking")
                }
            }

            unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
                unsafe { self.allocator.deallocate(ptr, layout) };
            }
        }

        let base = A::default();
        let bump: Bump<&A, C> = Bump::with_size_in(512, &base);
        bump.allocate(Layout::new::<u8>()).unwrap();

        assert_eq!(bump.stats().allocated(), 1);

        // allocate something and then shrink it with a greater alignment
        let old_layout = Layout::from_size_align(3, 1).unwrap();
        let new_layout = Layout::from_size_align(2, 2048).unwrap();

        let old_ptr = bump.allocate(old_layout).unwrap().cast::<u8>();
        assert_eq!(bump.stats().allocated(), 4);

        catch_unwind(|| unsafe { bump.shrink(old_ptr, old_layout, new_layout) }).unwrap_err();
        assert_eq!(bump.stats().allocated(), 4);
    }
}

use helper::{Testable, expected_drops};

use crate::common::{DisaligningAllocator, up_and_down};

mod helper {
    use std::{
        array,
        cell::Cell,
        hint::black_box,
        panic::{AssertUnwindSafe, RefUnwindSafe, UnwindSafe},
        string::String,
        thread_local,
    };

    pub fn assert_initialized(iter: impl IntoIterator) {
        // make sure items are initialized
        // miri will catch that
        for item in iter {
            black_box(item);
        }
    }

    pub(super) trait Testable: Clone + Default + UnwindSafe + RefUnwindSafe {
        fn array<const N: usize>() -> [Self; N] {
            array::from_fn(|_| Default::default())
        }
    }

    impl Testable for Wrap<i32> {}
    impl Testable for Wrap<()> {}

    thread_local! {
        static MAX_CLONES: Cell<usize> = const { Cell::new(0) };
        static CLONES: Cell<usize> = const { Cell::new(0) };
        static DROPS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Default)]
    pub(super) struct Cfg {
        max_clones: Option<usize>,
        expected_drops: usize,
        expected_msg: Option<&'static str>,
    }

    impl Cfg {
        pub(super) fn panic_on_clone(mut self, amount: usize) -> Self {
            self.max_clones = Some(amount);
            self
        }

        pub(super) fn expected_msg(mut self, msg: &'static str) -> Self {
            self.expected_msg = Some(msg);
            self
        }

        pub(super) fn run(self, f: impl FnOnce()) {
            let Self {
                max_clones,
                expected_drops,
                expected_msg: msg,
            } = self;
            let msg = msg.unwrap_or("too many clones");

            MAX_CLONES.set(max_clones.unwrap_or(usize::MAX));
            let panic = catch(AssertUnwindSafe(f)).unwrap_err();
            assert_eq!(panic, msg);
            assert_eq!(DROPS.get(), expected_drops);

            MAX_CLONES.set(0);
            CLONES.set(0);
            DROPS.set(0);
        }
    }

    pub(super) fn expected_drops(amount: usize) -> Cfg {
        Cfg {
            expected_drops: amount,
            expected_msg: None,
            max_clones: None,
        }
    }

    #[derive(Default)]
    pub(super) struct Wrap<T>(pub(super) T);

    impl<T: UnwindSafe> UnwindSafe for Wrap<T> {}
    impl<T: RefUnwindSafe> RefUnwindSafe for Wrap<T> {}

    impl<T: Clone> Clone for Wrap<T> {
        fn clone(&self) -> Self {
            let count = CLONES.get();
            if count >= MAX_CLONES.get() {
                panic!("too many clones");
            } else {
                CLONES.set(count + 1);
                Wrap(self.0.clone())
            }
        }
    }

    impl<T> Drop for Wrap<T> {
        fn drop(&mut self) {
            DROPS.set(DROPS.get() + 1);
        }
    }

    fn catch<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> Result<R, String> {
        let hook = std::panic::take_hook();

        std::panic::set_hook(std::boxed::Box::new(|_| {
            // be quiet
        }));

        let result = match std::panic::catch_unwind(f) {
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
        };

        std::panic::set_hook(hook);

        result
    }
}
