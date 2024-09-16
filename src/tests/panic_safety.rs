use crate::{mut_bump_vec, Bump, MutBumpVec};
use core::{
    cell::Cell,
    mem::ManuallyDrop,
    panic::{RefUnwindSafe, UnwindSafe},
};
use std::{
    panic::catch_unwind,
    sync::{Mutex, PoisonError},
};

fn check_t<T: Clone + Default + UnwindSafe + RefUnwindSafe>() {
    thread_local! {
        static MAX_CLONES: Cell<usize> = const { Cell::new(0) };
        static CLONES: Cell<usize> = const { Cell::new(0) };
        static DROPS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Default)]
    struct Cfg {
        max_clones: Option<usize>,
        expected_drops: usize,
        msg: Option<&'static str>,
    }

    impl Cfg {
        fn max_clones(mut self, amount: usize) -> Self {
            self.max_clones = Some(amount);
            self
        }

        fn expected_drops(mut self, amount: usize) -> Self {
            self.expected_drops = amount;
            self
        }

        fn expected_msg(mut self, msg: &'static str) -> Self {
            self.msg = Some(msg);
            self
        }

        fn run(self, f: fn()) {
            let Self {
                max_clones,
                expected_drops,
                msg,
            } = self;
            let msg = msg.unwrap_or("too many clones");

            MAX_CLONES.set(max_clones.unwrap_or(usize::MAX));
            let panic = catch(f).unwrap_err();
            assert_eq!(panic, msg);
            assert_eq!(DROPS.get(), expected_drops);

            MAX_CLONES.set(0);
            CLONES.set(0);
            DROPS.set(0);
        }
    }

    fn cfg() -> Cfg {
        Cfg::default()
    }

    struct Foo<T>(T);

    impl<T: Clone> Clone for Foo<T> {
        fn clone(&self) -> Self {
            let count = CLONES.get();
            if count >= MAX_CLONES.get() {
                panic!("too many clones");
            } else {
                CLONES.set(count + 1);
                Foo(self.0.clone())
            }
        }
    }

    impl<T> Drop for Foo<T> {
        fn drop(&mut self) {
            DROPS.set(DROPS.get() + 1);
        }
    }

    // Check `init_clone`
    cfg().max_clones(3).expected_drops(3).run(|| {
        let original = ManuallyDrop::new([
            Foo(T::default()),
            Foo(T::default()),
            Foo(T::default()),
            Foo(T::default()),
            Foo(T::default()),
        ]);
        let bump: Bump = Bump::new();
        let uninit = bump.alloc_uninit_slice::<Foo<T>>(5);
        let _init = uninit.init_clone(&*original);
    });

    // Check `init_fill`
    cfg().max_clones(3).expected_drops(4).run(|| {
        let bump: Bump = Bump::new();
        let uninit = bump.alloc_uninit_slice::<Foo<T>>(5);
        let _init = uninit.init_fill(Foo(T::default()));
    });

    // Check `init_fill_with`
    cfg().max_clones(3).expected_drops(3).run(|| {
        let original = ManuallyDrop::new(Foo(T::default()));
        let bump: Bump = Bump::new();
        let uninit = bump.alloc_uninit_slice::<Foo<T>>(5);
        let _init = uninit.init_fill_with(|| (*original).clone());
    });

    // Check `MutBumpVec::from_elem_in`
    cfg().max_clones(3).expected_drops(4).run(|| {
        let mut bump: Bump = Bump::new();
        let _vec = MutBumpVec::from_elem_in(Foo(T::default()), 5, &mut bump);
    });

    // Check `IntoIter`
    cfg().expected_drops(5).expected_msg("whoops").run(|| {
        let mut bump: Bump = Bump::new();
        let vec = mut_bump_vec![in bump; Foo(T::default()); 5];

        #[allow(clippy::manual_assert)]
        for (i, _) in vec.into_iter().enumerate() {
            if i == 3 {
                panic!("whoops");
            }
        }
    });
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn check() {
    check_t::<i32>();
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn check_zst() {
    check_t::<()>();
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
