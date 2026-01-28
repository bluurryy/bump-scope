use core::{
    ffi::CStr,
    fmt::{self, Debug},
    marker::PhantomData,
    mem::MaybeUninit,
    num::NonZeroUsize,
    panic::{RefUnwindSafe, UnwindSafe},
    ptr,
};

#[cfg(feature = "alloc")]
use core::ptr::NonNull;

#[cfg(feature = "nightly-clone-to-uninit")]
use core::clone::CloneToUninit;

use crate::{
    Bump, BumpBox, BumpClaimGuard, BumpScopeGuard, Checkpoint, ErrorBehavior, NoDrop, SizedTypeProperties,
    alloc::{AllocError, Allocator},
    down_align_usize, maybe_default_allocator,
    owned_slice::OwnedSlice,
    polyfill::{non_null, transmute_mut, transmute_ref, transmute_value},
    raw_bump::RawBump,
    settings::{BumpAllocatorSettings, BumpSettings, MinimumAlignment, SupportedMinimumAlignment},
    stats::{AnyStats, Stats},
    traits::{
        self, BumpAllocator, BumpAllocatorCore, BumpAllocatorScope, BumpAllocatorTyped, BumpAllocatorTypedScope,
        MutBumpAllocatorTypedScope,
    },
    up_align_usize_unchecked,
};

#[cfg(feature = "alloc")]
use crate::alloc::Global;

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

macro_rules! make_type {
    ($($allocator_parameter:tt)*) => {
        /// A bump allocation scope.
        ///
        /// A `BumpScope`'s allocations are live for `'a`, which is the lifetime of its associated `BumpScopeGuard(Root)` or `scoped` closure.
        ///
        /// `BumpScope` has mostly same api as [`Bump`].
        ///
        /// This type is provided as a parameter to the closure of [`scoped`], or created
        /// by [`BumpScopeGuard::scope`]. A [`Bump`] can also be turned into a `BumpScope` using
        /// [`as_scope`], [`as_mut_scope`] or `from` / `into`.
        ///
        /// [`scoped`]: crate::traits::BumpAllocator::scoped
        /// [`BumpScopeGuard::scope`]: crate::BumpScopeGuard::scope
        /// [`Bump`]: crate::Bump
        /// [`as_scope`]: crate::Bump::as_scope
        /// [`as_mut_scope`]: crate::Bump::as_mut_scope
        /// [`reset`]: crate::Bump::reset
        #[repr(transparent)]
        pub struct BumpScope<'a, $($allocator_parameter)*, S = BumpSettings>
        where
            S: BumpAllocatorSettings,
        {
            pub(crate) raw: RawBump<A, S>,

            /// Marks the lifetime of the mutably borrowed `BumpScopeGuard`.
            pub(crate) marker: PhantomData<&'a ()>,
        }
    };
}

maybe_default_allocator!(make_type);

impl<A, S> UnwindSafe for BumpScope<'_, A, S>
where
    A: UnwindSafe,
    S: BumpAllocatorSettings,
{
}

impl<A, S> RefUnwindSafe for BumpScope<'_, A, S>
where
    A: UnwindSafe,
    S: BumpAllocatorSettings,
{
}

impl<A, S> Debug for BumpScope<'_, A, S>
where
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(self.stats()).debug_format("BumpScope", f)
    }
}

impl<A, S> BumpScope<'_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    /// Returns this `&BumpScope` as a `&Bump`.
    ///
    /// This method exists so you can have `&Bump` function parameters and
    /// struct fields instead of `&BumpScope<'a>` that can still accept `BumpScope`'s.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// fn my_fn(bump: &Bump) {
    ///    // ...
    ///    # _ = bump;
    /// }
    ///
    /// let mut bump: Bump = Bump::new();
    ///
    /// bump.scoped(|bump_scope| {
    ///     my_fn(bump_scope.as_bump());
    /// });
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn as_bump(&self) -> &Bump<A, S> {
        unsafe { transmute_ref(self) }
    }

    /// Returns this `&mut BumpScope` as a `BumpScope`.
    ///
    /// This requires allocating a chunk if none has been allocated yet.
    ///
    /// This method exists so you can have `BumpScope<'a>` function parameters and
    /// struct fields instead of `&'b mut BumpScope<'a>` so you don't have to deal with `'b`.
    ///
    /// It also enables more settings conversions since [`with_settings`] can do more than
    /// [`borrow_mut_with_settings`].
    ///
    /// # Panics
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// Panics if the allocation fails.
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    /// [`with_settings`]: BumpScope::with_settings
    /// [`borrow_mut_with_settings`]: BumpScope::borrow_mut_with_settings
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn by_value(&mut self) -> BumpScope<'_, A, S> {
        panic_on_error(self.raw.make_allocated());

        BumpScope {
            raw: self.raw.clone(),
            marker: PhantomData,
        }
    }

    /// Returns this `&mut BumpScope` as a `BumpScope`.
    ///
    /// This requires allocating a chunk if none has been allocated yet.
    ///
    /// This method exists so you can have `BumpScope<'a>` function parameters and
    /// struct fields instead of `&'b mut BumpScope<'a>` so you don't have to deal with `'b`.
    ///
    /// It also enables more settings conversions since [`with_settings`] can do more than
    /// [`borrow_mut_with_settings`].
    ///
    /// # Errors
    /// Errors if the bump allocator is currently [claimed].
    ///
    /// Errors if the allocation fails.
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    /// [`with_settings`]: BumpScope::with_settings
    /// [`borrow_mut_with_settings`]: BumpScope::borrow_mut_with_settings
    #[inline(always)]
    pub fn try_by_value(&mut self) -> Result<BumpScope<'_, A, S>, AllocError> {
        self.raw.make_allocated::<AllocError>()?;

        Ok(BumpScope {
            raw: self.raw.clone(),
            marker: PhantomData,
        })
    }
}

impl<'a, A, S> BumpScope<'a, A, S>
where
    S: BumpAllocatorSettings,
{
    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'a, S> {
        self.raw.stats()
    }

    #[inline(always)]
    pub(crate) fn align<const ALIGN: usize>(&self)
    where
        MinimumAlignment<ALIGN>: SupportedMinimumAlignment,
    {
        self.raw.align::<ALIGN>();
    }

    #[inline(always)]
    pub(crate) unsafe fn cast<S2>(self) -> BumpScope<'a, A, S2>
    where
        S2: BumpAllocatorSettings,
    {
        unsafe { transmute_value(self) }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_mut<S2>(&mut self) -> &mut BumpScope<'a, A, S2>
    where
        S2: BumpAllocatorSettings,
    {
        unsafe { &mut *ptr::from_mut(self).cast::<BumpScope<'a, A, S2>>() }
    }

    /// Will error at compile time if `NEW_MIN_ALIGN < MIN_ALIGN`.
    #[inline(always)]
    pub(crate) fn must_align_more<const NEW_MIN_ALIGN: usize>(&self)
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        const {
            assert!(
                NEW_MIN_ALIGN >= S::MIN_ALIGN,
                "`into_aligned` or `as_mut_aligned` can't decrease the minimum alignment"
            );
        }

        self.align::<NEW_MIN_ALIGN>();
    }

    /// Mutably borrows `BumpScope` with a new minimum alignment.
    ///
    /// **This cannot decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment.
    ///
    /// When decreasing the alignment we need to make sure that the bump position is realigned to the original alignment.
    /// That can only be ensured by having a function that takes a closure, like the methods mentioned above do.
    #[inline(always)]
    pub fn as_mut_aligned<const NEW_MIN_ALIGN: usize>(
        &mut self,
    ) -> &mut BumpScope<'a, A, S::WithMinimumAlignment<NEW_MIN_ALIGN>>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_mut() }
    }

    /// Converts this `BumpScope` into a `BumpScope` with a new minimum alignment.
    ///
    /// **This cannot decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment.
    ///
    /// When decreasing the alignment we need to make sure that the bump position is realigned to the original alignment.
    /// That can only be ensured by having a function that takes a closure, like the methods mentioned above do.
    ///
    /// If this was allowed to decrease the alignment it would break minimum alignment:
    ///
    /// ```ignore
    /// # // We can't `compile_fail,E0080` this doc test because it does not do the compile step
    /// # // that triggers the error.
    /// # use bump_scope::{Bump, alloc::Global};
    /// let mut bump: Bump<Global, 8, true> = Bump::new();
    /// let mut guard = bump.scope_guard();
    ///
    /// {
    ///     let scope = guard.scope().into_aligned::<1>();
    ///     scope.alloc(0u8);
    /// }
    ///
    /// {
    ///     let scope = guard.scope();
    ///     // scope is not aligned to `MIN_ALIGN`!!
    /// }
    ///
    /// ```
    #[inline(always)]
    pub fn into_aligned<const NEW_MIN_ALIGN: usize>(self) -> BumpScope<'a, A, S::WithMinimumAlignment<NEW_MIN_ALIGN>>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast() }
    }

    /// Sets the bump position and aligns it to the required `MIN_ALIGN`.
    ///
    /// This does nothing if the current chunk is the UNALLOCATED one.
    #[inline(always)]
    pub(crate) unsafe fn set_pos(&self, pos: NonZeroUsize) {
        unsafe { self.raw.set_pos(pos) }
    }

    /// A version of [`set_pos`](Self::set_pos) that only aligns the pointer
    /// if it the `pos_align` is smaller than the `MIN_ALIGN`.
    ///
    /// This should only be called when the `pos_align` is statically known so
    /// the branch gets optimized out.
    ///
    /// This does nothing if the current chunk is the UNALLOCATED one.
    #[inline(always)]
    pub(crate) unsafe fn set_aligned_pos(&self, pos: NonZeroUsize, pos_align: usize) {
        unsafe { self.raw.set_aligned_pos(pos, pos_align) };
    }

    /// Converts this `BumpScope` into a `BumpScope` with new settings.
    ///
    /// Not all settings can be converted to. This function will fail to compile if:
    /// - `NewS::UP != S::UP`
    /// - `NewS::GUARANTEED_ALLOCATED > S::GUARANTEED_ALLOCATED`
    /// - `NewS::CLAIMABLE < S::CLAIMABLE`
    #[inline]
    pub fn with_settings<NewS>(self) -> BumpScope<'a, A, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        self.raw.ensure_satisfies_settings::<NewS>();
        unsafe { transmute_value(self) }
    }

    /// Borrows this `BumpScope` with new settings.
    ///
    /// Not all settings can be converted to. This function will fail to compile if:
    /// - `NewS::MIN_ALIGN != S::MIN_ALIGN`
    /// - `NewS::UP != S::UP`
    /// - `NewS::GUARANTEED_ALLOCATED > S::GUARANTEED_ALLOCATED`
    /// - `NewS::CLAIMABLE != S::CLAIMABLE`
    #[inline]
    pub fn borrow_with_settings<NewS>(&self) -> &BumpScope<'a, A, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        self.raw.ensure_satisfies_settings_for_borrow::<NewS>();
        unsafe { transmute_ref(self) }
    }

    /// Borrows this `BumpScope` mutably with new settings.
    ///
    /// Not all settings can be converted to. This function will fail to compile if:
    /// - `NewS::MIN_ALIGN < S::MIN_ALIGN`
    /// - `NewS::UP != S::UP`
    /// - `NewS::GUARANTEED_ALLOCATED != S::GUARANTEED_ALLOCATED`
    /// - `NewS::CLAIMABLE != S::CLAIMABLE`
    #[inline]
    pub fn borrow_mut_with_settings<NewS>(&mut self) -> &mut BumpScope<'a, A, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        self.raw.ensure_satisfies_settings_for_borrow_mut::<NewS>();
        unsafe { transmute_mut(self) }
    }
}

#[cfg(feature = "alloc")]
impl<S> BumpScope<'_, Global, S>
where
    S: BumpAllocatorSettings,
{
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
    /// - The settings must match the original ones.
    #[inline]
    #[must_use]
    pub unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        Self {
            raw: unsafe { RawBump::from_raw(ptr) },
            marker: PhantomData,
        }
    }
}

impl<A, S> NoDrop for BumpScope<'_, A, S> where S: BumpAllocatorSettings {}

/// Methods that forward to traits.
// Documentation is in the forwarded to methods.
#[allow(clippy::missing_errors_doc, clippy::missing_safety_doc)]
impl<'a, A, S> BumpScope<'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    traits::forward_methods! {
        self: self
        access: {self}
        access_mut: {self}
        lifetime: 'a
    }
}

/// Additional `alloc` methods that are not available in traits.
impl<'a, A, S> BumpScope<'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// There is also [`alloc_try_with_mut`](Self::alloc_try_with_mut), optimized for a mutable reference.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    pub fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpBox<'a, T>, E> {
        panic_on_error(self.generic_alloc_try_with(f))
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// There is also [`try_alloc_try_with_mut`](Self::try_alloc_try_with_mut), optimized for a mutable reference.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_try_with<T, E>(
        &self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpBox<'a, T>, E>, AllocError> {
        self.generic_alloc_try_with(f)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_try_with<B: ErrorBehavior, T, E>(
        &self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpBox<'a, T>, E>, B> {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(BumpBox::zst(value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let checkpoint_before_alloc = self.checkpoint();
        let uninit = self.generic_alloc_uninit::<B, Result<T, E>>()?;
        let ptr = BumpBox::into_raw(uninit).cast::<Result<T, E>>();

        // When bumping downwards the chunk's position is the same as `ptr`.
        // Using `ptr` is faster so we use that.
        let pos = if S::UP { self.raw.chunk.get().pos() } else { ptr.cast() };

        Ok(unsafe {
            non_null::write_with(ptr, f);

            // If `f` made allocations on this bump allocator we can't shrink the allocation.
            let can_shrink = pos == self.raw.chunk.get().pos();

            match non_null::result(ptr) {
                Ok(value) => Ok({
                    if can_shrink {
                        let new_pos = if S::UP {
                            let pos = value.add(1).addr().get();
                            up_align_usize_unchecked(pos, S::MIN_ALIGN)
                        } else {
                            let pos = value.addr().get();
                            down_align_usize(pos, S::MIN_ALIGN)
                        };

                        // The allocation of was successful, so our chunk must be allocated.
                        let chunk = self.raw.chunk.get().as_non_dummy_unchecked();
                        chunk.set_pos_addr(new_pos);
                    }

                    BumpBox::from_raw(value)
                }),
                Err(error) => Err({
                    let error = error.read();

                    if can_shrink {
                        self.reset_to(checkpoint_before_alloc);
                    }

                    error
                }),
            }
        })
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// This is just like [`alloc_try_with`](Self::alloc_try_with), but optimized for a mutable reference.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with_mut(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let result = bump.alloc_try_with_mut(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    pub fn alloc_try_with_mut<T, E>(&mut self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpBox<'a, T>, E> {
        panic_on_error(self.generic_alloc_try_with_mut(f))
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// This is just like [`try_alloc_try_with`](Self::try_alloc_try_with), but optimized for a mutable reference.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(offset_of_enum)]
    /// # use core::mem::offset_of;
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_try_with_mut<T, E>(
        &mut self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpBox<'a, T>, E>, AllocError> {
        self.generic_alloc_try_with_mut(f)
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_try_with_mut<B: ErrorBehavior, T, E>(
        &mut self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpBox<'a, T>, E>, B> {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(BumpBox::zst(value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let checkpoint = self.checkpoint();
        let ptr = self.raw.prepare_sized_allocation::<B, Result<T, E>>()?;

        Ok(unsafe {
            non_null::write_with(ptr, f);

            // There is no need for `can_shrink` checks, because we have a mutable reference
            // so there's no way anyone else has allocated in `f`.
            match non_null::result(ptr) {
                Ok(value) => Ok({
                    let new_pos = if S::UP {
                        let pos = value.add(1).addr().get();
                        up_align_usize_unchecked(pos, S::MIN_ALIGN)
                    } else {
                        let pos = value.addr().get();
                        down_align_usize(pos, S::MIN_ALIGN)
                    };

                    // The allocation was successful, so our chunk must be allocated.
                    let chunk = self.raw.chunk.get().as_non_dummy_unchecked();
                    chunk.set_pos_addr(new_pos);

                    BumpBox::from_raw(value)
                }),
                Err(error) => Err({
                    let error = error.read();
                    self.reset_to(checkpoint);
                    error
                }),
            }
        })
    }

    #[inline(always)]
    pub(crate) fn generic_alloc_uninit<B: ErrorBehavior, T>(&self) -> Result<BumpBox<'a, MaybeUninit<T>>, B> {
        if T::IS_ZST {
            return Ok(BumpBox::zst(MaybeUninit::uninit()));
        }

        let ptr = self.raw.alloc_sized::<B, T>()?.cast::<MaybeUninit<T>>();
        unsafe { Ok(BumpBox::from_raw(ptr)) }
    }
}
