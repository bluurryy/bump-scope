use std::{cell::Cell, convert::Infallible, fmt, thread_local};

thread_local! {
    static DROPS: Cell<usize> = const { Cell::new(0) };
    static CLONES: Cell<usize> = const { Cell::new(0) };
    static DEFAULTS: Cell<usize> = const { Cell::new(0) };
}

#[repr(transparent)]
pub(crate) struct TestWrap<T>(pub T);

impl<T: fmt::Debug> fmt::Debug for TestWrap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<T: PartialEq> PartialEq for TestWrap<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: Default> Default for TestWrap<T> {
    fn default() -> Self {
        DEFAULTS.set(DEFAULTS.get() + 1);
        TestWrap(T::default())
    }
}

impl<T: Clone> Clone for TestWrap<T> {
    fn clone(&self) -> Self {
        CLONES.set(CLONES.get() + 1);
        Self(self.0.clone())
    }
}

impl<T> Drop for TestWrap<T> {
    fn drop(&mut self) {
        DROPS.set(DROPS.get() + 1);
    }
}

impl<T> TestWrap<T> {
    pub(crate) fn peel_slice(this: &[TestWrap<T>]) -> &[T] {
        unsafe { &*(this as *const [TestWrap<T>] as *const [T]) }
    }
}

impl TestWrap<Infallible> {
    pub(crate) fn expect() -> TestZstExpect {
        TestZstExpect::default()
    }

    #[expect(dead_code)]
    pub(crate) fn current_defaults() -> usize {
        DEFAULTS.get()
    }

    #[expect(dead_code)]
    pub(crate) fn current_clones() -> usize {
        CLONES.get()
    }

    #[expect(dead_code)]
    pub(crate) fn current_drops() -> usize {
        DROPS.get()
    }
}

#[derive(Default)]
pub(crate) struct TestZstExpect {
    drops: usize,
    clones: usize,
    defaults: usize,
}

impl TestZstExpect {
    pub(crate) fn drops(mut self, amount: usize) -> Self {
        self.drops = amount;
        self
    }

    pub(crate) fn clones(mut self, amount: usize) -> Self {
        self.clones = amount;
        self
    }

    pub(crate) fn defaults(mut self, amount: usize) -> Self {
        self.defaults = amount;
        self
    }

    pub(crate) fn run<R>(self, f: impl FnOnce() -> R) -> R {
        DROPS.set(0);
        CLONES.set(0);
        DEFAULTS.set(0);

        let result = f();

        macro_rules! expected {
            ($expected:ident, $actual:ident, $what:literal) => {
                if self.$expected != $actual.get() {
                    panic!(
                        "expected {expected} {what}, got {actual}",
                        expected = self.$expected,
                        actual = $actual.get(),
                        what = $what,
                    )
                }
            };
        }

        expected!(drops, DROPS, "drops");
        expected!(clones, CLONES, "clones");
        expected!(defaults, DEFAULTS, "defaults");

        result
    }
}
