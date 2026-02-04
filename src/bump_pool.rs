use std::{
    alloc::Layout,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard, PoisonError},
    vec::Vec,
};

use crate::{
    Bump, BumpScope, ErrorBehavior,
    alloc::{AllocError, Allocator},
    maybe_default_allocator,
    settings::{BumpAllocatorSettings, BumpSettings},
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

macro_rules! make_pool {
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
        /// # use rayon::prelude::{ParallelIterator, IntoParallelIterator};
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
        pub struct BumpPool<$($allocator_parameter)*, S = BumpSettings>
        where
            A: Allocator,
            S: BumpAllocatorSettings,
        {
            bumps: Mutex<Vec<Bump<A, S>>>,
            allocator: A,
        }
    };
}

maybe_default_allocator!(make_pool);

impl<A, S> Default for BumpPool<A, S>
where
    A: Allocator + Default,
    S: BumpAllocatorSettings,
{
    fn default() -> Self {
        Self {
            bumps: Mutex::default(),
            allocator: Default::default(),
        }
    }
}

impl<A, S> BumpPool<A, S>
where
    A: Allocator + Default,
    S: BumpAllocatorSettings,
{
    /// Constructs a new `BumpPool`.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<A, S> BumpPool<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
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
    pub fn bumps(&mut self) -> &mut Vec<Bump<A, S>> {
        self.bumps.get_mut().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Bump<A, S>>> {
        self.bumps.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<A, S> BumpPool<A, S>
where
    A: Allocator + Clone,
    S: BumpAllocatorSettings,
{
    /// Borrows a bump allocator from the pool.
    /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
    ///
    /// If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[new]\()</code>.
    ///
    /// [new]: Bump::new
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn get(&self) -> BumpPoolGuard<'_, A, S> {
        let bump = match self.lock().pop() {
            Some(bump) => bump,
            None => Bump::new_in(self.allocator.clone()),
        };

        BumpPoolGuard {
            pool: self,
            bump: ManuallyDrop::new(bump),
        }
    }

    /// Borrows a bump allocator from the pool.
    /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
    ///
    /// If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[try_new]\()</code>.
    ///
    /// [try_new]: Bump::try_new
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn try_get(&self) -> Result<BumpPoolGuard<'_, A, S>, AllocError> {
        let bump = match self.lock().pop() {
            Some(bump) => bump,
            None => Bump::try_new_in(self.allocator.clone())?,
        };

        Ok(BumpPoolGuard {
            pool: self,
            bump: ManuallyDrop::new(bump),
        })
    }

    /// Borrows a bump allocator from the pool.
    /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
    ///
    ///  If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[with_size]\(size)</code>.
    ///
    /// [with_size]: Bump::with_size
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn get_with_size(&self, size: usize) -> BumpPoolGuard<'_, A, S> {
        panic_on_error(self.generic_get_with_size(size))
    }

    /// Borrows a bump allocator from the pool.
    /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
    ///
    ///  If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[try_with_size]\(size)</code>.
    ///
    /// [try_with_size]: Bump::try_with_size
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn try_get_with_size(&self, size: usize) -> Result<BumpPoolGuard<'_, A, S>, AllocError> {
        self.generic_get_with_size(size)
    }

    pub(crate) fn generic_get_with_size<E: ErrorBehavior>(&self, size: usize) -> Result<BumpPoolGuard<'_, A, S>, E> {
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
    ///
    /// If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[with_capacity]\(layout)</code>.
    ///
    /// [with_capacity]: Bump::with_capacity
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn get_with_capacity(&self, layout: Layout) -> BumpPoolGuard<'_, A, S> {
        panic_on_error(self.generic_get_with_capacity(layout))
    }

    /// Borrows a bump allocator from the pool.
    /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
    ///
    ///  If this needs to create a new `Bump`, it will be constructed by calling <code>Bump::[try_with_capacity]\(layout)</code>.
    ///
    /// [try_with_capacity]: Bump::try_with_capacity
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn try_get_with_capacity(&self, layout: Layout) -> Result<BumpPoolGuard<'_, A, S>, AllocError> {
        self.generic_get_with_capacity(layout)
    }

    pub(crate) fn generic_get_with_capacity<E: ErrorBehavior>(&self, layout: Layout) -> Result<BumpPoolGuard<'_, A, S>, E> {
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

macro_rules! make_pool_guard {
    ($($allocator_parameter:tt)*) => {

        /// This is a wrapper around [`Bump`] that mutably derefs to a [`BumpScope`] and returns its [`Bump`] back to the [`BumpPool`] on drop.
        #[derive(Debug)]
        pub struct BumpPoolGuard<'a, $($allocator_parameter)*, S = BumpSettings>
        where
            A: Allocator,
            S: BumpAllocatorSettings,
        {
            bump: ManuallyDrop<Bump<A, S>>,
            pool: &'a BumpPool<A, S>,
        }
    };
}

maybe_default_allocator!(make_pool_guard);

impl<'a, A, S> BumpPoolGuard<'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    /// The [`BumpPool`], this [`BumpPoolGuard`] was created from.
    pub fn pool(&self) -> &'a BumpPool<A, S> {
        self.pool
    }
}

impl<'a, A, S> Deref for BumpPoolGuard<'a, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    type Target = BumpScope<'a, A, S>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { transmute_lifetime(self.bump.as_scope()) }
    }
}

impl<A, S> DerefMut for BumpPoolGuard<'_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute_lifetime_mut(self.bump.as_mut_scope()) }
    }
}

impl<A, S> Drop for BumpPoolGuard<'_, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    fn drop(&mut self) {
        let bump = unsafe { ManuallyDrop::take(&mut self.bump) };
        self.pool.lock().push(bump);
    }
}

// This exists as a "safer" transmute that only transmutes the `'a` lifetime parameter.
#[expect(clippy::elidable_lifetime_names)]
unsafe fn transmute_lifetime<'from, 'to, 'b, A, S>(scope: &'b BumpScope<'from, A, S>) -> &'b BumpScope<'to, A, S>
where
    S: BumpAllocatorSettings,
{
    unsafe { mem::transmute(scope) }
}

// This exists as a "safer" transmute that only transmutes the `'a` lifetime parameter.
#[expect(clippy::elidable_lifetime_names)]
unsafe fn transmute_lifetime_mut<'from, 'to, 'b, A, S>(scope: &'b mut BumpScope<'from, A, S>) -> &'b mut BumpScope<'to, A, S>
where
    S: BumpAllocatorSettings,
{
    unsafe { mem::transmute(scope) }
}
