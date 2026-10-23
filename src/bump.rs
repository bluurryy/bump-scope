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
    BumpClaimGuard, BumpScope, BumpScopeGuard, allocator_impl,
    chunk_size::{ChunkSizeCapacity, ChunkSizeHint, ChunkSizeMin},
    config::{BumpConfig, DEFAULT_BUMP_CONFIG, config},
    maybe_default_allocator,
    owned_slice::OwnedSlice,
    polyfill::{transmute_mut, transmute_ref, transmute_value},
    raw_bump::RawBump,
    stats::{AnyStats, Stats},
    traits::{AsBumpScope, AsMutBumpScope},
};

#[cfg(feature = "panic-on-alloc")]
use crate::{BumpAllocation, panic_on_error};

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
        /// - const, without allocating a chunk: <code>[const_new]</code>
        ///
        /// [new]: Bump::new
        /// [new_in]: Bump::new_in
        /// [default]: Bump::default
        /// [with_size]: Bump::with_size
        /// [with_size_in]: Bump::with_size_in
        /// [with_capacity]: Bump::with_capacity
        /// [with_capacity_in]: Bump::with_capacity_in
        /// [const_new]: Bump::const_new
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
        ///   which can then be conveniently initialized by the [`init*` methods of `BoxU`](crate::BoxU#bumpbox-has-a-lot-of-methods).
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
        /// - [`with_config`], [`change_config`], [`ref_change_config`], [`mut_change_config`]
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
        /// [`ref_change_config`]: Bump::ref_change_config
        /// [`mut_change_config`]: Bump::ref_change_config
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
    /// With [`const_constructible`] this does the same as [`const_new`], otherwise it does the same as [`new`].
    ///
    /// # Panics
    /// Panics if this `Bump` is **not** [`const_constructible`] and the "panic-on-alloc" feature is disabled.
    ///
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    /// [`new`]: Bump::new
    /// [`const_new`]: Bump::const_new
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
    /// # Fails
    ///
    /// Fails to compile when not [`const_constructible`].
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
    /// let bump: Bump<Global, CONFIG> = Bump::const_new();
    /// # _ = bump;
    /// ```
    ///
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    #[must_use]
    pub const fn const_new() -> Self {
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

        self.raw.assert_convertible_claimable::<NEW_C>();
        self.raw.assert_convertible_const_constructible::<NEW_C>();
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
    pub fn ref_change_config<const NEW_C: BumpConfig>(&self) -> &Bump<A, NEW_C> {
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
    pub fn mut_change_config<const NEW_C: BumpConfig>(&mut self) -> &mut Bump<A, NEW_C> {
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
    /// See [`BumpScope::alloc`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc<T>(&self, value: T) -> BumpAllocation<T, &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc(value));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc`].
    #[inline(always)]
    pub fn try_alloc<T>(&self, value: T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc(value)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_with`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> BumpAllocation<T, &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_with(f));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_with`].
    #[inline(always)]
    pub fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_with(f)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_default`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_default<T: Default>(&self) -> BumpAllocation<T, &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_default());
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_default`].
    #[inline(always)]
    pub fn try_alloc_default<T: Default>(&self) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_default()?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_clone`].
    #[cfg(feature = "panic-on-alloc")]
    #[cfg(feature = "nightly-clone-to-uninit")]
    pub fn alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> BumpAllocation<T, &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_clone(value));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_clone`].
    #[cfg(feature = "nightly-clone-to-uninit")]
    pub fn try_alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_clone(value)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_uninit`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_uninit<T>(&self) -> BumpAllocation<MaybeUninit<T>, &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_uninit());
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_uninit`].
    #[inline(always)]
    pub fn try_alloc_uninit<T>(&self) -> Result<BumpAllocation<MaybeUninit<T>, &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_uninit()?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_slice_move`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_slice_move(slice));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_slice_move`].
    #[inline(always)]
    pub fn try_alloc_slice_move<T>(
        &self,
        slice: impl OwnedSlice<Item = T>,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_slice_move(slice)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_slice_copy`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_slice_copy(slice));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_slice_copy`].
    #[inline(always)]
    pub fn try_alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_slice_copy(slice)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_slice_clone`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_slice_clone(slice));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_slice_clone`].
    #[inline(always)]
    pub fn try_alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_slice_clone(slice)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_slice_fill`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_slice_fill(len, value));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_slice_fill`].
    #[inline(always)]
    pub fn try_alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_slice_fill(len, value)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_slice_fill_with`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_slice_fill_with(len, f));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_slice_fill_with`].
    #[inline(always)]
    pub fn try_alloc_slice_fill_with<T>(
        &self,
        len: usize,
        f: impl FnMut() -> T,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_slice_fill_with(len, f)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_uninit_slice`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_uninit_slice<T>(&self, len: usize) -> BumpAllocation<[MaybeUninit<T>], &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_uninit_slice(len));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_uninit_slice`].
    #[inline(always)]
    pub fn try_alloc_uninit_slice<T>(&self, len: usize) -> Result<BumpAllocation<[MaybeUninit<T>], &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_uninit_slice(len)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_uninit_slice_for`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_uninit_slice_for<T>(&self, slice: &[T]) -> BumpAllocation<[MaybeUninit<T>], &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_uninit_slice_for(slice));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_uninit_slice_for`].
    #[inline(always)]
    pub fn try_alloc_uninit_slice_for<T>(&self, slice: &[T]) -> Result<BumpAllocation<[MaybeUninit<T>], &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_uninit_slice_for(slice)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_str`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_str(&self, src: &str) -> BumpAllocation<str, &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_str(src));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_str`].
    #[inline(always)]
    pub fn try_alloc_str(&self, src: &str) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_str(src)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_fmt`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fmt(&self, args: fmt::Arguments) -> BumpAllocation<str, &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_fmt(args));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_fmt`].
    #[inline(always)]
    pub fn try_alloc_fmt(&self, args: fmt::Arguments) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_fmt(args)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_fmt`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fmt_mut(mut self: &mut Self, args: fmt::Arguments) -> BumpAllocation<str, &Self> {
        let boxu = panic_on_error(self.as_mut_bump_scope().imp_alloc_fmt_mut(args));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_fmt_mut`].
    #[inline(always)]
    pub fn try_alloc_fmt_mut(mut self: &mut Self, args: fmt::Arguments) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxu = self.as_mut_bump_scope().imp_alloc_fmt_mut(args)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_cstr`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr(&self, src: &CStr) -> &CStr {
        panic_on_error(self.as_bump_scope().imp_alloc_cstr(src))
    }

    /// See [`BumpScope::try_alloc_cstr`].
    #[inline(always)]
    pub fn try_alloc_cstr(&self, src: &CStr) -> Result<&CStr, AllocError> {
        self.as_bump_scope().imp_alloc_cstr(src)
    }

    /// See [`BumpScope::alloc_cstr_from_str`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_from_str(&self, src: &str) -> &CStr {
        panic_on_error(self.as_bump_scope().imp_alloc_cstr_from_str(src))
    }

    /// See [`BumpScope::try_alloc_cstr_from_str`].
    #[inline(always)]
    pub fn try_alloc_cstr_from_str(&self, src: &str) -> Result<&CStr, AllocError> {
        self.as_bump_scope().imp_alloc_cstr_from_str(src)
    }

    /// See [`BumpScope::alloc_cstr_fmt`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_fmt(&self, args: fmt::Arguments) -> &CStr {
        panic_on_error(self.as_bump_scope().imp_alloc_cstr_fmt(args))
    }

    /// See [`BumpScope::try_alloc_cstr_fmt`].
    #[inline(always)]
    pub fn try_alloc_cstr_fmt(&self, args: fmt::Arguments) -> Result<&CStr, AllocError> {
        self.as_bump_scope().imp_alloc_cstr_fmt(args)
    }

    /// See [`BumpScope::alloc_cstr_fmt_mut`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_fmt_mut(mut self: &mut Self, args: fmt::Arguments) -> &CStr {
        panic_on_error(self.as_mut_bump_scope().imp_alloc_cstr_fmt_mut(args))
    }

    /// See [`BumpScope::try_alloc_cstr_fmt`].
    #[inline(always)]
    pub fn try_alloc_cstr_fmt_mut(mut self: &mut Self, args: fmt::Arguments) -> Result<&CStr, AllocError> {
        self.as_mut_bump_scope().imp_alloc_cstr_fmt_mut(args)
    }

    /// See [`BumpScope::alloc_iter`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.as_bump_scope().imp_alloc_iter(iter));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_iter`].
    #[inline(always)]
    pub fn try_alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.as_bump_scope().imp_alloc_iter(iter)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::alloc_iter_mut`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter_mut<T>(mut self: &mut Self, iter: impl IntoIterator<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxu = panic_on_error(self.as_mut_bump_scope().imp_alloc_iter_mut(iter));
        BumpAllocation::wrap(boxu, self)
    }

    /// See [`BumpScope::try_alloc_iter_mut`].
    #[inline(always)]
    pub fn try_alloc_iter_mut<T>(
        mut self: &mut Self,
        iter: impl IntoIterator<Item = T>,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxu = self.as_mut_bump_scope().imp_alloc_iter_mut(iter)?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::try_alloc_try_with`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    pub fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpAllocation<T, &Self>, E> {
        match panic_on_error(self.as_bump_scope().imp_alloc_try_with(f)) {
            Ok(boxu) => Ok(BumpAllocation::wrap(boxu, self)),
            Err(err) => Err(err),
        }
    }

    /// See [`BumpScope::try_alloc_try_with`].
    #[inline(always)]
    pub fn try_alloc_try_with<T, E>(
        &self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpAllocation<T, &Self>, E>, AllocError> {
        Ok(match self.as_bump_scope().imp_alloc_try_with(f)? {
            Ok(boxu) => Ok(BumpAllocation::wrap(boxu, self)),
            Err(err) => Err(err),
        })
    }

    /// See [`BumpScope::alloc_try_with_mut`].
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    pub fn alloc_try_with_mut<T, E>(
        mut self: &mut Self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<BumpAllocation<T, &Self>, E> {
        let boxu = panic_on_error(self.as_mut_bump_scope().imp_alloc_try_with_mut(f))?;
        Ok(BumpAllocation::wrap(boxu, self))
    }

    /// See [`BumpScope::try_alloc_try_with_mut`].
    #[inline(always)]
    pub fn try_alloc_try_with_mut<T, E>(
        mut self: &mut Self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpAllocation<T, &Self>, E>, AllocError> {
        Ok(match self.as_mut_bump_scope().imp_alloc_try_with(f)? {
            Ok(boxu) => Ok(BumpAllocation::wrap(boxu, self)),
            Err(err) => Err(err),
        })
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
