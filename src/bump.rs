use core::{
    alloc::Layout,
    ffi::CStr,
    fmt::{self, Debug},
    mem::{ManuallyDrop, MaybeUninit},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

#[cfg(feature = "nightly-clone-to-uninit")]
use core::clone::CloneToUninit;

use crate::{
    BaseAllocator, BumpBox, BumpClaimGuard, BumpScope, BumpScopeGuard, Checkpoint, ErrorBehavior,
    alloc::{AllocError, Allocator},
    chunk::ChunkSize,
    maybe_default_allocator,
    owned_slice::OwnedSlice,
    polyfill::{transmute_mut, transmute_ref, transmute_value},
    raw_bump::RawBump,
    settings::{BumpAllocatorSettings, BumpSettings, False, MinimumAlignment, SupportedMinimumAlignment},
    stats::{AnyStats, Stats},
    traits::{
        self, BumpAllocator, BumpAllocatorCore, BumpAllocatorScope, BumpAllocatorTyped, BumpAllocatorTypedScope,
        MutBumpAllocatorTypedScope,
    },
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

macro_rules! make_type {
    ($($allocator_parameter:tt)*) => {
        /// The bump allocator.
        ///
        /// # Generic parameters
        /// - **`A`** — the base allocator, defaults to `Global` when the `alloc` feature is enabled
        /// - **`S`** — the bump allocator settings, see [`settings`](crate::settings)
        ///
        /// # Overview
        /// All of the mentioned methods that do allocations panic if the base allocator returned an error.
        /// For every such panicking method, there is a corresponding `try_`-prefixed version that returns a `Result` instead.
        ///
        /// #### Create a `Bump` ...
        /// - with a default size hint: <code>[new]\([_in][new_in])</code> / <code>[default]</code>
        /// - provide a size hint: <code>[with_size]\([_in][with_size_in])</code>
        /// - provide a minimum capacity: <code>[with_capacity]\([_in][with_capacity_in])</code>
        /// - const, without allocating a chunk: <code>[unallocated]</code>
        ///
        /// [new]: Bump::new
        /// [new_in]: Bump::new_in
        /// [default]: Bump::default
        /// [with_size]: Bump::with_size
        /// [with_size_in]: Bump::with_size_in
        /// [with_capacity]: Bump::with_capacity
        /// [with_capacity_in]: Bump::with_capacity_in
        /// [unallocated]: Bump::unallocated
        ///
        /// #### Allocate ...
        /// - sized values: [`alloc`], [`alloc_with`], [`alloc_default`], [`alloc_zeroed`]
        /// - strings: [`alloc_str`], <code>[alloc_fmt](BumpAllocatorTypedScope::alloc_fmt)([_mut](MutBumpAllocatorTypedScope::alloc_fmt_mut))</code>
        /// - c strings: [`alloc_cstr`], [`alloc_cstr_from_str`], <code>[alloc_cstr_fmt](BumpAllocatorTypedScope::alloc_cstr_fmt)([_mut](MutBumpAllocatorTypedScope::alloc_cstr_fmt_mut))</code>
        /// - slices: <code>alloc_slice_{[copy](BumpAllocatorTypedScope::alloc_slice_copy), [clone](BumpAllocatorTypedScope::alloc_slice_clone), [move](BumpAllocatorTypedScope::alloc_slice_move), [fill](BumpAllocatorTypedScope::alloc_slice_fill), [fill_with](BumpAllocatorTypedScope::alloc_slice_fill_with)}</code>,
        ///   [`alloc_zeroed_slice`]
        /// - slices from an iterator: [`alloc_iter`], [`alloc_iter_exact`], [`alloc_iter_mut`], [`alloc_iter_mut_rev`]
        /// - uninitialized values: [`alloc_uninit`], [`alloc_uninit_slice`], [`alloc_uninit_slice_for`]
        ///
        ///   which can then be conveniently initialized by the [`init*` methods of `BumpBox`](crate::BumpBox#bumpbox-has-a-lot-of-methods).
        /// - results: [`alloc_try_with`], [`alloc_try_with_mut`]
        /// - via clone *(nightly only)*: [`alloc_clone`]
        ///
        /// #### Free memory using ...
        /// - scopes: [`scoped`], [`scoped_aligned`], [`scope_guard`]
        /// - checkpoints: [`checkpoint`], [`reset_to`]
        /// - reset: [`reset`]
        /// - dealloc: [`dealloc`]
        ///
        /// #### Configure allocator settings ...
        /// - [`with_settings`], [`borrow_with_settings`], [`borrow_mut_with_settings`]
        ///
        /// ## Collections
        /// A `Bump` (and [`BumpScope`]) can be used to allocate collections of this crate...
        /// ```
        /// use bump_scope::{Bump, BumpString};
        /// let bump: Bump = Bump::new();
        ///
        /// let mut string = BumpString::new_in(&bump);
        /// string.push_str("Hello,");
        /// string.push_str(" world!");
        /// ```
        ///
        /// ... and collections from crates that use `allocator_api2`'s `Allocator` like [hashbrown](https://docs.rs/hashbrown)'s [`HashMap`](https://docs.rs/hashbrown/latest/hashbrown/struct.HashMap.html):
        ///
        /// *This requires the `allocator-api2-02` feature OR the `nightly-allocator-api` feature along with hashbrown's `nightly` feature.*
        // NOTE: This code is tested in `crates/test-hashbrown/lib.rs`.
        // It's not tested here because using hashbrown requires us to either have both the crate features for a nightly allocator api in bump-scope and hashbrown or neither.
        // This could be solved by making bump-scope's "nightly-allocator-api" depend on "hashbrown/nightly" but that currently breaks tools like cargo-hack and cargo-minimal-versions.
        /// ```
        /// # /*
        /// use bump_scope::Bump;
        /// use hashbrown::HashMap;
        ///
        /// let bump: Bump = Bump::new();
        /// let mut map = HashMap::new_in(&bump);
        /// map.insert("tau", 6.283);
        /// # */
        /// # ()
        /// ```
        ///
        /// On nightly and with the feature `nightly-allocator-api` you can also allocate collections from `std` that have an allocator parameter:
        #[cfg_attr(feature = "nightly-allocator-api", doc = "```")]
        #[cfg_attr(not(feature = "nightly-allocator-api"), doc = "```no_run")]
        /// # /*
        /// # those features are already been enabled by a `doc(test(attr`
        /// # but we still want it here for demonstration
        /// #![feature(allocator_api, btreemap_alloc)]
        /// # */
        /// # #[cfg(feature = "nightly-allocator-api")] fn main() {
        /// use bump_scope::Bump;
        /// use std::collections::{VecDeque, BTreeMap, LinkedList};
        ///
        /// let bump: Bump = Bump::new();
        /// let vec = Vec::new_in(&bump);
        /// let queue = VecDeque::new_in(&bump);
        /// let map = BTreeMap::new_in(&bump);
        /// let list = LinkedList::new_in(&bump);
        /// # let _: Vec<i32, _> = vec;
        /// # let _: VecDeque<i32, _> = queue;
        /// # let _: BTreeMap<i32, i32, _> = map;
        /// # let _: LinkedList<i32, _> = list;
        /// # }
        /// # #[cfg(not(feature = "nightly-allocator-api"))] fn main() {}
        /// ```
        ///
        /// [`alloc`]: BumpAllocatorTypedScope::alloc
        /// [`alloc_with`]: BumpAllocatorTypedScope::alloc_with
        /// [`alloc_default`]: BumpAllocatorTypedScope::alloc_default
        /// [`alloc_zeroed`]: crate::zerocopy_08::BumpAllocatorTypedScopeExt::alloc_zeroed
        ///
        /// [`alloc_str`]: BumpAllocatorTypedScope::alloc_str
        ///
        /// [`alloc_cstr`]: BumpAllocatorTypedScope::alloc_cstr
        /// [`alloc_cstr_from_str`]: BumpAllocatorTypedScope::alloc_cstr_from_str
        ///
        /// [`alloc_zeroed_slice`]: crate::zerocopy_08::BumpAllocatorTypedScopeExt::alloc_zeroed_slice
        ///
        /// [`alloc_iter`]: BumpAllocatorTypedScope::alloc_iter
        /// [`alloc_iter_exact`]: BumpAllocatorTypedScope::alloc_iter_exact
        /// [`alloc_iter_mut`]: MutBumpAllocatorTypedScope::alloc_iter_mut
        /// [`alloc_iter_mut_rev`]: MutBumpAllocatorTypedScope::alloc_iter_mut_rev
        ///
        /// [`alloc_uninit`]: BumpAllocatorTypedScope::alloc_uninit
        /// [`alloc_uninit_slice`]: BumpAllocatorTypedScope::alloc_uninit_slice
        /// [`alloc_uninit_slice_for`]: BumpAllocatorTypedScope::alloc_uninit_slice_for
        ///
        /// [`alloc_try_with`]: Bump::alloc_try_with
        /// [`alloc_try_with_mut`]: Bump::alloc_try_with_mut
        ///
        /// [`alloc_clone`]: BumpAllocatorTypedScope::alloc_clone
        ///
        /// [`scoped`]: crate::traits::BumpAllocator::scoped
        /// [`scoped_aligned`]: crate::traits::BumpAllocator::scoped_aligned
        /// [`scope_guard`]: crate::traits::BumpAllocator::scope_guard
        ///
        /// [`checkpoint`]: BumpAllocatorCore::checkpoint
        /// [`reset_to`]: BumpAllocatorCore::reset_to
        ///
        /// [`reset`]: Bump::reset
        /// [`dealloc`]: BumpAllocatorTyped::dealloc
        ///
        /// [`aligned`]: BumpAllocatorScope::aligned
        ///
        /// [`with_settings`]: Bump::with_settings
        /// [`borrow_with_settings`]: Bump::borrow_with_settings
        /// [`borrow_mut_with_settings`]: Bump::borrow_with_settings
        ///
        /// # Gotcha
        ///
        /// Having live allocations and entering bump scopes at the same time requires a `BumpScope`.
        /// This is due to the way lifetimes work, since `Bump` returns allocations with the lifetime
        /// of its own borrow instead of a separate lifetime like `BumpScope` does.
        ///
        /// So you can't do this:
        /// ```compile_fail,E0502
        /// # use bump_scope::Bump;
        /// let mut bump: Bump = Bump::new();
        ///
        /// let one = bump.alloc(1);
        ///
        /// bump.scoped(|bump| {
        ///     // whatever
        ///     # _ = bump;
        /// });
        /// # _ = one;
        /// ```
        /// But you can make the code work by converting the `Bump` it to a [`BumpScope`] first using [`as_mut_scope`]:
        /// ```
        /// # use bump_scope::Bump;
        /// let mut bump: Bump = Bump::new();
        /// let bump = bump.as_mut_scope();
        ///
        /// let one = bump.alloc(1);
        ///
        /// bump.scoped(|bump| {
        ///     // whatever
        ///     # _ = bump;
        /// });
        /// # _ = one;
        /// ```
        ///
        /// [`as_mut_scope`]: Bump::as_mut_scope
        #[repr(transparent)]
        pub struct Bump<$($allocator_parameter)*, S = BumpSettings>
        where
            A: Allocator,
            S: BumpAllocatorSettings,
        {
            pub(crate) raw: RawBump<A, S>,
        }
    };
}

maybe_default_allocator!(make_type);

// Sending Bumps when nothing is allocated is fine.
// When something is allocated Bump is borrowed and sending is not possible.
unsafe impl<A, S> Send for Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
}

impl<A, S> UnwindSafe for Bump<A, S>
where
    A: Allocator + UnwindSafe,
    S: BumpAllocatorSettings,
{
}

impl<A, S> RefUnwindSafe for Bump<A, S>
where
    A: Allocator + RefUnwindSafe,
    S: BumpAllocatorSettings,
{
}

impl<A, S> Drop for Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    fn drop(&mut self) {
        unsafe { self.raw.manually_drop() }
    }
}

impl<A, S> Debug for Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(self.stats()).debug_format("Bump", f)
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<A, S> Default for Bump<A, S>
where
    A: Allocator + Default,
    S: BumpAllocatorSettings,
{
    /// With [`GUARANTEED_ALLOCATED`] this does the same as [`new`], otherwise it does the same as [`unallocated`].
    ///
    /// [`GUARANTEED_ALLOCATED`]: crate::settings
    /// [`new`]: Bump::new
    /// [`unallocated`]: Bump::unallocated
    #[inline(always)]
    fn default() -> Self {
        if S::GUARANTEED_ALLOCATED {
            Self::new_in(Default::default())
        } else {
            use core::{cell::Cell, marker::PhantomData};

            use crate::{chunk::ChunkHeader, raw_bump::RawChunk};

            Self {
                raw: RawBump {
                    chunk: Cell::new(RawChunk {
                        header: ChunkHeader::unallocated::<S>().cast(),
                        marker: PhantomData,
                    }),
                },
            }
        }
    }
}

#[cfg(not(feature = "panic-on-alloc"))]
impl<A, S> Default for Bump<A, S>
where
    A: Allocator + Default,
    S: BumpAllocatorSettings<GuaranteedAllocated = False>,
{
    /// Does the same as [`unallocated`].
    ///
    /// [`unallocated`]: Bump::unallocated
    #[inline(always)]
    fn default() -> Self {
        Self::unallocated()
    }
}

impl<A, S> Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings<GuaranteedAllocated = False>,
{
    /// Constructs a new `Bump` without allocating a chunk.
    ///
    /// This requires the `GUARANTEED_ALLOCATED` setting to be `false`, see [`settings`].
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{
    ///     alloc::Global,
    ///     Bump,
    ///     settings::{BumpSettings, BumpAllocatorSettings}
    /// };
    ///
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    ///
    /// let bump: Bump<Global, Settings> = Bump::unallocated();
    /// # _ = bump;
    /// ```
    ///
    /// [`settings`]: crate::settings
    #[must_use]
    pub const fn unallocated() -> Self {
        Self { raw: RawBump::new() }
    }
}

/// Methods for a `Bump` with a default base allocator.
impl<A, S> Bump<A, S>
where
    A: Allocator + Default,
    S: BumpAllocatorSettings,
{
    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[with_size][]([MINIMUM_CHUNK_SIZE])</code>.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    ///
    /// let bump: Bump = Bump::new();
    /// # _ = bump;
    /// ```
    ///
    /// [with_size]: Bump::with_size
    /// [MINIMUM_CHUNK_SIZE]: crate::settings
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn new() -> Self {
        Self::with_size(S::MINIMUM_CHUNK_SIZE)
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[try_with_size][]([MINIMUM_CHUNK_SIZE])</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    ///
    /// let bump: Bump = Bump::try_new()?;
    /// # _ = bump;
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [try_with_size]: Bump::try_with_size
    /// [MINIMUM_CHUNK_SIZE]: crate::settings
    #[inline(always)]
    pub fn try_new() -> Result<Self, AllocError> {
        Self::try_with_size(S::MINIMUM_CHUNK_SIZE)
    }

    /// Constructs a new `Bump` with a size hint for the first chunk.
    ///
    /// If you want to ensure a specific capacity, use [`with_capacity`](Self::with_capacity) instead.
    ///
    /// An effort is made to ensure the size requested from the base allocator is friendly to an allocator that uses size classes and stores metadata alongside allocations.
    /// To achieve this, the requested size is rounded up to either the next power of two or the next multiple of `0x1000`, whichever is smaller.
    /// After that, the size of `[usize; 2]` is subtracted.
    ///
    /// If the base allocator returns a memory block that is larger than requested, then the chunk will use the extra space.
    ///
    /// **Disclaimer:** The way in which the chunk layout is calculated might change.
    /// Such a change is not considered semver breaking.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    ///
    /// // `Bump` with a roughly 1 Mebibyte sized chunk
    /// let bump_1mib: Bump = Bump::with_size(1024 * 1024);
    /// # _ = bump_1mib;
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_size(size: usize) -> Self {
        panic_on_error(Self::generic_with_size(size))
    }

    /// Constructs a new `Bump` with a size hint for the first chunk.
    ///
    /// If you want to ensure a specific capacity, use [`try_with_capacity`](Self::try_with_capacity) instead.
    ///
    /// An effort is made to ensure the size requested from the base allocator is friendly to an allocator that uses size classes and stores metadata alongside allocations.
    /// To achieve this, the requested size is rounded up to either the next power of two or the next multiple of `0x1000`, whichever is smaller.
    /// After that, the size of `[usize; 2]` is subtracted.
    ///
    /// If the base allocator returns a memory block that is larger than requested, then the chunk will use the extra space.
    ///
    /// **Disclaimer:** The way in which the chunk layout is calculated might change.
    /// Such a change is not considered semver breaking.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    ///
    /// // `Bump` with a roughly 1 Mebibyte sized chunk
    /// let bump_1mib: Bump = Bump::try_with_size(1024 * 1024)?;
    /// # _ = bump_1mib;
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_size(size: usize) -> Result<Self, AllocError> {
        Self::generic_with_size(size)
    }

    #[inline]
    pub(crate) fn generic_with_size<E: ErrorBehavior>(size: usize) -> Result<Self, E> {
        Self::generic_with_size_in(size, Default::default())
    }

    /// Constructs a new `Bump` with a chunk that has at least enough space for `layout`.
    ///
    /// To construct a `Bump` with a size hint use <code>[with_size](Bump::with_size)</code> instead.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    /// use core::alloc::Layout;
    ///
    /// let layout = Layout::array::<u8>(1234).unwrap();
    /// let bump: Bump = Bump::with_capacity(layout);
    /// assert!(bump.stats().capacity() >= layout.size());
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_capacity(layout: Layout) -> Self {
        panic_on_error(Self::generic_with_capacity(layout))
    }

    /// Constructs a new `Bump` with a chunk that has at least enough space for `layout`.
    ///
    /// To construct a `Bump` with a size hint use <code>[try_with_size](Bump::try_with_size)</code> instead.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    /// use core::alloc::Layout;
    ///
    /// let layout = Layout::array::<u8>(1234).unwrap();
    /// let bump: Bump = Bump::try_with_capacity(layout)?;
    /// assert!(bump.stats().capacity() >= layout.size());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_capacity(layout: Layout) -> Result<Self, AllocError> {
        Self::generic_with_capacity(layout)
    }

    #[inline]
    pub(crate) fn generic_with_capacity<E: ErrorBehavior>(layout: Layout) -> Result<Self, E> {
        Self::generic_with_capacity_in(layout, Default::default())
    }
}

/// Methods that are always available.
impl<A, S> Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[with_size_in][]([MINIMUM_CHUNK_SIZE], allocator)</code>.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    /// use bump_scope::alloc::Global;
    ///
    /// let bump: Bump = Bump::new_in(Global);
    /// # _ = bump;
    /// ```
    ///
    /// [with_size_in]: Bump::with_size_in
    /// [MINIMUM_CHUNK_SIZE]: crate::settings
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn new_in(allocator: A) -> Self {
        Self::with_size_in(S::MINIMUM_CHUNK_SIZE, allocator)
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[try_with_size_in][]([MINIMUM_CHUNK_SIZE], allocator)</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    /// use bump_scope::alloc::Global;
    ///
    /// let bump: Bump = Bump::try_new_in(Global)?;
    /// # _ = bump;
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [try_with_size_in]: Bump::try_with_size_in
    /// [MINIMUM_CHUNK_SIZE]: crate::settings
    #[inline(always)]
    pub fn try_new_in(allocator: A) -> Result<Self, AllocError> {
        Self::try_with_size_in(S::MINIMUM_CHUNK_SIZE, allocator)
    }

    /// Constructs a new `Bump` with a size hint for the first chunk.
    ///
    /// If you want to ensure a specific capacity, use [`with_capacity_in`](Self::with_capacity_in) instead.
    ///
    /// An effort is made to ensure the size requested from the base allocator is friendly to an allocator that uses size classes and stores metadata alongside allocations.
    /// To achieve this, the requested size is rounded up to either the next power of two or the next multiple of `0x1000`, whichever is smaller.
    /// After that, the size of `[usize; 2]` is subtracted.
    ///
    /// If the base allocator returns a memory block that is larger than requested, then the chunk will use the extra space.
    ///
    /// **Disclaimer:** The way in which the chunk layout is calculated might change.
    /// Such a change is not considered semver breaking.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    /// use bump_scope::alloc::Global;
    ///
    /// // `Bump` with a roughly 1 Mebibyte sized chunk
    /// let bump_1mib: Bump = Bump::with_size_in(1024 * 1024, Global);
    /// # _ = bump_1mib;
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_size_in(size: usize, allocator: A) -> Self {
        panic_on_error(Self::generic_with_size_in(size, allocator))
    }

    /// Constructs a new `Bump` with a size hint for the first chunk.
    ///
    /// If you want to ensure a specific capacity, use [`try_with_capacity`](Self::try_with_capacity) instead.
    ///
    /// An effort is made to ensure the size requested from the base allocator is friendly to an allocator that uses size classes and stores metadata alongside allocations.
    /// To achieve this, the requested size is rounded up to either the next power of two or the next multiple of `0x1000`, whichever is smaller.
    /// After that, the size of `[usize; 2]` is subtracted.
    ///
    /// If the base allocator returns a memory block that is larger than requested, then the chunk will use the extra space.
    ///
    /// **Disclaimer:** The way in which the chunk layout is calculated might change.
    /// Such a change is not considered semver breaking.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    /// use bump_scope::alloc::Global;
    ///
    /// // `Bump` with a roughly 1 Mebibyte sized chunk
    /// let bump_1mib: Bump = Bump::try_with_size_in(1024 * 1024, Global)?;
    /// # _ = bump_1mib;
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_size_in(size: usize, allocator: A) -> Result<Self, AllocError> {
        Self::generic_with_size_in(size, allocator)
    }

    #[inline]
    pub(crate) fn generic_with_size_in<E: ErrorBehavior>(size: usize, allocator: A) -> Result<Self, E> {
        Ok(Self {
            raw: RawBump::with_size(
                ChunkSize::<A, S>::from_hint(size).ok_or_else(E::capacity_overflow)?,
                allocator,
            )?,
        })
    }

    /// Constructs a new `Bump` with at least enough space for `layout`.
    ///
    /// To construct a `Bump` with a size hint use <code>[with_size_in](Bump::with_size_in)</code> instead.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    /// use bump_scope::alloc::Global;
    /// use core::alloc::Layout;
    ///
    /// let layout = Layout::array::<u8>(1234).unwrap();
    /// let bump: Bump = Bump::with_capacity_in(layout, Global);
    /// assert!(bump.stats().capacity() >= layout.size());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_capacity_in(layout: Layout, allocator: A) -> Self {
        panic_on_error(Self::generic_with_capacity_in(layout, allocator))
    }

    /// Constructs a new `Bump` with at least enough space for `layout`.
    ///
    /// To construct a `Bump` with a size hint use <code>[try_with_size_in](Bump::try_with_size_in)</code> instead.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::Bump;
    /// use bump_scope::alloc::Global;
    /// use core::alloc::Layout;
    ///
    /// let layout = Layout::array::<u8>(1234).unwrap();
    /// let bump: Bump = Bump::try_with_capacity_in(layout, Global)?;
    /// assert!(bump.stats().capacity() >= layout.size());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_capacity_in(layout: Layout, allocator: A) -> Result<Self, AllocError> {
        Self::generic_with_capacity_in(layout, allocator)
    }

    #[inline]
    pub(crate) fn generic_with_capacity_in<E: ErrorBehavior>(layout: Layout, allocator: A) -> Result<Self, E> {
        Ok(Self {
            raw: RawBump::with_size(
                ChunkSize::<A, S>::from_capacity(layout).ok_or_else(E::capacity_overflow)?,
                allocator,
            )?,
        })
    }

    /// Resets this bump allocator and deallocates all but the largest chunk.
    ///
    /// This deallocates all allocations at once by resetting
    /// the bump pointer to the start of the retained chunk.
    ///
    /// For a version of this function that doesn't deallocate chunks, see [`reset_to_start`].
    ///
    /// [`reset_to_start`]: Self::reset_to_start
    ///
    /// ```
    /// use bump_scope::Bump;
    ///
    /// let mut bump: Bump = Bump::with_size(512);
    ///
    /// // won't fit in the first chunk
    /// bump.alloc_uninit_slice::<u8>(600);
    ///
    /// let chunks = bump.stats().small_to_big().collect::<Vec<_>>();
    /// assert_eq!(chunks.len(), 2);
    /// assert!(chunks[0].size() < chunks[1].size());
    /// assert_eq!(chunks[0].allocated(), 0);
    /// assert_eq!(chunks[1].allocated(), 600);
    /// let last_chunk_size = chunks[1].size();
    ///
    /// bump.reset();
    ///
    /// let chunks = bump.stats().small_to_big().collect::<Vec<_>>();
    /// assert_eq!(chunks.len(), 1);
    /// assert_eq!(chunks[0].size(), last_chunk_size);
    /// assert_eq!(chunks[0].allocated(), 0);
    /// ```
    #[inline(always)]
    pub fn reset(&mut self) {
        self.raw.reset();
    }

    /// Resets this bump allocator.
    ///
    /// This deallocates all allocations at once by resetting
    /// the bump pointer to the start of the first chunk.
    ///
    /// For a version of this function that also deallocates chunks, see [`reset`].
    ///
    /// [`reset`]: Self::reset
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.stats().allocated(), 5);
    ///     # _ = hello;
    /// }
    ///
    /// unsafe { bump.reset_to_start(); }
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    pub fn reset_to_start(&mut self) {
        self.raw.reset_to_start();
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'_, A, S> {
        self.as_scope().stats()
    }

    /// Returns this `&Bump` as a `&BumpScope`.
    #[must_use]
    #[inline(always)]
    pub fn as_scope(&self) -> &BumpScope<'_, A, S> {
        unsafe { transmute_ref(self) }
    }

    /// Returns this `&mut Bump` as a `&mut BumpScope`.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut BumpScope<'_, A, S> {
        unsafe { transmute_mut(self) }
    }

    /// Converts this `Bump` into a `Bump` with new settings.
    ///
    /// This function will fail to compile if:
    /// - `NewS::UP != S::UP`
    ///
    /// # Panics
    /// Panics if `!NewS::CLAIMABLE` and the bump allocator is currently [claimed].
    ///
    /// Panics if `NewS::GUARANTEED_ALLOCATED` and no chunk has been allocated.
    ///
    /// [claimed]: crate::traits::BumpAllocatorScope::claim
    #[inline]
    pub fn with_settings<NewS>(self) -> Bump<A, NewS>
    where
        A: BaseAllocator<NewS::GuaranteedAllocated>,
        NewS: BumpAllocatorSettings,
    {
        self.raw.ensure_satisfies_settings::<NewS>();
        unsafe { transmute_value(self) }
    }

    /// Borrows this `Bump` with new settings.
    ///
    /// This function will fail to compile if:
    /// - `NewS::MIN_ALIGN != S::MIN_ALIGN`
    /// - `NewS::UP != S::UP`
    /// - `NewS::CLAIMABLE != S::CLAIMABLE`
    /// - `NewS::GUARANTEED_ALLOCATED > S::GUARANTEED_ALLOCATED`
    #[inline]
    pub fn borrow_with_settings<NewS>(&self) -> &Bump<A, NewS>
    where
        A: BaseAllocator<NewS::GuaranteedAllocated>,
        NewS: BumpAllocatorSettings,
    {
        self.raw.ensure_satisfies_settings_for_borrow::<NewS>();
        unsafe { transmute_ref(self) }
    }

    /// Borrows this `Bump` mutably with new settings.
    ///
    /// This function will fail to compile if:
    /// - `NewS::MIN_ALIGN < S::MIN_ALIGN`
    /// - `NewS::UP != S::UP`
    /// - `NewS::GUARANTEED_ALLOCATED != S::GUARANTEED_ALLOCATED`
    /// - `NewS::CLAIMABLE != S::CLAIMABLE`
    #[inline]
    pub fn borrow_mut_with_settings<NewS>(&mut self) -> &mut Bump<A, NewS>
    where
        A: BaseAllocator<NewS::GuaranteedAllocated>,
        NewS: BumpAllocatorSettings,
    {
        self.raw.ensure_satisfies_settings_for_borrow_mut::<NewS>();
        unsafe { transmute_mut(self) }
    }

    /// Converts this `Bump` into a raw pointer.
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// #
    /// let bump: Bump = Bump::new();
    /// bump.alloc_str("Hello, ");
    ///
    /// let ptr = bump.into_raw();
    /// let bump: Bump = unsafe { Bump::from_raw(ptr) };
    ///
    /// bump.alloc_str("World!");
    /// # assert_eq!(bump.stats().allocated(), 13);
    /// ```
    #[inline]
    #[must_use]
    pub fn into_raw(self) -> NonNull<()> {
        ManuallyDrop::new(self).raw.clone().into_raw()
    }

    /// Converts the raw pointer that was created with [`into_raw`](Bump::into_raw) back into a `Bump`.
    ///
    /// # Safety
    /// - `ptr` must come from a call to `Self::into_raw`.
    /// - This function must only be called once with this `ptr`.
    /// - The settings must match the original ones.
    #[inline]
    #[must_use]
    pub unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        Self {
            raw: unsafe { RawBump::from_raw(ptr) },
        }
    }
}

impl<'b, A, S> From<&'b Bump<A, S>> for &'b BumpScope<'b, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn from(value: &'b Bump<A, S>) -> Self {
        value.as_scope()
    }
}

impl<'b, A, S> From<&'b mut Bump<A, S>> for &'b mut BumpScope<'b, A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn from(value: &'b mut Bump<A, S>) -> Self {
        value.as_mut_scope()
    }
}

/// Methods that forward to traits.
// Documentation is in the forwarded to methods.
#[allow(clippy::missing_errors_doc, clippy::missing_safety_doc)]
impl<A, S> Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    traits::forward_methods! {
        self: self
        access: {self.as_scope()}
        access_mut: {self.as_mut_scope()}
        lifetime: '_
    }
}

/// Additional `alloc` methods that are not available in traits.
impl<A, S> Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
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
    pub fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpBox<'_, T>, E> {
        self.as_scope().alloc_try_with(f)
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
    ) -> Result<Result<BumpBox<'_, T>, E>, AllocError> {
        self.as_scope().try_alloc_try_with(f)
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
    pub fn alloc_try_with_mut<T, E>(&mut self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpBox<'_, T>, E> {
        self.as_mut_scope().alloc_try_with_mut(f)
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
    ) -> Result<Result<BumpBox<'_, T>, E>, AllocError> {
        self.as_mut_scope().try_alloc_try_with_mut(f)
    }
}
