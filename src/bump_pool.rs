use std::{
    alloc::Layout,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard, PoisonError},
    vec::Vec,
};

use crate::{
    error_behavior_generic_methods_allocation_failure, BaseAllocator, Bump, BumpScope, MinimumAlignment,
    SupportedMinimumAlignment,
};

macro_rules! bump_pool_declaration {
    ($($allocator_parameter:tt)*) => {
        /// A pool of bump allocators.
        ///
        /// This type allows bump allocations in parallel, with the allocations' lifetimes tied to the pool.
        ///
        /// # Examples
        ///
        /// Using `BumpPool` with parallel iterators from [`rayon`](https://docs.rs/rayon):
        /// ```
        /// # use bump_scope::BumpPool;
        /// # use rayon::prelude::{ ParallelIterator, IntoParallelIterator };
        /// # if cfg!(miri) { return } // rayon violates strict-provenance :(
        /// #
        /// let mut pool: BumpPool = BumpPool::new();
        ///
        /// let strings: Vec<&str> = (0..1000)
        ///     .into_par_iter()
        ///     .map_init(|| pool.get(), |bump, i| {
        ///         // do some expensive work
        ///         bump.alloc_fmt(format_args!("{i}")).into_ref()
        ///     })
        ///     .collect();
        ///
        /// dbg!(&strings);
        ///
        /// pool.reset();
        ///
        /// // memory of the strings is freed, trying to access `strings` will result in a lifetime error
        /// // dbg!(&strings);
        /// ```
        ///
        /// Using `BumpPool` with [`std::thread::scope`]:
        /// ```
        /// # use bump_scope::BumpPool;
        /// let pool: BumpPool = BumpPool::new();
        /// let (sender, receiver) = std::sync::mpsc::sync_channel(10);
        ///
        /// std::thread::scope(|s| {
        ///     s.spawn(|| {
        ///         let bump = pool.get();
        ///         let string = bump.alloc_str("Hello");
        ///         sender.send(string).unwrap();
        ///         drop(sender);
        ///     });
        ///
        ///     s.spawn(|| {
        ///         for string in receiver {
        ///             assert_eq!(string, "Hello");
        ///         }
        ///     });
        /// });
        /// ```
        ///
        #[doc(alias = "Herd")]
        #[derive(Debug)]
        pub struct BumpPool<
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
        > where
            A: BaseAllocator,
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        {
            bumps: Mutex<Vec<Bump<A, MIN_ALIGN, UP>>>,
            allocator: A,
        }
    };
}

crate::maybe_default_allocator!(bump_pool_declaration);

impl<A, const MIN_ALIGN: usize, const UP: bool> Default for BumpPool<A, MIN_ALIGN, UP>
where
    A: BaseAllocator + Default,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn default() -> Self {
        Self {
            bumps: Mutex::default(),
            allocator: Default::default(),
        }
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> BumpPool<A, MIN_ALIGN, UP>
where
    A: BaseAllocator + Default,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    /// Constructs a new `BumpPool`.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> BumpPool<A, MIN_ALIGN, UP>
where
    A: BaseAllocator,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    /// Constructs a new `BumpPool` with the provided allocator.
    #[inline]
    #[must_use]
    pub const fn new_in(allocator: A) -> Self {
        Self {
            bumps: Mutex::new(Vec::new()),
            allocator,
        }
    }

    /// [Resets](Bump::reset) all `Bump`s in this pool.
    pub fn reset(&mut self) {
        for bump in self.bumps() {
            bump.reset();
        }
    }

    /// Returns the vector of `Bump`s.
    pub fn bumps(&mut self) -> &mut Vec<Bump<A, MIN_ALIGN, UP>> {
        self.bumps.get_mut().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Bump<A, MIN_ALIGN, UP>>> {
        self.bumps.lock().unwrap_or_else(PoisonError::into_inner)
    }

    error_behavior_generic_methods_allocation_failure! {
        /// Borrows a bump allocator from the pool.
        /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
        impl
        ///
        /// If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[new]\()</code>.
        ///
        /// [new]: Bump::new
        #[must_use]
        for fn get
        ///
        /// If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[try_new]\()</code>.
        ///
        /// [try_new]: Bump::try_new
        for fn try_get
        use fn generic_get(&self,) -> BumpPoolGuard<A, MIN_ALIGN, UP> {
            let bump = match self.lock().pop() {
                Some(bump) => bump,
                None => Bump::generic_new_in(self.allocator.clone())?,
            };

            Ok(BumpPoolGuard {
                pool: self,
                bump: ManuallyDrop::new(bump),
            })
        }

        /// Borrows a bump allocator from the pool.
        /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
        impl
        ///
        /// If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[with_size]\(size)</code>.
        ///
        /// [with_size]: Bump::with_size
        #[must_use]
        for fn get_with_size
        ///
        ///  If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[try_with_size]\(size)</code>.
        ///
        /// [try_with_size]: Bump::try_with_size
        for fn try_get_with_size
        use fn generic_get_with_size(&self, size: usize) -> BumpPoolGuard<A, MIN_ALIGN, UP> {
            let bump = match self.lock().pop() {
                Some(bump) => bump,
                None => Bump::generic_with_size_in(size, self.allocator.clone())?,
            };

            Ok(BumpPoolGuard {
                pool: self,
                bump: ManuallyDrop::new(bump),
            })
        }

        /// Borrows a bump allocator from the pool.
        /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
        impl
        ///
        /// If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[with_capacity]\(layout)</code>.
        ///
        /// [with_capacity]: Bump::with_capacity
        #[must_use]
        for fn get_with_capacity
        ///
        ///  If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[try_with_capacity]\(layout)</code>.
        ///
        /// [try_with_capacity]: Bump::try_with_capacity
        for fn try_get_with_capacity
        use fn generic_get_with_capacity(&self, layout: Layout) -> BumpPoolGuard<A, MIN_ALIGN, UP> {
            let bump = match self.lock().pop() {
                Some(bump) => bump,
                None => Bump::generic_with_capacity_in(layout, self.allocator.clone())?,
            };

            Ok(BumpPoolGuard {
                pool: self,
                bump: ManuallyDrop::new(bump),
            })
        }
    }
}

macro_rules! bump_pool_guard_declaration {
    ($($allocator_parameter:tt)*) => {

        /// This is a wrapper around [`Bump`] that mutably derefs to a [`BumpScope`] and returns its [`Bump`] back to the [`BumpPool`] on drop.
        #[derive(Debug)]
        pub struct BumpPoolGuard<
            'a,
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
        > where
            A: BaseAllocator,
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        {
            bump: ManuallyDrop<Bump<A, MIN_ALIGN, UP>>,

            /// The [`BumpPool`], this [`BumpPoolGuard`] was created from.
            pub pool: &'a BumpPool<A, MIN_ALIGN, UP>,
        }
    };
}

crate::maybe_default_allocator!(bump_pool_guard_declaration);

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Deref for BumpPoolGuard<'a, A, MIN_ALIGN, UP>
where
    A: BaseAllocator,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    type Target = BumpScope<'a, A, MIN_ALIGN, UP>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { transmute_lifetime(self.bump.as_scope()) }
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> DerefMut for BumpPoolGuard<'_, A, MIN_ALIGN, UP>
where
    A: BaseAllocator,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute_lifetime_mut(self.bump.as_mut_scope()) }
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Drop for BumpPoolGuard<'_, A, MIN_ALIGN, UP>
where
    A: BaseAllocator,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn drop(&mut self) {
        let bump = unsafe { ManuallyDrop::take(&mut self.bump) };
        self.pool.lock().push(bump);
    }
}

// This exists as a "safer" transmute that only transmutes the `'a` lifetime parameter.
#[allow(clippy::needless_lifetimes, clippy::elidable_lifetime_names)]
unsafe fn transmute_lifetime<'from, 'to, 'b, A, const MIN_ALIGN: usize, const UP: bool>(
    scope: &'b BumpScope<'from, A, MIN_ALIGN, UP>,
) -> &'b BumpScope<'to, A, MIN_ALIGN, UP> {
    mem::transmute(scope)
}

// This exists as a "safer" transmute that only transmutes the `'a` lifetime parameter.
#[allow(clippy::needless_lifetimes, clippy::elidable_lifetime_names)]
unsafe fn transmute_lifetime_mut<'from, 'to, 'b, A, const MIN_ALIGN: usize, const UP: bool>(
    scope: &'b mut BumpScope<'from, A, MIN_ALIGN, UP>,
) -> &'b mut BumpScope<'to, A, MIN_ALIGN, UP> {
    mem::transmute(scope)
}
