#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::{
    alloc::Layout,
    cell::Cell,
    mem::ManuallyDrop,
    panic::{RefUnwindSafe, catch_unwind},
    ptr::NonNull,
};

use bump_scope::{
    Bump, BumpVec, MutBumpVec, MutBumpVecRev,
    alloc::{AllocError, Allocator, Global},
    bump_vec, mut_bump_vec, mut_bump_vec_rev,
};

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

    mut_bump_vec_from_elem_in

    into_iter

    bump_vec_extend_from_slice

    mut_bump_vec_extend_from_slice

    mut_bump_vec_rev_extend_from_slice
}

fn init_clone<T: Testable>() {
    expected_drops(3).panic_on_clone(3).run(|| {
        let original = ManuallyDrop::new(T::array::<5>());
        let bump: Bump = Bump::new();
        let uninit = bump.alloc_uninit_slice::<T>(5);
        let _init = uninit.init_clone(&*original);
    });
}

fn init_fill<T: Testable>() {
    expected_drops(4).panic_on_clone(3).run(|| {
        let bump: Bump = Bump::new();
        let uninit = bump.alloc_uninit_slice::<T>(5);
        let _init = uninit.init_fill(T::default());
    });
}

fn init_fill_with<T: Testable>() {
    expected_drops(3).panic_on_clone(3).run(|| {
        let original = ManuallyDrop::new(T::default());
        let bump: Bump = Bump::new();
        let uninit = bump.alloc_uninit_slice::<T>(5);
        let _init = uninit.init_fill_with(|| (*original).clone());
    });
}

fn mut_bump_vec_from_elem_in<T: Testable>() {
    expected_drops(4).panic_on_clone(3).run(|| {
        let mut bump: Bump = Bump::new();
        let _vec = MutBumpVec::from_elem_in(T::default(), 5, &mut bump);
    });
}

fn into_iter<T: Testable>() {
    expected_drops(5).expected_msg("whoops").run(|| {
        let mut bump: Bump = Bump::new();
        let vec = mut_bump_vec![in &mut bump; T::default(); 5];

        #[expect(clippy::manual_assert)]
        for (i, _) in vec.into_iter().enumerate() {
            if i == 3 {
                panic!("whoops");
            }
        }
    });
}

fn bump_vec_extend_from_slice<T: Testable>() {
    let bump: Bump = Bump::new();
    let mut vec: BumpVec<T, _> = bump_vec![in &bump];
    let slice = ManuallyDrop::new(T::array::<5>());

    expected_drops(0).panic_on_clone(3).run(|| {
        vec.extend_from_slice_clone(&*slice);
    });

    assert_eq!(vec.len(), 3);
    assert_initialized(vec);
}

fn mut_bump_vec_extend_from_slice<T: Testable>() {
    let mut bump: Bump = Bump::new();
    let mut vec: MutBumpVec<T, _> = mut_bump_vec![in &mut bump];
    let slice = ManuallyDrop::new(T::array::<5>());

    expected_drops(0).panic_on_clone(3).run(|| {
        vec.extend_from_slice_clone(&*slice);
    });

    assert_eq!(vec.len(), 3);
    assert_initialized(vec);
}

fn mut_bump_vec_rev_extend_from_slice<T: Testable>() {
    let mut bump: Bump = Bump::new();
    let mut vec: MutBumpVecRev<T, _> = mut_bump_vec_rev![in &mut bump];
    let slice = ManuallyDrop::new(T::array::<5>());

    expected_drops(0).panic_on_clone(3).run(|| {
        vec.extend_from_slice_clone(&*slice);
    });

    assert_eq!(vec.len(), 3);
    assert_initialized(vec);
}

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
    test_shrink_unfit_in_another_chunk
}

fn test_shrink_unfit_in_another_chunk<const UP: bool>() {
    // Here we make sure to create a first chunk that is only aligned to the required chunk
    // alignment and no more, so when a shrink is done with a high alignment it can't
    // fit in the first chunk.

    #[derive(Default)]
    struct A {
        allocation_count: Cell<usize>,
    }

    impl RefUnwindSafe for A {}

    const LAYOUT: Layout = unsafe { Layout::from_size_align_unchecked(1024, 1024) };

    unsafe impl Allocator for A {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            let count = self.allocation_count.get();
            self.allocation_count.set(count + 1);

            if count == 0 {
                assert!(layout.align() < LAYOUT.align());
                assert!(layout.size() < (LAYOUT.size() - layout.align()));

                let slice = Global.allocate(LAYOUT)?;

                unsafe {
                    let len = slice.len() - layout.align();
                    let ptr = slice.cast::<u8>().add(layout.align());
                    Ok(NonNull::slice_from_raw_parts(ptr, len))
                }
            } else {
                panic!("intentional panic when shrinking")
            }
        }

        unsafe fn deallocate(&self, ptr: NonNull<u8>, _: Layout) {
            unsafe {
                let ptr = ptr.as_ptr();
                let ptr = ptr.with_addr(down_align(ptr.addr(), LAYOUT.align()));
                let ptr = NonNull::new_unchecked(ptr);
                Global.deallocate(ptr, LAYOUT);
            }
        }
    }

    let bump: Bump<A> = Bump::with_size(0);
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

fn down_align(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    let mask = align - 1;
    addr & !mask
}

use helper::{Testable, assert_initialized, expected_drops};

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
