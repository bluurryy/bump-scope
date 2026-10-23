use core::{
    alloc::{AllocError, Allocator, Layout},
    ffi::CStr,
    fmt::{self, Debug},
    marker::PhantomData,
    mem::MaybeUninit,
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::{self, NonNull},
};

#[cfg(feature = "nightly-clone-to-uninit")]
use core::clone::CloneToUninit;

use crate::{
    BoxU, BumpAllocation, BumpClaimGuard, BumpScopeGuard, ErrorBehavior, NoDrop, SizedTypeProperties, Unmanaged, VecU,
    allocator_impl,
    bump_align_guard::BumpAlignGuard,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG, config},
    down_align_usize,
    extra::{
        box_from_utf8_unchecked, box_init, box_init_clone, box_init_fill, box_init_fill_with, boxu, boxu_uninit_zst,
        boxu_uninit_zst_slice,
    },
    maybe_default_allocator,
    owned_slice::{OwnedSlice, TakeOwnedSlice as _},
    polyfill::{transmute_mut, transmute_ref, transmute_value},
    raw_bump::RawBump,
    stats::{AnyStats, Stats},
    string::String,
    traits::{BumpAllocator, Sealed},
    up_align_usize_unchecked, util,
};

#[cfg(feature = "alloc")]
use alloc::alloc::Global;
use alloc::vec::Vec;

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

macro_rules! make_type {
    ($($allocator_parameter:tt)*) => {
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
        pub struct BumpScope<'a, $($allocator_parameter)*, const C: BumpConfig = DEFAULT_BUMP_CONFIG>
        where

        {
            pub(crate) raw: RawBump<A, C>,

            /// Marks the lifetime of the mutably borrowed `BumpScopeGuard`.
            pub(crate) marker: PhantomData<&'a ()>,
        }
    };
}

maybe_default_allocator!(make_type);

impl<A, const C: BumpConfig> UnwindSafe for BumpScope<'_, A, C> where A: RefUnwindSafe {}

impl<A, const C: BumpConfig> RefUnwindSafe for BumpScope<'_, A, C> where A: RefUnwindSafe {}

impl<A, const C: BumpConfig> Debug for BumpScope<'_, A, C> {
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
    /// [`self.is_claimed()`]: Self::is_claimed
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
}

#[cfg(feature = "alloc")]
impl<const C: BumpConfig> BumpScope<'_, Global, C> {
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

/// `alloc` methods.
#[allow(clippy::missing_errors_doc, clippy::missing_safety_doc)]
impl<'a, A, const C: BumpConfig> BumpScope<'a, A, C>
where
    A: Allocator,
{
    /// Allocate an object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc(123);
    /// assert_eq!(allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc<T>(&self, value: T) -> BumpAllocation<T, &Self> {
        let boxu = panic_on_error(self.imp_alloc(value));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate an object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc(123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc<T>(&self, value: T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxu = self.imp_alloc(value)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocates space for an object, then calls `f` to produce the
    /// value to be put in that place.
    ///
    /// In some cases this could be more performant than `alloc(f())` because it
    /// permits the compiler to directly place `T` in the allocated memory instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_with(|| 123);
    /// assert_eq!(allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> BumpAllocation<T, &Self> {
        let boxu = panic_on_error(self.imp_alloc_with(f));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocates space for an object, then calls `f` to produce the
    /// value to be put in that place.
    ///
    /// In some cases this could be more performant than `try_alloc(f())` because it
    /// permits the compiler to directly place `T` in the allocated memory instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_with(|| 123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxu = self.imp_alloc_with(f)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[alloc_with](crate::traits::BumpAllocatorScope::alloc_with)(T::default)</code>.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_default::<i32>();
    /// assert_eq!(allocated, 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_default<T: Default>(&self) -> BumpAllocation<T, &Self> {
        let boxu = panic_on_error(self.imp_alloc_default());
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[try_alloc_with](crate::traits::BumpAllocatorScope::try_alloc_with)(T::default)</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_default()?;
    /// assert_eq!(allocated, 0);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_default<T: Default>(&self) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxu = self.imp_alloc_default()?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate an object by cloning it.
    ///
    /// Unlike `alloc(value.clone())` this method also works for dynamically-sized types.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// Allocate a `slice`, `str`, `CStr`, `Path`:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    ///
    /// let cloned = bump.alloc_clone(&[1, 2, 3]);
    /// assert_eq!(cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.alloc_clone("foo");
    /// assert_eq!(cloned, "foo");
    ///
    /// let cloned = bump.alloc_clone(c"foo");
    /// assert_eq!(cloned, c"foo");
    ///
    /// let cloned = bump.alloc_clone(Path::new("foo"));
    /// assert_eq!(cloned, Path::new("foo"));
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// # use bump_scope::Bump;
    ///
    /// trait FnClone: Fn() -> String + CloneToUninit {}
    /// impl<T: ?Sized + Fn() -> String + CloneToUninit> FnClone for T {}
    ///
    /// // the closure references a local variable
    /// let reference = &String::from("Hello,");
    ///
    /// // and owns a string that it will have to clone
    /// let value = String::from("world!");
    ///
    /// let closure = move || format!("{reference} {value}");
    /// let object: &dyn FnClone = &closure;
    ///
    /// assert_eq!(object(), "Hello, world!");
    ///
    /// let bump: Bump = Bump::new();
    /// let object_clone = bump.alloc_clone(object);
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    #[cfg(feature = "nightly-clone-to-uninit")]
    pub fn alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> BumpAllocation<T, &Self> {
        let boxu = panic_on_error(self.imp_alloc_clone(value));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate an object by cloning it.
    ///
    /// Unlike `alloc(value.clone())` this method also works for dynamically-sized types.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    ///
    /// Allocate a `slice`, `str`, `CStr`, `Path`:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    ///
    /// let cloned = bump.try_alloc_clone(&[1, 2, 3])?;
    /// assert_eq!(cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.try_alloc_clone("foo")?;
    /// assert_eq!(cloned, "foo");
    ///
    /// let cloned = bump.try_alloc_clone(c"foo")?;
    /// assert_eq!(cloned, c"foo");
    ///
    /// let cloned = bump.try_alloc_clone(Path::new("foo"))?;
    /// assert_eq!(cloned, Path::new("foo"));
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// # use bump_scope::Bump;
    ///
    /// trait FnClone: Fn() -> String + CloneToUninit {}
    /// impl<T: ?Sized + Fn() -> String + CloneToUninit> FnClone for T {}
    ///
    /// // the closure references a local variable
    /// let reference = &String::from("Hello,");
    ///
    /// // and owns a string that it will have to clone
    /// let value = String::from("world!");
    ///
    /// let closure = move || format!("{reference} {value}");
    /// let object: &dyn FnClone = &closure;
    ///
    /// assert_eq!(object(), "Hello, world!");
    ///
    /// let bump: Bump = Bump::new();
    /// let object_clone = bump.try_alloc_clone(object)?;
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[cfg(feature = "nightly-clone-to-uninit")]
    pub fn try_alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxu = self.imp_alloc_clone(value)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`init`](BoxU::init),
    /// `init_zeroed` <sup>[\[1\]](crate::zerocopy_08::InitZeroed::init_zeroed) [\[2\]](crate::bytemuck::InitZeroed::init_zeroed)</sup>
    /// or unsafely with [`assume_init`](BoxU::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit();
    ///
    /// let five = uninit.init(5);
    ///
    /// assert_eq!(*five, 5)
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.alloc_uninit();
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_uninit<T>(&self) -> BumpAllocation<MaybeUninit<T>, &Self> {
        let boxu = panic_on_error(self.imp_alloc_uninit());
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`init`](BoxU::init),
    /// `init_zeroed` <sup>[\[1\]](crate::zerocopy_08::InitZeroed::init_zeroed) [\[2\]](crate::bytemuck::InitZeroed::init_zeroed)</sup>
    /// or unsafely with [`assume_init`](BoxU::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.try_alloc_uninit()?;
    ///
    /// let five = uninit.init(5);
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.try_alloc_uninit()?;
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_uninit<T>(&self) -> Result<BumpAllocation<MaybeUninit<T>, &Self>, AllocError> {
        let boxu = self.imp_alloc_uninit()?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// // by value
    /// let a = bump.alloc_slice_move([1, 2]);
    /// let b = bump.alloc_slice_move(vec![3, 4]);
    /// let c = bump.alloc_slice_move(bump.alloc_iter(5..=6));
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.alloc_slice_move(&mut other);
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(a, [1, 2]);
    /// assert_eq!(b, [3, 4]);
    /// assert_eq!(c, [5, 6]);
    /// assert_eq!(d, [7, 8]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.imp_alloc_slice_move(slice));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// // by value
    /// let a = bump.try_alloc_slice_move([1, 2])?;
    /// let b = bump.try_alloc_slice_move(vec![3, 4])?;
    /// let c = bump.try_alloc_slice_move(bump.alloc_iter(5..=6))?;
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.try_alloc_slice_move(&mut other)?;
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(a, [1, 2]);
    /// assert_eq!(b, [3, 4]);
    /// assert_eq!(c, [5, 6]);
    /// assert_eq!(d, [7, 8]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_move<T>(
        &self,
        slice: impl OwnedSlice<Item = T>,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.imp_alloc_slice_move(slice)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_copy(&[1, 2, 3]);
    /// assert_eq!(allocated, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.imp_alloc_slice_copy(slice));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_slice_copy(&[1, 2, 3])?;
    /// assert_eq!(allocated, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.imp_alloc_slice_copy(slice)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_clone(&[String::from("a"), String::from("b")]);
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.imp_alloc_slice_clone(slice));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_slice_clone(&[String::from("a"), String::from("b")])?;
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.imp_alloc_slice_clone(slice)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill(3, "ho");
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.imp_alloc_slice_fill(len, value));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_slice_fill(3, "ho")?;
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.imp_alloc_slice_fill(len, value)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`alloc_slice_fill`](crate::traits::BumpAllocatorScope::alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill_with::<i32>(3, Default::default);
    /// assert_eq!(allocated, [0, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.imp_alloc_slice_fill_with(len, f));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`try_alloc_slice_fill`](crate::traits::BumpAllocatorScope::try_alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_slice_fill_with::<i32>(3, Default::default)?;
    /// assert_eq!(allocated, [0, 0, 0]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_fill_with<T>(
        &self,
        len: usize,
        f: impl FnMut() -> T,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.imp_alloc_slice_fill_with(len, f)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BoxU::init_fill),
    /// [`init_fill_with`](BoxU::init_fill_with),
    /// [`init_copy`](BoxU::init_copy),
    /// [`init_clone`](BoxU::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BoxU::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit_slice(3);
    ///
    /// let values = uninit.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(values, [1, 2, 3])
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.alloc_uninit_slice(3);
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_uninit_slice<T>(&self, len: usize) -> BumpAllocation<[MaybeUninit<T>], &Self> {
        let boxu = panic_on_error(self.imp_alloc_uninit_slice(len));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BoxU::init_fill),
    /// [`init_fill_with`](BoxU::init_fill_with),
    /// [`init_copy`](BoxU::init_copy),
    /// [`init_clone`](BoxU::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BoxU::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.try_alloc_uninit_slice(3)?;
    ///
    /// let values = uninit.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.try_alloc_uninit_slice(3)?;
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_uninit_slice<T>(&self, len: usize) -> Result<BumpAllocation<[MaybeUninit<T>], &Self>, AllocError> {
        let boxu = self.imp_alloc_uninit_slice(len)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BoxU::init_fill),
    /// [`init_fill_with`](BoxU::init_fill_with),
    /// [`init_copy`](BoxU::init_copy),
    /// [`init_clone`](BoxU::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BoxU::assume_init).
    ///
    /// This is just like [`alloc_uninit_slice`](crate::traits::BumpAllocatorScope::alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = &[1, 2, 3];
    /// let uninit_slice = bump.alloc_uninit_slice_for(slice);
    /// assert_eq!(uninit_slice.len(), 3);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_uninit_slice_for<T>(&self, slice: &[T]) -> BumpAllocation<[MaybeUninit<T>], &Self> {
        let boxu = panic_on_error(self.imp_alloc_uninit_slice_for(slice));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BoxU::init_fill),
    /// [`init_fill_with`](BoxU::init_fill_with),
    /// [`init_copy`](BoxU::init_copy),
    /// [`init_clone`](BoxU::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BoxU::assume_init).
    ///
    /// This is just like [`try_alloc_uninit_slice`](crate::traits::BumpAllocatorScope::try_alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = &[1, 2, 3];
    /// let uninit_slice = bump.try_alloc_uninit_slice_for(slice)?;
    /// assert_eq!(uninit_slice.len(), 3);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_uninit_slice_for<T>(&self, slice: &[T]) -> Result<BumpAllocation<[MaybeUninit<T>], &Self>, AllocError> {
        let boxu = self.imp_alloc_uninit_slice_for(slice)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate a `str`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_str("Hello, world!");
    /// assert_eq!(allocated, "Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_str(&self, src: &str) -> BumpAllocation<str, &Self> {
        let boxu = panic_on_error(self.imp_alloc_str(src));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate a `str`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_str("Hello, world!")?;
    /// assert_eq!(allocated, "Hello, world!");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_str(&self, src: &str) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxu = self.imp_alloc_str(src)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`alloc_fmt_mut`](crate::traits::MutBumpAllocatorScope::alloc_fmt_mut)
    /// instead for better performance.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fmt(&self, args: fmt::Arguments) -> BumpAllocation<str, &Self> {
        let boxu = panic_on_error(self.imp_alloc_fmt(args));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_fmt_mut`](crate::traits::MutBumpAllocatorScope::try_alloc_fmt_mut)
    /// instead for better performance.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_fmt(&self, args: fmt::Arguments) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxu = self.imp_alloc_fmt(args)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate a `CStr`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr(c"Hello, world!");
    /// assert_eq!(allocated, c"Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr(&self, src: &CStr) -> &'a CStr {
        panic_on_error(self.imp_alloc_cstr(src))
    }

    /// Allocate a `CStr`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_cstr(c"Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr(&self, src: &CStr) -> Result<&'a CStr, AllocError> {
        self.imp_alloc_cstr(src)
    }

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr_from_str("Hello, world!");
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.alloc_cstr_from_str("abc\0def");
    /// assert_eq!(allocated, c"abc");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_from_str(&self, src: &str) -> &'a CStr {
        panic_on_error(self.imp_alloc_cstr_from_str(src))
    }

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_cstr_from_str("Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.try_alloc_cstr_from_str("abc\0def")?;
    /// assert_eq!(allocated, c"abc");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr_from_str(&self, src: &str) -> Result<&'a CStr, AllocError> {
        self.imp_alloc_cstr_from_str(src)
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`alloc_cstr_fmt_mut`](crate::traits::MutBumpAllocatorScope::alloc_cstr_fmt_mut)
    /// instead for better performance.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_fmt(&self, args: fmt::Arguments) -> &'a CStr {
        panic_on_error(self.imp_alloc_cstr_fmt(args))
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_cstr_fmt_mut`](crate::traits::MutBumpAllocatorScope::try_alloc_cstr_fmt_mut)
    /// instead for better performance.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr_fmt(&self, args: fmt::Arguments) -> Result<&'a CStr, AllocError> {
        self.imp_alloc_cstr_fmt(args)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`alloc_iter_mut`].
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    ///
    /// [`alloc_iter_exact`]: crate::traits::BumpAllocatorScope::alloc_iter_exact
    /// [`alloc_iter_mut`]: crate::traits::MutBumpAllocatorScope::alloc_iter_mut
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.imp_alloc_iter(iter));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`try_alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`try_alloc_iter_mut`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.try_alloc_iter([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_iter_exact`]: crate::traits::BumpAllocatorScope::try_alloc_iter_exact
    /// [`try_alloc_iter_mut`]: crate::traits::MutBumpAllocatorScope::try_alloc_iter_mut
    #[inline(always)]
    pub fn try_alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.imp_alloc_iter(iter)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

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
    pub fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpAllocation<T, &Self>, E> {
        match panic_on_error(self.imp_alloc_try_with(f)) {
            Ok(boxu) => Ok(BumpAllocation::wrap(boxu, self)),
            Err(err) => Err(err),
        }
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
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_try_with<T, E>(
        &self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpAllocation<T, &Self>, E>, AllocError> {
        Ok(match self.imp_alloc_try_with(f)? {
            Ok(boxu) => Ok(BumpAllocation::wrap(boxu, self)),
            Err(err) => Err(err),
        })
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`alloc_iter`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    ///
    /// [`alloc_iter`]: crate::traits::BumpAllocatorScope::alloc_iter
    /// [`alloc_iter_mut_rev`]: crate::traits::MutBumpAllocatorScope::alloc_iter_mut_rev
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter_mut<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.imp_alloc_iter_mut(iter));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_iter`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// When bumping downwards, prefer [`try_alloc_iter_mut_rev`] instead.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.try_alloc_iter_mut([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_iter`]: crate::traits::BumpAllocatorScope::alloc_iter
    /// [`try_alloc_iter_mut_rev`]: crate::traits::MutBumpAllocatorScope::alloc_iter_mut_rev
    #[inline(always)]
    pub fn try_alloc_iter_mut<T>(
        &mut self,
        iter: impl IntoIterator<Item = T>,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.imp_alloc_iter_mut(iter)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`alloc_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    ///
    /// [`alloc_fmt`]: crate::traits::BumpAllocatorScope::alloc_fmt
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fmt_mut(&mut self, args: fmt::Arguments) -> BumpAllocation<str, &Self> {
        let boxu = panic_on_error(self.imp_alloc_fmt_mut(args));
        BumpAllocation::wrap(boxu, self)
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_fmt`]: crate::traits::BumpAllocatorScope::try_alloc_fmt
    #[inline(always)]
    pub fn try_alloc_fmt_mut(&mut self, args: fmt::Arguments) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxu = self.imp_alloc_fmt_mut(args)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`alloc_cstr_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt_mut(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    ///
    /// [`alloc_cstr_fmt`]: crate::traits::BumpAllocatorScope::alloc_cstr_fmt
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> &'a CStr {
        panic_on_error(self.imp_alloc_cstr_fmt_mut(args))
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_cstr_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt_mut(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_cstr_fmt`]: crate::traits::BumpAllocatorScope::try_alloc_cstr_fmt
    #[inline(always)]
    pub fn try_alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> Result<&'a CStr, AllocError> {
        self.imp_alloc_cstr_fmt_mut(args)
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
    pub fn alloc_try_with_mut<T, E>(&mut self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpAllocation<T, &Self>, E> {
        let boxu = panic_on_error(self.imp_alloc_try_with_mut(f))?;
        Ok(BumpAllocation::wrap(boxu, self))
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
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(result.unwrap_err(), 123);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_try_with_mut<T, E>(
        &mut self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpAllocation<T, &Self>, E>, AllocError> {
        Ok(match self.imp_alloc_try_with(f)? {
            Ok(boxu) => Ok(BumpAllocation::wrap(boxu, self)),
            Err(err) => Err(err),
        })
    }

    /// TODO
    pub fn alloc_vec_greedy<E, T>(&self, len: usize) -> VecU<'a, T> {
        panic_on_error(self.imp_alloc_vec_greedy(len))
    }

    /// TODO
    pub fn try_alloc_vec_greedy<E, T>(&self, len: usize) -> Result<VecU<'a, T>, AllocError> {
        self.imp_alloc_vec_greedy(len)
    }
}

/// Generic `alloc` methods.
impl<'a, A, const C: BumpConfig> BumpScope<'a, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    pub(crate) fn imp_alloc<E, T>(&self, value: T) -> Result<BoxU<'a, T>, E>
    where
        E: ErrorBehavior,
    {
        Ok(box_init(self.imp_alloc_uninit()?, value))
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_with<E, T>(&self, f: impl FnOnce() -> T) -> Result<BoxU<'a, T>, E>
    where
        E: ErrorBehavior,
    {
        Ok(box_init(self.imp_alloc_uninit()?, f()))
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_default<E, T: Default>(&self) -> Result<BoxU<'a, T>, E>
    where
        E: ErrorBehavior,
    {
        self.imp_alloc_with(T::default)
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_clone<E, T: CloneToUninit + ?Sized>(&self, value: &T) -> Result<BoxU<'a, T>, E>
    where
        E: ErrorBehavior,
    {
        let data = self.raw.allocate(Layout::for_value(value))?;
        let metadata = ptr::metadata(value);

        unsafe {
            value.clone_to_uninit(data.as_ptr());
            let ptr = ptr::from_raw_parts_mut(data.as_ptr(), metadata);
            Ok(BoxU::from_raw_in(ptr, &Unmanaged))
        }
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_uninit<E: ErrorBehavior, T>(&self) -> Result<BoxU<'a, MaybeUninit<T>>, E>
    where
        E: ErrorBehavior,
    {
        if T::IS_ZST {
            return unsafe { Ok(BoxU::from_raw_in(ptr::dangling_mut(), &Unmanaged)) };
        }

        let ptr = self.raw.allocate_sized()?.as_ptr();
        unsafe { Ok(BoxU::from_raw_in(ptr, &Unmanaged)) }
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_slice_move<E, T>(&self, slice: impl OwnedSlice<Item = T>) -> Result<BoxU<'a, [T]>, E>
    where
        E: ErrorBehavior,
    {
        unsafe {
            let mut owned_slice = slice.into_take_owned_slice();
            let owned_slice_ref = owned_slice.owned_slice_ref();

            let len = owned_slice_ref.len();
            let src = owned_slice_ref.as_ptr();
            let dst = self.spec_alloc(Layout::for_value(owned_slice_ref))?.as_ptr().cast();
            ptr::copy_nonoverlapping(src, dst, len);

            owned_slice.take_owned_slice();
            Ok(BoxU::from_raw_in(ptr::slice_from_raw_parts_mut(dst, len), &Unmanaged))
        }
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_slice_copy<E, T: Copy>(&self, slice: &[T]) -> Result<BoxU<'a, [T]>, E>
    where
        E: ErrorBehavior,
    {
        if T::IS_ZST {
            return Ok(box_init_clone(boxu_uninit_zst_slice(slice.len()), slice));
        }

        let len = slice.len();
        let src = slice.as_ptr();
        let dst = self.raw.allocate_slice_for(slice)?.as_ptr();

        unsafe {
            core::ptr::copy_nonoverlapping(src, dst, len);
            Ok(BoxU::from_raw_in(ptr::slice_from_raw_parts_mut(dst, len), &Unmanaged))
        }
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_slice_clone<E: ErrorBehavior, T: Clone>(&self, slice: &[T]) -> Result<BoxU<'a, [T]>, E>
    where
        E: ErrorBehavior,
    {
        Ok(box_init_clone(self.imp_alloc_uninit_slice_for(slice)?, slice))
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_slice_fill<E, T: Clone>(&self, len: usize, value: T) -> Result<BoxU<'a, [T]>, E>
    where
        E: ErrorBehavior,
    {
        Ok(box_init_fill(self.imp_alloc_uninit_slice(len)?, value))
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_slice_fill_with<E, T>(&self, len: usize, f: impl FnMut() -> T) -> Result<BoxU<'a, [T]>, E>
    where
        E: ErrorBehavior,
    {
        Ok(box_init_fill_with(self.imp_alloc_uninit_slice(len)?, f))
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_uninit_slice<E, T>(&self, len: usize) -> Result<BoxU<'a, [MaybeUninit<T>]>, E>
    where
        E: ErrorBehavior,
    {
        if T::IS_ZST {
            return Ok(boxu_uninit_zst_slice(len));
        }

        let ptr = self.raw.allocate_slice::<E, MaybeUninit<T>>(len)?.as_ptr();

        unsafe {
            let slice_ptr = ptr::slice_from_raw_parts_mut(ptr, len);
            Ok(BoxU::from_raw_in(slice_ptr, &Unmanaged))
        }
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_uninit_slice_for<E, T>(&self, slice: &[T]) -> Result<BoxU<'a, [MaybeUninit<T>]>, E>
    where
        E: ErrorBehavior,
    {
        let ptr = self.raw.allocate_slice_for(slice)?.cast::<MaybeUninit<T>>().as_ptr();

        unsafe {
            let slice_ptr = ptr::slice_from_raw_parts_mut(ptr, slice.len());
            Ok(BoxU::from_raw_in(slice_ptr, &Unmanaged))
        }
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_str<E>(&self, src: &str) -> Result<BoxU<'a, str>, E>
    where
        E: ErrorBehavior,
    {
        let bytes = self.imp_alloc_slice_copy(src.as_bytes())?;

        // SAFETY: src is utf8 so this is too
        Ok(unsafe { box_from_utf8_unchecked(bytes) })
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_fmt<E>(&self, args: fmt::Arguments) -> Result<BoxU<'a, str>, E>
    where
        E: ErrorBehavior,
    {
        if let Some(string) = args.as_str() {
            return self.imp_alloc_str(string);
        }

        let mut string = String::<_, E>::new(self);
        string.write_fmt(args)?;
        Ok(string.into_str())
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_fmt_mut<E>(&mut self, args: fmt::Arguments) -> Result<BoxU<'a, str>, E>
    where
        E: ErrorBehavior,
    {
        if let Some(string) = args.as_str() {
            return self.imp_alloc_str(string);
        }

        let vec = self.imp_alloc_vec_greedy::<E, u8>(0)?;
        let mut string = String::<_, E>::new_from_vec(vec);
        string.write_fmt(args)?;
        Ok(string.into_str())
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_cstr<E>(&self, src: &CStr) -> Result<&'a CStr, E>
    where
        E: ErrorBehavior,
    {
        let (slice, _) = BoxU::into_raw_with_allocator(self.imp_alloc_slice_copy(src.to_bytes_with_nul())?);

        // SAFETY: input is `CStr` so this is too
        Ok(unsafe { CStr::from_bytes_with_nul_unchecked(&*slice) })
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_cstr_from_str<E: ErrorBehavior>(&self, src: &str) -> Result<&'a CStr, E> {
        let src = src.as_bytes();

        if let Some(nul) = src.iter().position(|&c| c == b'\0') {
            let bytes_with_nul = unsafe { src.get_unchecked(..nul + 1) };
            let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) };
            self.imp_alloc_cstr(cstr)
        } else {
            // `src` contains no null
            let dst = self.raw.allocate_slice::<E, u8>(src.len() + 1)?;

            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
                dst.as_ptr().add(src.len()).write(0);

                let bytes = core::slice::from_raw_parts(dst.as_ptr(), src.len() + 1);
                Ok(CStr::from_bytes_with_nul_unchecked(bytes))
            }
        }
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_cstr_fmt<E>(&self, args: fmt::Arguments) -> Result<&'a CStr, E>
    where
        E: ErrorBehavior,
    {
        if let Some(string) = args.as_str() {
            return self.imp_alloc_cstr_from_str(string);
        }

        let mut string = String::<_, E>::new(self);
        string.write_fmt(args)?;
        string.into_cstr()
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_cstr_fmt_mut<E>(&mut self, args: fmt::Arguments) -> Result<&'a CStr, E>
    where
        E: ErrorBehavior,
    {
        if let Some(string) = args.as_str() {
            return self.imp_alloc_cstr_from_str(string);
        }

        let vec = self.imp_alloc_vec_greedy(0)?;
        let mut string = String::new_from_vec(vec);
        string.write_fmt(args)?;
        string.into_cstr()
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_iter<E, T>(&self, iter: impl IntoIterator<Item = T>) -> Result<BoxU<'a, [T]>, E>
    where
        E: ErrorBehavior,
    {
        if E::FALLIBLE {
            todo!()
        }

        let mut vec = Vec::new_in(self);
        vec.extend(iter);
        Ok(boxu(vec.into_boxed_slice()))
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_iter_mut<E, T>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<BoxU<'a, [T]>, E>
    where
        E: ErrorBehavior,
    {
        if E::FALLIBLE {
            todo!()
        }

        let mut vec = self.imp_alloc_vec_greedy::<E, T>(0)?;
        vec.extend(iter);
        Ok(boxu(vec.into_boxed_slice()))
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_try_with<B: ErrorBehavior, T, E>(
        &self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BoxU<'a, T>, E>, B> {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(box_init(boxu_uninit_zst(), value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let checkpoint_before_alloc = self.checkpoint();
        let uninit = self.imp_alloc_uninit::<B, Result<T, E>>()?;
        let ptr = BoxU::into_raw_with_allocator(uninit).0.cast::<Result<T, E>>();

        // When bumping downwards the chunk's position is the same as `ptr`.
        // Using `ptr` is faster so we use that.
        let pos = if config!(upwards) {
            self.raw.chunk.get().pos().as_ptr()
        } else {
            ptr.cast::<u8>()
        };

        Ok(unsafe {
            ptr.write(f());

            match util::ptr_result(ptr) {
                Ok(value) => Ok(BoxU::from_raw_in(value, &Unmanaged)),
                Err(error) => Err({
                    let error = error.read();

                    // If `f` made allocations on this bump allocator we can't shrink the allocation.
                    let can_shrink = pos == self.raw.chunk.get().pos().as_ptr();

                    if can_shrink {
                        self.reset_to(checkpoint_before_alloc);
                    }

                    error
                }),
            }
        })
    }

    #[inline(always)]
    pub(crate) fn imp_alloc_try_with_mut<B, T, E>(
        &mut self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BoxU<'a, T>, E>, B>
    where
        B: ErrorBehavior,
    {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(box_init(boxu_uninit_zst(), value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let ptr = self.raw.prepare_sized_allocation::<B, Result<T, E>>()?.as_ptr();

        Ok(unsafe {
            ptr.write(f());

            // There is no need for `can_shrink` checks, because we have a mutable reference
            // so there's no way anyone else has allocated in `f`.
            match util::ptr_result(ptr) {
                Ok(value) => Ok({
                    let new_pos = if config!(upwards) {
                        let pos = value.add(1).addr();
                        up_align_usize_unchecked(pos, config!(min_align))
                    } else {
                        let pos = value.addr();
                        down_align_usize(pos, config!(min_align))
                    };

                    // The allocation was successful, so our chunk must be allocated.
                    let chunk = self.raw.chunk.get().as_allocated_unchecked();
                    chunk.set_pos_addr(new_pos);

                    BoxU::from_raw_in(value, &Unmanaged)
                }),
                Err(error) => Err(error.read()),
            }
        })
    }

    /// TODO
    // FIXME: non-greedy variant for a more regular api / easy switching?
    pub(crate) fn imp_alloc_vec_greedy<E, T>(&self, len: usize) -> Result<VecU<'a, T>, E>
    where
        E: ErrorBehavior,
    {
        let slice = self.spec_alloc_slice_greedy::<E, T>(len)?;
        let cap = slice.len();
        let ptr = slice.cast::<T>().as_ptr();
        unsafe { Ok(VecU::from_raw_parts_in(ptr, 0, cap, &Unmanaged)) }
    }
}

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
    /// [`self.is_claimed()`]: Self::is_claimed
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
