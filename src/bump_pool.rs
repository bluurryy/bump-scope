use std::{
    alloc::{AllocError, Allocator, Layout},
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard, PoisonError},
    vec::Vec,
};

use alloc::alloc::Global;

use crate::{
    Bump, BumpScope,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG},
};

/// A pool of bump allocators.
///
/// This type allows bump allocations in parallel, with the allocations' lifetimes tied to the pool.
///
/// # Examples
///
/// Using `BumpPool` with parallel iterators from [`rayon`](https://docs.rs/rayon):
/// ```
/// use bump_scope::{BumpPool, BumpAllocator};
/// use rayon::prelude::{ParallelIterator, IntoParallelIterator};
/// # if cfg!(miri) { return } // rayon violates strict-provenance :(
///
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
/// use bump_scope::{BumpPool, BumpAllocator};
///
/// let pool: BumpPool = BumpPool::new();
/// let (sender, receiver) = std::sync::mpsc::sync_channel(10);
///
/// std::thread::scope(|s| {
///     s.spawn(|| {
///         let bump = pool.get();
///         let string = bump.alloc_str("Hello").into_ref();
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
pub struct BumpPool<A = Global, const C: BumpConfig = DEFAULT_BUMP_CONFIG>
where
    A: Allocator,
{
    bumps: Mutex<Vec<Bump<A, C>>>,
    allocator: A,
}

impl<A, const C: BumpConfig> Default for BumpPool<A, C>
where
    A: Allocator + Default,
{
    fn default() -> Self {
        Self {
            bumps: Mutex::default(),
            allocator: Default::default(),
        }
    }
}

impl<A, const C: BumpConfig> BumpPool<A, C>
where
    A: Allocator + Default,
{
    /// Constructs a new `BumpPool`.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<A, const C: BumpConfig> BumpPool<A, C>
where
    A: Allocator,
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

    /// [`reset`](Bump::reset) all `Bump`s in this pool.
    pub fn reset(&mut self) {
        for bump in self.bumps() {
            bump.reset();
        }
    }

    /// [`reset_to_start`](Bump::reset_to_start) all `Bump`s in this pool.
    pub fn reset_to_start(&mut self) {
        for bump in self.bumps() {
            bump.reset_to_start();
        }
    }

    /// Returns the vector of `Bump`s.
    pub fn bumps(&mut self) -> &mut Vec<Bump<A, C>> {
        self.bumps.get_mut().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Bump<A, C>>> {
        self.bumps.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<A, const C: BumpConfig> BumpPool<A, C>
where
    A: Allocator + Clone,
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
    pub fn get(&self) -> BumpPoolGuard<'_, A, C> {
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
    pub fn try_get(&self) -> Result<BumpPoolGuard<'_, A, C>, AllocError> {
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
    pub fn get_with_size(&self, size: usize) -> BumpPoolGuard<'_, A, C> {
        let bump = match self.lock().pop() {
            Some(bump) => bump,
            None => Bump::with_size_in(size, self.allocator.clone()),
        };

        BumpPoolGuard {
            pool: self,
            bump: ManuallyDrop::new(bump),
        }
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
    pub fn try_get_with_size(&self, size: usize) -> Result<BumpPoolGuard<'_, A, C>, AllocError> {
        let bump = match self.lock().pop() {
            Some(bump) => bump,
            None => Bump::try_with_size_in(size, self.allocator.clone())?,
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
    pub fn get_with_capacity(&self, layout: Layout) -> BumpPoolGuard<'_, A, C> {
        let bump = match self.lock().pop() {
            Some(bump) => bump,
            None => Bump::with_capacity_in(layout, self.allocator.clone()),
        };

        BumpPoolGuard {
            pool: self,
            bump: ManuallyDrop::new(bump),
        }
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
    pub fn try_get_with_capacity(&self, layout: Layout) -> Result<BumpPoolGuard<'_, A, C>, AllocError> {
        let bump = match self.lock().pop() {
            Some(bump) => bump,
            None => Bump::try_with_capacity_in(layout, self.allocator.clone())?,
        };

        Ok(BumpPoolGuard {
            pool: self,
            bump: ManuallyDrop::new(bump),
        })
    }
}

/// This is a wrapper around [`Bump`] that mutably derefs to a [`BumpScope`] and returns its [`Bump`] back to the [`BumpPool`] on drop.
#[derive(Debug)]
pub struct BumpPoolGuard<'a, A: Allocator = Global, const C: BumpConfig = DEFAULT_BUMP_CONFIG> {
    bump: ManuallyDrop<Bump<A, C>>,
    pool: &'a BumpPool<A, C>,
}

impl<'a, A, const C: BumpConfig> BumpPoolGuard<'a, A, C>
where
    A: Allocator,
{
    /// The [`BumpPool`], this [`BumpPoolGuard`] was created from.
    pub fn pool(&self) -> &'a BumpPool<A, C> {
        self.pool
    }
}

impl<'a, A, const C: BumpConfig> Deref for BumpPoolGuard<'a, A, C>
where
    A: Allocator,
{
    type Target = BumpScope<'a, A, C>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: We return allocations for the lifetime of the pool reference
        // instead of `bump`s own lifetime. This is fine because `BumpPool` and
        // `BumpPoolGuard` don't expose any api to invalidate an allocation for
        // lifetime `'a`.
        //
        // - `BumpPool::reset` requires a mutable reference, and since allocations
        //   immutably borrow the pool, the compiler doesn't allow you to `reset`
        //   before all allocations are gone.
        //
        // - `mem::forget(bump_pool_guard)` will not return the `Bump` to its pool,
        //   however the `Bump` won't be dropped either, so its allocations will
        //   live for `'static` which is compatible with `'a`.
        unsafe { transmute_lifetime(self.bump.as_scope()) }
    }
}

impl<A, const C: BumpConfig> DerefMut for BumpPoolGuard<'_, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute_lifetime_mut(self.bump.as_mut_scope()) }
    }
}

impl<A, const C: BumpConfig> Drop for BumpPoolGuard<'_, A, C>
where
    A: Allocator,
{
    fn drop(&mut self) {
        let bump = unsafe { ManuallyDrop::take(&mut self.bump) };
        self.pool.lock().push(bump);
    }
}

// This exists as a "safer" transmute that only transmutes the `'a` lifetime parameter.
#[expect(clippy::elidable_lifetime_names)]
unsafe fn transmute_lifetime<'from, 'to, 'b, A, const C: BumpConfig>(
    scope: &'b BumpScope<'from, A, C>,
) -> &'b BumpScope<'to, A, C>
where
{
    unsafe { mem::transmute(scope) }
}

// This exists as a "safer" transmute that only transmutes the `'a` lifetime parameter.
#[expect(clippy::elidable_lifetime_names)]
unsafe fn transmute_lifetime_mut<'from, 'to, 'b, A, const C: BumpConfig>(
    scope: &'b mut BumpScope<'from, A, C>,
) -> &'b mut BumpScope<'to, A, C>
where
{
    unsafe { mem::transmute(scope) }
}
