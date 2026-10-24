use core::{
    alloc::{AllocError, Allocator, Layout},
    fmt,
    marker::PhantomData,
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

use alloc::alloc::Global;

use crate::{
    BumpClaimGuard, BumpScopeGuard, NoDrop, allocator_impl,
    bump_align_guard::BumpAlignGuard,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG, config},
    polyfill::{transmute_mut, transmute_ref, transmute_value},
    raw_bump::RawBump,
    stats::{AnyStats, Stats},
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// A bump allocation scope.
///
/// A `BumpScope`'s allocations are live for `'a`, which is the lifetime of its associated `BumpScopeGuard` or `scoped` closure.
///
/// `BumpScope` has mostly same api as [`Bump`].
///
/// This type is provided as a parameter to the closure of [`scoped`], or created
/// by [`BumpScopeGuard::scope`]. A [`Bump`] can also be turned into a `BumpScope` using
/// [`as_scope`], [`as_mut_scope`] or `from` / `into`.
///
/// [`scoped`]: crate::BumpScope::scoped
/// [`BumpScopeGuard::scope`]: crate::BumpScopeGuard::scope
/// [`Bump`]: crate::Bump
/// [`as_scope`]: crate::Bump::as_scope
/// [`as_mut_scope`]: crate::Bump::as_mut_scope
/// [`reset`]: crate::Bump::reset
#[repr(transparent)]
pub struct BumpScope<'a, A = Global, const C: BumpConfig = DEFAULT_BUMP_CONFIG> {
    pub(crate) raw: RawBump<A, C>,

    /// Marks the lifetime of the mutably borrowed `BumpScopeGuard`.
    pub(crate) marker: PhantomData<&'a ()>,
}

impl<A, const C: BumpConfig> UnwindSafe for BumpScope<'_, A, C> where A: RefUnwindSafe {}

impl<A, const C: BumpConfig> RefUnwindSafe for BumpScope<'_, A, C> where A: RefUnwindSafe {}

impl<A, const C: BumpConfig> fmt::Debug for BumpScope<'_, A, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(self.stats()).debug_format("BumpScope", f)
    }
}

impl<A, const C: BumpConfig> BumpScope<'_, A, C>
where
    A: Allocator,
{
    /// Converts this `&mut BumpScope` into an owned `BumpScope`.
    ///
    /// Allocates a chunk if none has been allocated yet.
    ///
    /// This can be used to remove an indirection, simplify type signatures and allow
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
    pub fn by_value(&mut self) -> BumpScope<'_, A, C> {
        panic_on_error(self.raw.make_allocated());

        BumpScope {
            raw: self.raw.clone(),
            marker: PhantomData,
        }
    }

    /// Converts this `&mut BumpScope` into an owned `BumpScope`.
    ///
    /// Allocates a chunk if none has been allocated yet.
    ///
    /// This can be used to remove an indirection, simplify type signatures and allow
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
    pub fn try_by_value(&mut self) -> Result<BumpScope<'_, A, C>, AllocError> {
        self.raw.make_allocated::<AllocError>()?;

        Ok(BumpScope {
            raw: self.raw.clone(),
            marker: PhantomData,
        })
    }
}

impl<'a, A, const C: BumpConfig> BumpScope<'a, A, C> {
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, A, C> {
        self.raw.stats()
    }

    /// Converts this `BumpScope` into a `BumpScope` with a new config.
    ///
    /// # Panics
    /// - Panics if [`claimable`] becomes false and [`self.is_claimed()`]
    ///
    /// # Fails
    /// - Fails to compile if [`upwards`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`self.is_claimed()`]: crate::traits::BumpAllocator::is_claimed
    #[inline]
    pub fn change_config<const NEW_C: BumpConfig>(self) -> BumpScope<'a, A, NEW_C> {
        RawBump::<A, C>::const_assert_upwards_matches::<NEW_C>();

        // Since `self` is by value, this can only happen if a claim guard was forgotten.
        self.raw.assert_convertible_claimable::<NEW_C>();

        // No need to check const_constructible, a scope by value must have been created
        // by `(try_)by_value` which always makes it allocated.

        self.raw.align_to(config!(NEW_C, min_align));

        unsafe { transmute_value(self) }
    }

    /// Borrows this `BumpScope` with a new config.
    ///
    /// # Panics
    /// - Panics if [`const_constructible`] becomes false and `self` has no chunks
    ///
    /// # Fails
    /// - Fails to compile if [`upwards`] changes
    /// - Fails to compile if [`claimable`] changes
    /// - Fails to compile if [`min_align`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    /// [`min_align`]: crate::config::BumpConfigBuilder::min_align
    #[inline]
    pub fn ref_change_config<const NEW_C: BumpConfig>(&self) -> &BumpScope<'a, A, NEW_C> {
        self.raw.ensure_satisfies_config_for_borrow::<NEW_C>();
        unsafe { transmute_ref(self) }
    }

    /// Borrows this `BumpScope` mutably with a new config.
    ///
    /// # Fails
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
    pub fn mut_change_config<const NEW_C: BumpConfig>(&mut self) -> &mut BumpScope<'a, A, NEW_C> {
        self.raw.ensure_satisfies_config_for_borrow_mut::<NEW_C>();
        unsafe { transmute_mut(self) }
    }

    /// Converts this `BumpScope` into a raw pointer.
    #[inline]
    #[must_use]
    pub fn into_raw(self) -> NonNull<()> {
        self.raw.into_raw()
    }

    /// Converts the raw pointer that was created with [`into_raw`](Self::into_raw) back into a `BumpScope`.
    ///
    /// # Safety
    /// This is highly unsafe, due to the number of invariants that aren't checked:
    /// - `ptr` must have been created with `Self::into_raw`.
    /// - This function must only be called once with this `ptr`.
    /// - Nothing must have been allocated since then.
    /// - The lifetime must match the original one.
    /// - The config must match the original one.
    #[inline]
    #[must_use]
    pub unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        Self {
            raw: unsafe { RawBump::from_raw(ptr) },
            marker: PhantomData,
        }
    }
}

impl<A, const C: BumpConfig> NoDrop for BumpScope<'_, A, C> {}

/// Additional `alloc` methods that are not available in traits.
impl<'a, A, const C: BumpConfig> BumpScope<'a, A, C>
where
    A: Allocator,
{
    /// Returns this bump allocator as a `&BumpScope`.
    #[must_use]
    pub fn as_scope(&self) -> &BumpScope<'_, A, C> {
        self
    }

    /// Returns this bump allocator as a `&mut BumpScope`.
    #[must_use]
    pub fn as_mut_scope(&mut self) -> &mut BumpScope<'_, A, C> {
        // SAFETY: we shorten the lifetime that allocations will have which is sound
        unsafe { transmute_mut(self) }
    }

    /// Creates a new [`BumpScopeGuard`].
    ///
    /// This allows for creation of child scopes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    ///
    /// {
    ///     let mut guard = bump.scope_guard();
    ///     let bump = guard.scope();
    ///     bump.alloc_str("Hello, world!");
    ///     assert_eq!(bump.stats().allocated(), 13);
    /// }
    ///
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    ///
    /// [claimed]: crate::BumpScope::claim
    #[inline(always)]
    pub fn scope_guard(&mut self) -> BumpScopeGuard<'_, A, C> {
        BumpScopeGuard::new(&mut self.raw)
    }

    /// Calls `f` with a new child scope.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    ///
    /// bump.scoped(|bump| {
    ///     bump.alloc_str("Hello, world!");
    ///     assert_eq!(bump.stats().allocated(), 13);
    /// });
    ///
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    ///
    /// [claimed]: crate::BumpScope::claim
    #[inline(always)]
    pub fn scoped<R>(&mut self, f: impl FnOnce(&mut BumpScope<'_, A, C>) -> R) -> R {
        let mut guard = self.scope_guard();
        f(guard.scope())
    }

    /// Calls `f` on a child scope with a new config.
    ///
    /// # Fails
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
    /// # use bump_scope::{Bump, config::{BumpConfig, BumpConfigBuilder}};
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
    /// bump.alloc(1u8);
    ///
    /// // now the bump is only aligned to 1
    /// // (if our `MIN_ALIGN` was higher, it would be that)
    /// assert!(bump.stats().bump_position().unwrap().addr().get() % 2 == 1);
    /// assert_eq!(bump.stats().allocated(), 1);
    ///
    /// bump.scoped_with_config::<CONFIG_MIN_ALIGN_8, ()>(|bump| {
    ///    // in here, the bump will have the specified minimum alignment of 8
    ///    assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///    assert_eq!(bump.stats().allocated(), 8);
    ///
    ///    // allocating a value with its size being a multiple of 8 will no longer have
    ///    // to align the bump pointer before allocation
    ///    bump.alloc(1u64);
    ///    assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///    assert_eq!(bump.stats().allocated(), 16);
    ///    
    ///    // allocating a value smaller than the minimum alignment must align the bump pointer
    ///    // after the allocation, resulting in some wasted space
    ///    bump.alloc(1u8);
    ///    assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///    assert_eq!(bump.stats().allocated(), 24);
    /// });
    ///
    /// assert_eq!(bump.stats().allocated(), 1);
    /// ```
    ///
    /// [claimed]: crate::BumpScope::claim
    #[inline(always)]
    pub fn scoped_with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<A, NEW_C>) -> R) -> R {
        RawBump::<A, C>::const_assert_upwards_matches::<NEW_C>();

        self.raw.assert_convertible_claimable::<NEW_C>();
        self.raw.assert_convertible_const_constructible::<NEW_C>();

        // The scope guard will reset the bump pointer to the current position,
        // which is aligned to the current `min_align`.
        let mut guard = self.scope_guard();
        let scope = guard.scope();
        scope.raw.align_to(config!(NEW_C, min_align));
        f(unsafe { transmute_mut(scope) })
    }

    /// Calls `f` with this scope but with a new config.
    ///
    /// # Fails
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
    /// # use bump_scope::{Bump, config::{BumpConfig, BumpConfigBuilder}};
    /// const MIN_ALIGN_8: BumpConfig = BumpConfigBuilder::new().min_align(8).build();
    ///
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_scope();
    ///
    /// // here we're allocating with a `MIN_ALIGN` of `1`
    /// let foo = bump.alloc_str("foo");
    /// assert_eq!(bump.stats().allocated(), 3);
    ///
    /// let bar = bump.with_config::<MIN_ALIGN_8, _>(|bump| {
    ///     // in here the bump position has been aligned to `8`
    ///     assert_eq!(bump.stats().allocated(), 8);
    ///     assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    ///
    ///     // make some allocations that benefit from the higher `MIN_ALIGN` of `8`
    ///     let bar = bump.alloc(0u64);
    ///     assert_eq!(bump.stats().allocated(), 16);
    ///  
    ///     // the bump position will stay aligned to `8`
    ///     bump.alloc(0u8);
    ///     assert_eq!(bump.stats().allocated(), 24);
    ///
    ///     bar
    /// });
    ///
    /// assert_eq!(bump.stats().allocated(), 24);
    ///
    /// // continue making allocations with a `MIN_ALIGN` of `1`
    /// let baz = bump.alloc_str("baz");
    /// assert_eq!(bump.stats().allocated(), 24 + 3);
    ///
    /// dbg!(foo, bar, baz);
    /// ```
    ///
    /// Decrease the minimum alignment:
    /// ```
    /// # use std::alloc::Global;
    /// # use bump_scope::{Bump, config::{BumpConfig, BumpConfigBuilder}};
    /// const MIN_ALIGN_8: BumpConfig = BumpConfigBuilder::new().min_align(8).build();
    /// const MIN_ALIGN_1: BumpConfig = BumpConfigBuilder::new().build();
    ///
    /// let mut bump: Bump<Global, MIN_ALIGN_8> = Bump::new();
    /// let bump = bump.as_mut_scope();
    ///
    /// // make some allocations that benefit from the `MIN_ALIGN` of `8`
    /// let foo = bump.alloc(0u64);
    ///
    /// let bar = bump.with_config::<MIN_ALIGN_1, _>(|bump| {
    ///     // make some allocations that benefit from the lower `MIN_ALIGN` of `1`
    ///     let bar = bump.alloc(0u8);
    ///
    ///     // the bump position will not get aligned to `8` in here
    ///     assert_eq!(bump.stats().allocated(), 8 + 1);
    ///
    ///     bar
    /// });
    ///
    /// // after `with_config()`, the bump position will be aligned to `8` again
    /// // to satisfy our `MIN_ALIGN`
    /// assert!(bump.stats().bump_position().unwrap().addr().get() % 8 == 0);
    /// assert_eq!(bump.stats().allocated(), 16);
    ///
    /// // continue making allocations that benefit from the `MIN_ALIGN` of `8`
    /// let baz = bump.alloc(0u64);
    ///
    /// dbg!(foo, bar, baz);
    /// ```
    #[inline(always)]
    pub fn with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<'a, A, NEW_C>) -> R) -> R {
        RawBump::<A, C>::const_assert_upwards_matches::<NEW_C>();

        const {
            assert!(
                config!(NEW_C, claimable) == config!(claimable),
                "can't change `claimable` using `Bump(Scope)::with_config`"
            );

            assert!(
                config!(NEW_C, const_constructible) == config!(const_constructible),
                "can't change `const_constructible` using `Bump(Scope)::with_config`"
            );
        }

        if config!(NEW_C, min_align) < config!(min_align) {
            let guard = BumpAlignGuard::new(self);

            // SAFETY: bump is already aligned to the new alignment and the guard will ensure
            // that the bump pointer will again be aligned to old alignment once it drops
            let bump = unsafe { transmute_mut(guard.scope) };

            f(bump)
        } else {
            self.raw.align_to(config!(NEW_C, min_align));

            // SAFETY: we aligned the bump pointer
            let bump = unsafe { transmute_mut(self) };

            f(bump)
        }
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
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpVec, bump_vec};
    ///
    /// let bump: Bump = Bump::new();
    /// let vec1: BumpVec<u8, _> = bump_vec![in &bump; 1, 2, 3];
    /// let vec2: BumpVec<u8, _> = bump_vec![in &bump; 4, 5, 6];
    ///
    /// bump.claim().scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    ///
    /// assert_eq!(vec1, [1, 2, 3]);
    /// assert_eq!(vec2, [4, 5, 6]);
    /// ```
    pub fn claim(&self) -> BumpClaimGuard<'_, 'a, A, C> {
        BumpClaimGuard::new(self)
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    pub fn allocator(&self) -> Option<&'a A> {
        self.raw.allocator()
    }
}

unsafe impl<A, const C: BumpConfig> Allocator for BumpScope<'_, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        allocator_impl::allocate(&self.raw, layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { allocator_impl::deallocate(&self.raw, ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::grow(&self.raw, ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::grow_zeroed(&self.raw, ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { allocator_impl::shrink(&self.raw, ptr, old_layout, new_layout) }
    }
}
