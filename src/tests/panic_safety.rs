use crate::{bump_vec, mut_bump_vec, mut_bump_vec_rev, Bump, BumpVec, MutBumpVec, MutBumpVecRev};
use core::{
    cell::Cell,
    mem::ManuallyDrop,
    panic::{RefUnwindSafe, UnwindSafe},
};
use std::{
    hint::black_box,
    panic::catch_unwind,
    sync::{Mutex, PoisonError},
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
        let vec = mut_bump_vec![in bump; T::default(); 5];

        #[allow(clippy::manual_assert)]
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
    let mut vec: MutBumpVec<T> = mut_bump_vec![in bump];
    let slice = ManuallyDrop::new(T::array::<5>());

    expected_drops(0).panic_on_clone(3).run(|| {
        vec.extend_from_slice_clone(&*slice);
    });

    assert_eq!(vec.len(), 3);
    assert_initialized(vec);
}

fn mut_bump_vec_rev_extend_from_slice<T: Testable>() {
    let mut bump: Bump = Bump::new();
    let mut vec: MutBumpVecRev<T> = mut_bump_vec_rev![in bump];
    let slice = ManuallyDrop::new(T::array::<5>());

    expected_drops(0).panic_on_clone(3).run(|| {
        vec.extend_from_slice_clone(&*slice);
    });

    assert_eq!(vec.len(), 3);
    assert_initialized(vec);
}

use helper::{assert_initialized, expected_drops, Testable};

mod helper {
    use core::{
        array,
        cell::Cell,
        hint::black_box,
        panic::{AssertUnwindSafe, RefUnwindSafe, UnwindSafe},
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

        std::panic::set_hook(Box::new(|_| {
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
