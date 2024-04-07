use core::{
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
};
use std::sync::{Mutex, PoisonError};

use allocator_api2::alloc::{AllocError, Allocator};

#[cfg(feature = "alloc")]
use allocator_api2::alloc::Global;

use crate::{Bump, BumpScope, MinimumAlignment, SupportedMinimumAlignment};

/// A pool of bump allocators.
///
/// This type allows you to do bump allocations from different threads that have their lifetime tied to the pool.
///
/// # Examples
///
/// Using `BumpPool` with parallel iterators from [`rayon`](https://docs.rs/rayon):
/// ```
/// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
/// # use bump_scope::{ BumpPool, allocator_api2::alloc::Global };
/// # use rayon::prelude::{ ParallelIterator, IntoParallelIterator };
/// # if cfg!(miri) { return } // rayon violates strict-provenance :(
/// #
/// let mut pool: BumpPool = BumpPool::new();
///
/// let ints: Vec<&mut usize> = (0..1000)
///     .into_par_iter()
///     .map_init(|| pool.get(), |bump, i| {
///         // do some expensive work
///         bump.alloc(i).into_mut()
///     })
///     .collect();
///
/// dbg!(&ints);
///
/// pool.reset();
///
/// // memory of the int references is freed, trying to access ints will result in a lifetime error
/// // dbg!(&ints);
/// ```
///
/// Using `BumpPool` with [`std::thread::scope`]:
/// ```
/// # #![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
/// # use bump_scope::{ BumpPool, allocator_api2::alloc::Global };
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
pub struct BumpPool<
    #[cfg(feature = "alloc")] A = Global,
    #[cfg(not(feature = "alloc"))] A,
    const MIN_ALIGN: usize = 1,
    const UP: bool = true,
> where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    bumps: Mutex<Vec<Bump<A, MIN_ALIGN, UP>>>,
    allocator: A,
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Default for BumpPool<A, MIN_ALIGN, UP>
where
    A: Allocator + Clone + Default,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn default() -> Self {
        Self {
            bumps: Mutex::default(),
            allocator: Default::default(),
        }
    }
}

#[cfg(feature = "alloc")]
impl<const MIN_ALIGN: usize, const UP: bool> BumpPool<Global, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    /// Constructs a new `BumpPool`.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::new_in(Global)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> BumpPool<A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    /// Constructs a new `BumpPool` with the provided allocator.
    #[inline]
    #[must_use]
    pub fn new_in(allocator: A) -> Self {
        Self {
            bumps: Mutex::default(),
            allocator,
        }
    }

    /// [Resets](Bump::reset) all `Bump`s in this pool.
    pub fn reset(&mut self) {
        for bump in self.bumps().iter_mut() {
            bump.reset();
        }
    }

    /// Returns the vector of `Bump`s.
    pub fn bumps(&mut self) -> &mut Vec<Bump<A, MIN_ALIGN, UP>> {
        self.bumps.get_mut().unwrap_or_else(PoisonError::into_inner)
    }

    /// Borrows a bump allocator from the pool.
    /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[must_use]
    #[cfg(not(no_global_oom_handling))]
    pub fn get(&self) -> BumpPoolGuard<A, MIN_ALIGN, UP> {
        let bump = self.bumps.lock().unwrap_or_else(PoisonError::into_inner).pop();
        let bump = bump.unwrap_or_else(|| Bump::new_in(self.allocator.clone()));

        BumpPoolGuard {
            pool: self,
            bump: ManuallyDrop::new(bump),
        }
    }

    /// Borrows a bump allocator from the pool.
    /// With this `BumpPoolGuard` you can make allocations that live for as long as the pool lives.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    pub fn try_get(&self) -> Result<BumpPoolGuard<A, MIN_ALIGN, UP>, AllocError> {
        let bump = self.bumps.lock().unwrap_or_else(PoisonError::into_inner).pop();

        let bump = match bump {
            Some(bump) => bump,
            None => Bump::try_new_in(self.allocator.clone())?,
        };

        Ok(BumpPoolGuard {
            pool: self,
            bump: ManuallyDrop::new(bump),
        })
    }
}

/// This is a wrapper around [`Bump`] that mutably derefs to a [`BumpScope`] and returns its [`Bump`] back to the [`BumpPool`] on drop.
#[derive(Debug)]
pub struct BumpPoolGuard<'a, A, const MIN_ALIGN: usize, const UP: bool>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    bump: ManuallyDrop<Bump<A, MIN_ALIGN, UP>>,
    pool: &'a BumpPool<A, MIN_ALIGN, UP>,
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Deref for BumpPoolGuard<'a, A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    type Target = BumpScope<'a, A, MIN_ALIGN, UP>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { transmute_lifetime(self.bump.as_scope()) }
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> DerefMut for BumpPoolGuard<'a, A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute_lifetime_mut(self.bump.as_mut_scope()) }
    }
}

impl<'a, A, const MIN_ALIGN: usize, const UP: bool> Drop for BumpPoolGuard<'a, A, MIN_ALIGN, UP>
where
    A: Allocator + Clone,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn drop(&mut self) {
        let mut bumps = self.pool.bumps.lock().unwrap();
        let bump = unsafe { ManuallyDrop::take(&mut self.bump) };
        bumps.push(bump);
    }
}

// This exists as a "safer" transmute that only transmutes the `'a` lifetime parameter.
#[allow(clippy::needless_lifetimes)]
unsafe fn transmute_lifetime<'from, 'to, 'b, A, const MIN_ALIGN: usize, const UP: bool>(
    scope: &'b BumpScope<'from, A, MIN_ALIGN, UP>,
) -> &'b BumpScope<'to, A, MIN_ALIGN, UP> {
    mem::transmute(scope)
}

// This exists as a "safer" transmute that only transmutes the `'a` lifetime parameter.
#[allow(clippy::needless_lifetimes)]
unsafe fn transmute_lifetime_mut<'from, 'to, 'b, A, const MIN_ALIGN: usize, const UP: bool>(
    scope: &'b mut BumpScope<'from, A, MIN_ALIGN, UP>,
) -> &'b mut BumpScope<'to, A, MIN_ALIGN, UP> {
    mem::transmute(scope)
}
