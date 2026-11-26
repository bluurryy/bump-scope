use core::{
    alloc::{AllocError, Allocator, Layout},
    fmt,
    marker::PhantomData,
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

use alloc::alloc::Global;

use bump_scope_core::{
    BumpScope as CoreBumpScope,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG},
    stats::Stats,
};

use crate::{
    BumpClaimGuard, BumpScopeGuard, NoDrop,
    util::{transmute_mut, transmute_ref, transmute_value},
};

#[cfg(feature = "panic-on-alloc")]
use crate::error_mode::panic_on_error;

/// A bump allocation scope.
///
/// A `BumpScope`'s allocations are live for `'a`, which is the lifetime of its associated `BumpScopeGuard` or `scoped` closure.
///
/// This type is provided as a parameter to the closure of [`scoped`], or created
/// by [`BumpScopeGuard::scope`]. A [`Bump`] can also be turned into a `BumpScope` using
/// [`as_bump_scope`] and [`as_mut_bump_scope`].
///
/// [`scoped`]: crate::BumpScope::scoped
/// [`BumpScopeGuard::scope`]: crate::BumpScopeGuard::scope
/// [`Bump`]: crate::Bump
/// [`as_bump_scope`]: crate::AsBumpScope::as_bump_scope
/// [`as_mut_bump_scope`]: crate::AsMutBumpScope::as_mut_bump_scope
/// [`reset`]: crate::Bump::reset
#[repr(transparent)]
pub struct BumpScope<'a, const C: BumpConfig = DEFAULT_BUMP_CONFIG, A: Allocator = Global> {
    pub(crate) core: CoreBumpScope<C, A>,

    /// Marks the lifetime of the mutably borrowed `BumpScopeGuard`.
    pub(crate) marker: PhantomData<&'a u8>,
}

impl<const C: BumpConfig, A: Allocator + UnwindSafe + RefUnwindSafe> UnwindSafe for BumpScope<'_, C, A> {}
impl<const C: BumpConfig, A: Allocator + RefUnwindSafe> RefUnwindSafe for BumpScope<'_, C, A> {}

impl<const C: BumpConfig, A: Allocator> fmt::Debug for BumpScope<'_, C, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.core.fmt(f)
    }
}

impl<const C: BumpConfig, A: Allocator> AsRef<CoreBumpScope<C, A>> for BumpScope<'_, C, A> {
    fn as_ref(&self) -> &CoreBumpScope<C, A> {
        &self.core
    }
}

impl<const C: BumpConfig, A: Allocator> AsMut<CoreBumpScope<C, A>> for BumpScope<'_, C, A> {
    fn as_mut(&mut self) -> &mut CoreBumpScope<C, A> {
        &mut self.core
    }
}

/// # Create a child scope.
impl<const C: BumpConfig, A: Allocator> BumpScope<'_, C, A> {
    /// Calls `f` with a new child scope.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// bump.scoped(|bump| {
    ///     bump.alloc_str("Hello, world!").into_ref();
    ///     assert_eq!(bump.allocated(), 13);
    /// });
    ///
    /// assert_eq!(bump.allocated(), 0);
    /// ```
    ///
    /// [claimed]: crate::BumpScope::claim
    #[inline(always)]
    pub fn scoped<R>(&mut self, f: impl FnOnce(&mut BumpScope<'_, C, A>) -> R) -> R {
        let mut guard = self.begin_scope();
        f(guard.scope())
    }

    /// Creates a new [`BumpScopeGuard`].
    ///
    /// This allows for creation of child scopes.
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// {
    ///     let mut guard = bump.begin_scope();
    ///     let bump = guard.scope();
    ///     bump.alloc_str("Hello, world!").into_ref();
    ///     assert_eq!(bump.allocated(), 13);
    /// }
    ///
    /// assert_eq!(bump.allocated(), 0);
    /// ```
    ///
    /// [claimed]: crate::BumpScope::claim
    #[inline(always)]
    pub fn begin_scope(&mut self) -> BumpScopeGuard<'_, C, A> {
        BumpScopeGuard::new(unsafe { transmute_mut(self) })
    }
}

/// # Miscellaneous.
impl<'a, const C: BumpConfig, A: Allocator> BumpScope<'a, C, A> {
    pub(crate) unsafe fn from_core(core: CoreBumpScope<C, A>) -> Self {
        Self {
            core,
            marker: PhantomData,
        }
    }

    pub(crate) unsafe fn ref_from_core(core: &CoreBumpScope<C, A>) -> &Self {
        // SAFETY: we are repr(transparent)
        unsafe { transmute_ref(core) }
    }

    pub(crate) unsafe fn mut_from_core(core: &mut CoreBumpScope<C, A>) -> &mut Self {
        // SAFETY: we are repr(transparent)
        unsafe { transmute_mut(core) }
    }

    /// Claims exclusive access to the bump allocator from a shared reference.
    ///
    /// This makes it possible to enter scopes while a there are still outstanding
    /// references to that bump allocator.
    ///
    /// The `claim` call replaces this allocator with a dummy allocator while the returned `BumpClaimGuard`
    /// is live. This dummy allocator errors on `allocate` / `grow`, does nothing on `deallocate` / `shrink` and
    /// reports an empty bump allocator from the `stats` api.
    ///
    /// # Panics
    /// Panics if the bump allocator is already claimed.
    ///
    /// # Compile-Time Assertions
    /// Fails to compile when not [`claimable`].
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, extra};
    ///
    /// let bump: Bump = Bump::new();
    /// let vec1: Vec<u8, _> = extra::vec![in &bump; 1, 2, 3];
    /// let vec2: Vec<u8, _> = extra::vec![in &bump; 4, 5, 6];
    ///
    /// bump.claim().scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    ///
    /// assert_eq!(vec1, [1, 2, 3]);
    /// assert_eq!(vec2, [4, 5, 6]);
    /// ```
    ///
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    pub fn claim(&self) -> BumpClaimGuard<'_, 'a, C, A> {
        BumpClaimGuard::new(self)
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, C> {
        // SAFETY: stats are valid while the allocations are valid
        unsafe { transmute_value(self.core.stats()) }
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    pub fn allocator(&self) -> &A {
        self.core.allocator()
    }

    /// Converts this `&mut BumpScope` into an owned `BumpScope`.
    ///
    /// Allocates a chunk if none has been allocated yet.
    ///
    /// This can be used to remove an indirection and allow
    /// for more config conversions via [`change_config`].
    ///
    /// # Panics
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// Panics if the allocation fails.
    ///
    /// [claimed]: crate::BumpScope::claim
    /// [`change_config`]: BumpScope::change_config
    /// [`mut_change_config`]: BumpScope::mut_change_config
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn by_value(&mut self) -> BumpScope<'_, C, A> {
        BumpScope {
            core: panic_on_error(self.core.by_value()),
            marker: PhantomData,
        }
    }

    /// Converts this `&mut BumpScope` into an owned `BumpScope`.
    ///
    /// Allocates a chunk if none has been allocated yet.
    ///
    /// This can be used to remove an indirection and allow
    /// for more config conversions via [`change_config`].
    ///
    /// # Errors
    /// Errors if the bump allocator is currently [claimed].
    ///
    /// Errors if the allocation fails.
    ///
    /// [claimed]: crate::BumpScope::claim
    /// [`change_config`]: BumpScope::change_config
    /// [`mut_change_config`]: BumpScope::mut_change_config
    #[inline(always)]
    pub fn try_by_value(&mut self) -> Result<BumpScope<'_, C, A>, AllocError> {
        Ok(BumpScope {
            core: self.core.by_value()?,
            marker: PhantomData,
        })
    }
}

/// # Change config parameters.
impl<'a, const C: BumpConfig, A: Allocator> BumpScope<'a, C, A> {
    /// Converts this `BumpScope` into a `BumpScope` with a new config.
    ///
    /// # Panics
    /// - Panics if [`claimable`] becomes false and [`self.is_claimed()`]
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`self.is_claimed()`]: crate::traits::BumpAllocator::is_claimed
    #[inline]
    pub fn change_config<const NEW_C: BumpConfig>(self) -> BumpScope<'a, NEW_C, A> {
        // SAFETY: the allocation lifetime stays the same
        unsafe { BumpScope::from_core(self.core.change_config()) }
    }

    /// Borrows this `BumpScope` with a new config.
    ///
    /// # Panics
    /// - Panics if [`const_constructible`] becomes false and `self` has no chunks
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    /// - Fails to compile if [`claimable`] changes
    /// - Fails to compile if [`min_align`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    /// [`min_align`]: crate::config::BumpConfigBuilder::min_align
    #[inline]
    pub fn ref_change_config<const NEW_C: BumpConfig>(&self) -> &BumpScope<'a, NEW_C, A> {
        // SAFETY: the allocation lifetime stays the same
        unsafe { BumpScope::ref_from_core(self.core.ref_change_config()) }
    }

    /// Borrows this `BumpScope` mutably with a new config.
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    /// - Fails to compile if [`claimable`] changes
    /// - Fails to compile if [`const_constructible`] changes
    /// - Fails to compile if [`min_align`] decreases
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    /// [`min_align`]: crate::config::BumpConfigBuilder::min_align
    #[inline]
    pub fn mut_change_config<const NEW_C: BumpConfig>(&mut self) -> &mut BumpScope<'a, NEW_C, A> {
        // SAFETY: the allocation lifetime stays the same
        unsafe { BumpScope::mut_from_core(self.core.mut_change_config()) }
    }

    /// Calls `f` with this scope but with a new config.
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    /// - Fails to compile if [`claimable`] changes
    /// - Fails to compile if [`const_constructible`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    /// # Examples
    ///
    /// Increase the minimum alignment:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, AsMutBumpScope, config::{BumpConfig, BumpConfigBuilder}};
    ///
    /// const MIN_ALIGN_8: BumpConfig = BumpConfigBuilder::new().min_align(8).build();
    ///
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_bump_scope();
    ///
    /// // here we're allocating with a `MIN_ALIGN` of `1`
    /// let foo = bump.alloc_str("foo").into_ref();
    /// assert_eq!(bump.allocated(), 3);
    ///
    /// let bar = bump.with_config::<MIN_ALIGN_8, _>(|bump| {
    ///     // in here the bump position has been aligned to `8`
    ///     assert_eq!(bump.allocated(), 8);
    ///     assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///
    ///     // make some allocations that benefit from the higher `MIN_ALIGN` of `8`
    ///     let bar = bump.alloc(0u64).into_ref();
    ///     assert_eq!(bump.allocated(), 16);
    ///  
    ///     // the bump position will stay aligned to `8`
    ///     bump.alloc(0u8).into_ref();
    ///     assert_eq!(bump.allocated(), 24);
    ///
    ///     bar
    /// });
    ///
    /// assert_eq!(bump.allocated(), 24);
    ///
    /// // continue making allocations with a `MIN_ALIGN` of `1`
    /// let baz = bump.alloc_str("baz").into_ref();
    /// assert_eq!(bump.allocated(), 24 + 3);
    ///
    /// dbg!(foo, bar, baz);
    /// ```
    ///
    /// Decrease the minimum alignment:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, AsMutBumpScope, config::{BumpConfig, BumpConfigBuilder}};
    ///
    /// const MIN_ALIGN_8: BumpConfig = BumpConfigBuilder::new().min_align(8).build();
    /// const MIN_ALIGN_1: BumpConfig = BumpConfigBuilder::new().build();
    ///
    /// let mut bump: Bump<MIN_ALIGN_8> = Bump::new();
    /// let bump = bump.as_mut_bump_scope();
    ///
    /// // make some allocations that benefit from the `MIN_ALIGN` of `8`
    /// let foo = bump.alloc(0u64).into_ref();
    ///
    /// let bar = bump.with_config::<MIN_ALIGN_1, _>(|bump| {
    ///     // make some allocations that benefit from the lower `MIN_ALIGN` of `1`
    ///     let bar = bump.alloc(0u8).into_ref();
    ///
    ///     // the bump position will not get aligned to `8` in here
    ///     assert_eq!(bump.allocated(), 8 + 1);
    ///
    ///     bar
    /// });
    ///
    /// // after `with_config()`, the bump position will be aligned to `8` again
    /// // to satisfy our `MIN_ALIGN`
    /// assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    /// assert_eq!(bump.allocated(), 16);
    ///
    /// // continue making allocations that benefit from the `MIN_ALIGN` of `8`
    /// let baz = bump.alloc(0u64);
    ///
    /// dbg!(foo, bar, baz);
    /// ```
    #[inline(always)]
    pub fn with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<'a, NEW_C, A>) -> R) -> R {
        self.core
            .with_config(move |scope| unsafe { f(BumpScope::mut_from_core(scope)) })
    }

    /// Calls `f` on a child scope with a new config.
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    ///
    /// # Panics
    ///
    /// - Panics if [`claimable`] becomes false and [`self.is_claimed()`]
    /// - Panics if [`const_constructible`] becomes false and `self` has no chunks
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`self.is_claimed()`]: crate::traits::BumpAllocator::is_claimed
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, config::{BumpConfig, BumpConfigBuilder}};
    ///
    /// const CONFIG_MIN_ALIGN_8: BumpConfig = BumpConfigBuilder::new()
    ///     .min_align(8)
    ///     .build();
    ///
    /// let mut bump: Bump = Bump::new();
    ///
    /// // bump starts off by being aligned to 16
    /// assert!(bump.stats().bump_position().unwrap().addr().get() % 16 == 0);
    ///
    /// // allocate one byte
    /// bump.alloc(1u8).into_ref();
    ///
    /// // now the bump is only aligned to 1
    /// // (if our `MIN_ALIGN` was higher, it would be that)
    /// assert!(bump.stats().bump_position().unwrap().addr().get() % 2 == 1);
    /// assert_eq!(bump.allocated(), 1);
    ///
    /// bump.scoped_with_config::<CONFIG_MIN_ALIGN_8, ()>(|bump| {
    ///    // in here, the bump will have the specified minimum alignment of 8
    ///    assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///    assert_eq!(bump.allocated(), 8);
    ///
    ///    // allocating a value with its size being a multiple of 8 will no longer have
    ///    // to align the bump pointer before allocation
    ///    bump.alloc(1u64).into_ref();
    ///    assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///    assert_eq!(bump.allocated(), 16);
    ///    
    ///    // allocating a value smaller than the minimum alignment must align the bump pointer
    ///    // after the allocation, resulting in some wasted space
    ///    bump.alloc(1u8).into_ref();
    ///    assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///    assert_eq!(bump.allocated(), 24);
    /// });
    ///
    /// assert_eq!(bump.allocated(), 1);
    /// ```
    ///
    /// [claimed]: crate::BumpScope::claim
    #[inline(always)]
    pub fn scoped_with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<NEW_C, A>) -> R) -> R {
        self.core
            .scoped_with_config(move |scope| unsafe { f(BumpScope::mut_from_core(scope)) })
    }
}

impl<const C: BumpConfig, A: Allocator> NoDrop for BumpScope<'_, C, A> {}

unsafe impl<const C: BumpConfig, A: Allocator> Allocator for BumpScope<'_, C, A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.core.allocate(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.core.deallocate(ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.core.grow(ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.core.grow_zeroed(ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.core.shrink(ptr, old_layout, new_layout) }
    }
}
