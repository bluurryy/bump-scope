use core::{
    alloc::{AllocError, Allocator, Layout},
    ffi::CStr,
    fmt::{self, Debug},
    mem::{ManuallyDrop, MaybeUninit},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

#[cfg(feature = "nightly-clone-to-uninit")]
use core::clone::CloneToUninit;

use crate::{
    BumpBox, BumpClaimGuard, BumpScope, BumpScopeGuard, Checkpoint, allocator_impl,
    chunk_size::{ChunkSizeCapacity, ChunkSizeHint, ChunkSizeMin},
    config::{BumpConfig, DEFAULT_BUMP_CONFIG, config},
    error_behavior, maybe_default_allocator,
    owned_slice::OwnedSlice,
    polyfill::{transmute_mut, transmute_ref, transmute_value},
    raw_bump::RawBump,
    stats::{AnyStats, Stats},
    traits::{self, BumpAllocator, BumpAllocatorCore, BumpAllocatorScope, MutBumpAllocatorScope},
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

macro_rules! make_type {
    ($($allocator_parameter:tt)*) => {
        /// The bump allocator.
        ///
        /// # Generic parameters
        /// - **`A`** — the base allocator, defaults to `Global` when the `alloc` feature is enabled
        /// - **`C`** — the bump allocator config, see [`config`](crate::config)
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
        /// - strings: [`alloc_str`], <code>[alloc_fmt](BumpAllocatorScope::alloc_fmt)([_mut](MutBumpAllocatorScope::alloc_fmt_mut))</code>
        /// - c strings: [`alloc_cstr`], [`alloc_cstr_from_str`], <code>[alloc_cstr_fmt](BumpAllocatorScope::alloc_cstr_fmt)([_mut](MutBumpAllocatorScope::alloc_cstr_fmt_mut))</code>
        /// - slices: <code>alloc_slice_{[copy](BumpAllocatorScope::alloc_slice_copy), [clone](BumpAllocatorScope::alloc_slice_clone), [move](BumpAllocatorScope::alloc_slice_move), [fill](BumpAllocatorScope::alloc_slice_fill), [fill_with](BumpAllocatorScope::alloc_slice_fill_with)}</code>,
        ///   [`alloc_zeroed_slice`]
        /// - slices from an iterator: [`alloc_iter`], [`alloc_iter_exact`], [`alloc_iter_mut`], [`alloc_iter_mut_rev`]
        /// - uninitialized values: [`alloc_uninit`], [`alloc_uninit_slice`], [`alloc_uninit_slice_for`]
        ///
        ///   which can then be conveniently initialized by the [`init*` methods of `BumpBox`](crate::BumpBox#bumpbox-has-a-lot-of-methods).
        /// - results: [`alloc_try_with`], [`alloc_try_with_mut`]
        /// - via clone *(nightly only)*: [`alloc_clone`]
        ///
        /// #### Free memory using ...
        /// - scopes: [`scoped`], [`scope_guard`], [`scoped_with_config`]
        /// - checkpoints: [`checkpoint`], [`reset_to`]
        /// - reset: [`reset`], [`reset_to_start`]
        /// - dealloc: [`dealloc`]
        ///
        /// #### Configure allocator config ...
        /// - [`with_config`], [`change_config`], [`borrow_change_config`], [`borrow_mut_change_config`]
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
        /// *At the time of writing (2026-10-10) this requires hashbrown's `nightly` feature.*
        // NOTE: This code is tested in `crates/test-hashbrown/lib.rs`.
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
        /// On stable you can allocate collections like `Box` and `Vec`.
        /// ```
        /// use bump_scope::Bump;
        ///
        /// let bump: Bump = Bump::new();
        /// let boxed = Box::new_in(123, &bump);
        /// let vec = Vec::new_in(&bump);
        /// # let _: Box<i32, _> = boxed;
        /// # let _: Vec<i32, _> = vec;
        /// ```
        ///
        /// With the nightly feature `allocator_ext` you can allocate more collections from `std`:
        /// ```
        /// # /*
        /// # those features are already been enabled by a `doc(test(attr`
        /// # but we still want it here for demonstration
        /// #![feature(allocator_ext)]
        /// # */
        /// # #[cfg(feature = "nightly-tests")]
        /// # fn main() {
        /// use bump_scope::Bump;
        /// # // use std::collections::{VecDeque, BTreeMap, LinkedList};
        /// use std::collections::{VecDeque, LinkedList};
        ///
        /// let bump: Bump = Bump::new();
        /// let queue = VecDeque::new_in(&bump);
        /// # /* FIXME: uncomment once BTreeMap uses allocator_ext instead of btreemap_alloc feature
        /// let map = BTreeMap::new_in(&bump);
        /// # */
        /// let list = LinkedList::new_in(&bump);
        /// # let _: VecDeque<i32, _> = queue;
        /// # // let _: BTreeMap<i32, i32, _> = map;
        /// # let _: LinkedList<i32, _> = list;
        /// # }
        /// # #[cfg(not(feature = "nightly-tests"))] fn main() {}
        /// ```
        ///
        /// [`alloc`]: BumpAllocatorScope::alloc
        /// [`alloc_with`]: BumpAllocatorScope::alloc_with
        /// [`alloc_default`]: BumpAllocatorScope::alloc_default
        /// [`alloc_zeroed`]: crate::zerocopy_08::BumpAllocatorScopeExt::alloc_zeroed
        ///
        /// [`alloc_str`]: BumpAllocatorScope::alloc_str
        ///
        /// [`alloc_cstr`]: BumpAllocatorScope::alloc_cstr
        /// [`alloc_cstr_from_str`]: BumpAllocatorScope::alloc_cstr_from_str
        ///
        /// [`alloc_zeroed_slice`]: crate::zerocopy_08::BumpAllocatorScopeExt::alloc_zeroed_slice
        ///
        /// [`alloc_iter`]: BumpAllocatorScope::alloc_iter
        /// [`alloc_iter_exact`]: BumpAllocatorScope::alloc_iter_exact
        /// [`alloc_iter_mut`]: MutBumpAllocatorScope::alloc_iter_mut
        /// [`alloc_iter_mut_rev`]: MutBumpAllocatorScope::alloc_iter_mut_rev
        ///
        /// [`alloc_uninit`]: BumpAllocatorScope::alloc_uninit
        /// [`alloc_uninit_slice`]: BumpAllocatorScope::alloc_uninit_slice
        /// [`alloc_uninit_slice_for`]: BumpAllocatorScope::alloc_uninit_slice_for
        ///
        /// [`alloc_try_with`]: Bump::alloc_try_with
        /// [`alloc_try_with_mut`]: Bump::alloc_try_with_mut
        ///
        /// [`alloc_clone`]: BumpAllocatorScope::alloc_clone
        ///
        /// [`scoped`]: crate::BumpScope::scoped
        /// [`scoped_with_config`]: crate::BumpScope::scoped_with_config
        /// [`scope_guard`]: crate::BumpScope::scope_guard
        ///
        /// [`checkpoint`]: BumpAllocatorCore::checkpoint
        /// [`reset_to`]: BumpAllocatorCore::reset_to
        ///
        /// [`reset`]: Bump::reset
        /// [`reset_to_start`]: Bump::reset_to_start
        /// [`dealloc`]: BumpAllocator::dealloc
        ///
        /// [`with_config`]: Bump::with_config
        /// [`change_config`]: Bump::change_config
        /// [`borrow_change_config`]: Bump::borrow_change_config
        /// [`borrow_mut_change_config`]: Bump::borrow_change_config
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
        pub struct Bump<$($allocator_parameter)*, const C: BumpConfig = DEFAULT_BUMP_CONFIG>
        where
            A: Allocator,

        {
            pub(crate) raw: RawBump<A, C>,
        }
    };
}

maybe_default_allocator!(make_type);

// Sending Bumps when nothing is allocated is fine.
// When something is allocated Bump is borrowed and sending is not possible.
unsafe impl<A, const C: BumpConfig> Send for Bump<A, C> where A: Send + Allocator {}

impl<A, const C: BumpConfig> UnwindSafe for Bump<A, C> where A: Allocator + UnwindSafe {}

impl<A, const C: BumpConfig> RefUnwindSafe for Bump<A, C> where A: Allocator + RefUnwindSafe {}

impl<A, const C: BumpConfig> Drop for Bump<A, C>
where
    A: Allocator,
{
    fn drop(&mut self) {
        unsafe { self.raw.manually_drop() }
    }
}

impl<A, const C: BumpConfig> Debug for Bump<A, C>
where
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(self.stats()).debug_format("Bump", f)
    }
}

impl<A, const C: BumpConfig> Default for Bump<A, C>
where
    A: Allocator + Default + 'static,
{
    /// With [`const_constructible`] this does the same as [`unallocated`], otherwise it does the same as [`new`].
    ///
    /// # Panics
    /// Panics if this `Bump` is **not** [`const_constructible`] and the "panic-on-alloc" feature is disabled.
    ///
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    /// [`new`]: Bump::new
    /// [`unallocated`]: Bump::unallocated
    #[inline(always)]
    fn default() -> Self {
        if config!(const_constructible) {
            Self { raw: RawBump::new() }
        } else {
            cfg_select! {
                feature = "panic-on-alloc" => Self::new(),
                _ => panic!("can't default construct a `Bump` without `const_constructible` and no \"panic-on-alloc\" feature"),
            }
        }
    }
}

impl<A, const C: BumpConfig> Bump<A, C>
where
    A: Allocator + Default + 'static,
{
    /// Constructs a new `Bump` without allocating a chunk.
    ///
    /// # Panics
    ///
    /// Panics when not [`const_constructible`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::alloc::Global;
    ///
    /// use bump_scope::{Bump, config::{BumpConfig, BumpConfigBuilder}};
    ///
    /// const CONFIG: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
    ///
    /// let bump: Bump<Global, CONFIG> = Bump::unallocated();
    /// # _ = bump;
    /// ```
    ///
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    #[must_use]
    pub const fn unallocated() -> Self {
        const { assert!(config!(const_constructible)) };
        Self { raw: RawBump::new() }
    }
}

/// Methods for a `Bump` with a default base allocator.
impl<A, const C: BumpConfig> Bump<A, C>
where
    A: Allocator + Default,
{
    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[with_size][]([min_chunk_size])</code>.
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
    /// [min_chunk_size]: crate::config::BumpConfigBuilder::min_chunk_size
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn new() -> Self {
        Self::new_in(A::default())
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[try_with_size][]([min_chunk_size])</code>.
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
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [try_with_size]: Bump::try_with_size
    /// [min_chunk_size]: crate::config::BumpConfigBuilder::min_chunk_size
    #[inline(always)]
    pub fn try_new() -> Result<Self, AllocError> {
        Self::try_new_in(A::default())
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
        Self::with_size_in(size, A::default())
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
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_size(size: usize) -> Result<Self, AllocError> {
        Self::try_with_size_in(size, A::default())
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
        Self::with_capacity_in(layout, A::default())
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
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_capacity(layout: Layout) -> Result<Self, AllocError> {
        Self::try_with_capacity_in(layout, A::default())
    }
}

/// Methods that are always available.
impl<A, const C: BumpConfig> Bump<A, C>
where
    A: Allocator,
{
    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[with_size_in][]([min_chunk_size], allocator)</code>.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use std::alloc::Global;
    /// use bump_scope::Bump;
    ///
    /// let bump: Bump = Bump::new_in(Global);
    /// # _ = bump;
    /// ```
    ///
    /// [with_size_in]: Bump::with_size_in
    /// [min_chunk_size]: crate::config::BumpConfigBuilder::min_chunk_size
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn new_in(allocator: A) -> Self {
        Self {
            raw: panic_on_error(RawBump::with_size(ChunkSizeMin, allocator)),
        }
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[try_with_size_in][]([min_chunk_size], allocator)</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use std::alloc::Global;
    /// use bump_scope::Bump;
    ///
    /// let bump: Bump = Bump::try_new_in(Global)?;
    /// # _ = bump;
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [try_with_size_in]: Bump::try_with_size_in
    /// [min_chunk_size]: crate::config::BumpConfigBuilder::min_chunk_size
    #[inline(always)]
    pub fn try_new_in(allocator: A) -> Result<Self, AllocError> {
        Ok(Self {
            raw: RawBump::with_size(ChunkSizeMin, allocator)?,
        })
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
    /// use std::alloc::Global;
    /// use bump_scope::Bump;
    ///
    /// // `Bump` with a roughly 1 Mebibyte sized chunk
    /// let bump_1mib: Bump = Bump::with_size_in(1024 * 1024, Global);
    /// # _ = bump_1mib;
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_size_in(size: usize, allocator: A) -> Self {
        Self {
            raw: panic_on_error(RawBump::with_size(ChunkSizeHint(size), allocator)),
        }
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
    /// use std::alloc::Global;
    /// use bump_scope::Bump;
    ///
    /// // `Bump` with a roughly 1 Mebibyte sized chunk
    /// let bump_1mib: Bump = Bump::try_with_size_in(1024 * 1024, Global)?;
    /// # _ = bump_1mib;
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_size_in(size: usize, allocator: A) -> Result<Self, AllocError> {
        Ok(Self {
            raw: RawBump::with_size(ChunkSizeHint(size), allocator)?,
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
    /// use std::alloc::{Global, Layout};
    /// use bump_scope::Bump;
    ///
    /// let layout = Layout::array::<u8>(1234).unwrap();
    /// let bump: Bump = Bump::with_capacity_in(layout, Global);
    /// assert!(bump.stats().capacity() >= layout.size());
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn with_capacity_in(layout: Layout, allocator: A) -> Self {
        Self {
            raw: panic_on_error(RawBump::with_size(ChunkSizeCapacity(layout), allocator)),
        }
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
    /// use std::alloc::{Global, Layout};
    ///
    /// use bump_scope::Bump;
    ///
    /// let layout = Layout::array::<u8>(1234).unwrap();
    /// let bump: Bump = Bump::try_with_capacity_in(layout, Global)?;
    /// assert!(bump.stats().capacity() >= layout.size());
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_with_capacity_in(layout: Layout, allocator: A) -> Result<Self, AllocError> {
        Ok(Self {
            raw: RawBump::with_size(ChunkSizeCapacity(layout), allocator)?,
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
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut scratch_allocator: Bump = Bump::new();
    /// let items = [1, 2, 3];
    ///
    /// for item in items {
    ///     // the scratch allocator is empty
    ///     assert_eq!(scratch_allocator.stats().allocated(), 0);
    ///
    ///     // allocate on scratch allocator...
    ///     let _ = scratch_allocator.alloc_fmt(format_args!("hello {item}"));
    ///
    ///     // reset the scratch allocator for the next iteration
    ///     scratch_allocator.reset();
    /// }
    /// ```
    ///
    /// Detailed behavior:
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::with_size(512);
    ///
    /// // won't fit in the first chunk, will be allocated in another chunk
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
    /// Basic usage:
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut scratch_allocator: Bump = Bump::new();
    /// let items = [1, 2, 3];
    ///
    /// for item in items {
    ///     // the scratch allocator is empty
    ///     assert_eq!(scratch_allocator.stats().allocated(), 0);
    ///
    ///     // allocate on scratch allocator...
    ///     let _ = scratch_allocator.alloc_fmt(format_args!("hello {item}"));
    ///
    ///     // reset the scratch allocator for the next iteration
    ///     scratch_allocator.reset_to_start();
    /// }
    /// ```
    ///
    /// Detailed behavior:
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::with_size(512);
    ///
    /// // won't fit in the first chunk, will be allocated in another chunk
    /// bump.alloc_uninit_slice::<u8>(600);
    ///
    /// let chunks = bump.stats().small_to_big().collect::<Vec<_>>();
    /// assert_eq!(chunks.len(), 2);
    /// assert!(chunks[0].size() < chunks[1].size());
    /// assert_eq!(chunks[0].allocated(), 0);
    /// assert_eq!(chunks[1].allocated(), 600);
    ///
    /// bump.reset_to_start();
    ///
    /// // we still have two chunks...
    /// assert_eq!(bump.stats().count(), 2);
    ///
    /// // ... but everything is deallocated
    /// assert_eq!(bump.stats().allocated(), 0);
    ///
    /// // we are back at the first chunk
    /// assert!(bump.stats().current_chunk().unwrap().prev().is_none());
    /// ```
    #[inline(always)]
    pub fn reset_to_start(&mut self) {
        self.raw.reset_to_start();
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'_, A, C> {
        self.as_scope().stats()
    }

    /// Returns this `&Bump` as a `&BumpScope`.
    #[must_use]
    #[inline(always)]
    pub fn as_scope(&self) -> &BumpScope<'_, A, C> {
        unsafe { transmute_ref(self) }
    }

    /// Returns this `&mut Bump` as a `&mut BumpScope`.
    #[must_use]
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut BumpScope<'_, A, C> {
        unsafe { transmute_mut(self) }
    }

    /// Converts this `Bump` into a `Bump` with a new config.
    ///
    /// # Panics
    /// - Panics if [`claimable`] becomes false and [`self.is_claimed()`]
    /// - Panics if [`const_constructible`] becomes false and `self` has no chunks
    ///
    /// # Fails
    /// - Fails to compile if [`upwards`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`self.is_claimed()`]: Self::is_claimed
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    #[inline]
    pub fn change_config<const NEW_C: BumpConfig>(self) -> Bump<A, NEW_C> {
        RawBump::<A, C>::const_assert_upwards_matches::<NEW_C>();

        if !self.raw.can_convert_claimable::<NEW_C>() {
            error_behavior::panic::claimed();
        }

        if !self.raw.can_convert_const_constructible::<NEW_C>() {
            error_behavior::panic::unallocated();
        }

        self.raw.align_to(config!(NEW_C, min_align));

        unsafe { transmute_value(self) }
    }

    /// Borrows this `Bump` with a new config.
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
    pub fn borrow_change_config<const NEW_C: BumpConfig>(&self) -> &Bump<A, NEW_C> {
        self.raw.ensure_satisfies_config_for_borrow::<NEW_C>();
        unsafe { transmute_ref(self) }
    }

    /// Borrows this `Bump` mutably with a new config.
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
    pub fn borrow_mut_change_config<const NEW_C: BumpConfig>(&mut self) -> &mut Bump<A, NEW_C> {
        self.raw.ensure_satisfies_config_for_borrow_mut::<NEW_C>();
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
    /// - The config must match the original one.
    #[inline]
    #[must_use]
    pub unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        Self {
            raw: unsafe { RawBump::from_raw(ptr) },
        }
    }
}

impl<'b, A, const C: BumpConfig> From<&'b Bump<A, C>> for &'b BumpScope<'b, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    fn from(value: &'b Bump<A, C>) -> Self {
        value.as_scope()
    }
}

impl<'b, A, const C: BumpConfig> From<&'b mut Bump<A, C>> for &'b mut BumpScope<'b, A, C>
where
    A: Allocator,
{
    #[inline(always)]
    fn from(value: &'b mut Bump<A, C>) -> Self {
        value.as_mut_scope()
    }
}

/// Methods that forward to traits.
// Documentation is in the forwarded to methods.
#[allow(clippy::missing_errors_doc, clippy::missing_safety_doc)]
impl<A, const C: BumpConfig> Bump<A, C>
where
    A: Allocator,
{
    traits::forward_methods! {
        self: self
        access: {self.as_scope()}
        access_mut: {self.as_mut_scope()}
        lifetime: '_
    }
}

/// Additional `alloc` methods that are not available in traits.
impl<A, const C: BumpConfig> Bump<A, C>
where
    A: Allocator,
{
    /// Forwards to [`BumpScope::allocator`].
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> Option<&A> {
        self.as_scope().allocator()
    }

    /// Forwards to [`BumpScope::claim`].
    #[inline(always)]
    pub fn claim(&self) -> BumpClaimGuard<'_, '_, A, C> {
        self.as_scope().claim()
    }

    /// Forwards to [`BumpScope::scope_guard`].
    #[inline(always)]
    pub fn scope_guard(&mut self) -> BumpScopeGuard<'_, A, C> {
        self.as_mut_scope().scope_guard()
    }

    /// Forwards to [`BumpScope::scoped`].
    #[inline(always)]
    pub fn scoped<R>(&mut self, f: impl FnOnce(&mut BumpScope<A, C>) -> R) -> R {
        self.as_mut_scope().scoped(f)
    }

    /// Forwards to [`BumpScope::scoped_with_config`].
    #[inline(always)]
    pub fn scoped_with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<A, NEW_C>) -> R) -> R {
        self.as_mut_scope().scoped_with_config::<NEW_C, R>(f)
    }

    /// Forwards to [`BumpScope::with_config`].
    #[inline(always)]
    pub fn with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<'_, A, NEW_C>) -> R) -> R {
        self.as_mut_scope().with_config(f)
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
    ) -> Result<Result<BumpBox<'_, T>, E>, AllocError> {
        self.as_mut_scope().try_alloc_try_with_mut(f)
    }
}

unsafe impl<A, const C: BumpConfig> Allocator for Bump<A, C>
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
