use core::mem::MaybeUninit;

use zerocopy::FromZeroes;

use crate::{define_alloc_methods, BaseAllocator, Bump, BumpBox, BumpScope, MinimumAlignment, SupportedMinimumAlignment};

impl<'a, T> BumpBox<'a, MaybeUninit<T>>
where
    T: FromZeroes,
{
    /// Initializes `self` by filling it with zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit::<i32>();
    /// let init = uninit.init_zeroed();
    /// assert_eq!(*init, 0);
    /// ```
    pub fn init_zeroed(self) -> BumpBox<'a, T> {
        unsafe {
            self.ptr.as_ptr().write_bytes(0, 1);
            self.assume_init()
        }
    }
}

impl<'a, T> BumpBox<'a, [MaybeUninit<T>]>
where
    T: FromZeroes,
{
    /// Initializes `self` by filling it with zeroes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit_slice::<i32>(10);
    /// let init = uninit.init_zeroed();
    /// assert_eq!(*init, [0; 10]);
    /// ```
    pub fn init_zeroed(self) -> BumpBox<'a, [T]> {
        unsafe {
            let len = self.len();

            self.ptr.as_ptr().cast::<T>().write_bytes(0, len);
            self.assume_init()
        }
    }
}

define_alloc_methods! {
    macro alloc_zeroed_methods

    /// Allocate a zeroed object.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let zero = bump.alloc_zeroed::<i32>();
    /// assert_eq!(*zero, 0)
    /// ```
    for pub fn alloc_zeroed
    for pub fn try_alloc_zeroed
    fn generic_alloc_zeroed<{T}>(&self) -> BumpBox<T> | BumpBox<'a, T>
    where {
        T: FromZeroes
    } in {
        Ok(self.generic_alloc_uninit::<B, T>()?.init_zeroed())
    }

    /// Allocate a zeroed object slice.
    impl
    do examples
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let zeroes = bump.alloc_zeroed_slice::<i32>(3);
    /// assert_eq!(*zeroes, [0; 3])
    /// ```
    for pub fn alloc_zeroed_slice
    for pub fn try_alloc_zeroed_slice
    fn generic_alloc_zeroed_slice<{T}>(&self, len: usize) -> BumpBox<[T]> | BumpBox<'a, [T]>
    where {
        T: FromZeroes
    } in {
        Ok(self.generic_alloc_uninit_slice::<B, T>(len)?.init_zeroed())
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    BumpScope<'a, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    alloc_zeroed_methods!(BumpScope);
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    alloc_zeroed_methods!(Bump);
}
