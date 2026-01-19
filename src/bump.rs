use core::{
    alloc::Layout,
    cell::Cell,
    ffi::CStr,
    fmt::{self, Debug},
    mem::{ManuallyDrop, MaybeUninit, transmute},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::{self, NonNull},
};

#[cfg(feature = "nightly-clone-to-uninit")]
use core::clone::CloneToUninit;

use crate::{
    BaseAllocator, BumpBox, BumpScope, BumpScopeGuardRoot, Checkpoint, ErrorBehavior,
    alloc::{AllocError, Allocator},
    chunk::{ChunkSize, RawChunk},
    maybe_default_allocator,
    owned_slice::OwnedSlice,
    polyfill::{transmute_mut, transmute_ref},
    settings::{BumpAllocatorSettings, BumpSettings, False, MinimumAlignment, SupportedMinimumAlignment, True},
    stats::{AnyStats, Stats},
    traits::{self, BumpAllocatorCore},
};

// For docs.
#[allow(unused_imports)]
use crate::{traits::BumpAllocatorTyped, traits::BumpAllocatorTypedScope, traits::MutBumpAllocatorTypedScope};

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
        /// - const, without allocation: <code>[unallocated]</code>
        ///
        /// [new]: Self::new
        /// [new_in]: Self::new_in
        /// [default]: Self::default
        /// [with_size]: Self::with_size
        /// [with_size_in]: Self::with_size_in
        /// [with_capacity]: Self::with_capacity
        /// [with_capacity_in]: Self::with_capacity_in
        /// [unallocated]: Self::unallocated
        ///
        /// #### Allocate ...
        /// - sized values: [`alloc`], [`alloc_with`], [`alloc_default`], [`alloc_zeroed`]
        /// - strings: [`alloc_str`], <code>[alloc_fmt](Self::alloc_fmt)([_mut](Self::alloc_fmt_mut))</code>
        /// - c strings: [`alloc_cstr`], [`alloc_cstr_from_str`], <code>[alloc_cstr_fmt](Self::alloc_cstr_fmt)([_mut](Self::alloc_cstr_fmt_mut))</code>
        /// - slices: <code>alloc_slice_{[copy](Self::alloc_slice_copy), [clone](Self::alloc_slice_clone), [move](Self::alloc_slice_move), [fill](Self::alloc_slice_fill), [fill_with](Self::alloc_slice_fill_with)}</code>,
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
        /// - guaranteed allocated: <code>{[as](Self::as_guaranteed_allocated), [as_mut](Self::as_mut_guaranteed_allocated), [into](Self::into_guaranteed_allocated)}_guaranteed_allocated</code>
        /// - other: [`with_settings`], [`borrow_with_settings`], [`borrow_mut_with_settings`]
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
        /// [`alloc`]: Self::alloc
        /// [`alloc_with`]: Self::alloc_with
        /// [`alloc_default`]: Self::alloc_default
        /// [`alloc_zeroed`]: crate::zerocopy_08::BumpAllocatorTypedScopeExt::alloc_zeroed
        ///
        /// [`alloc_str`]: Self::alloc_str
        ///
        /// [`alloc_cstr`]: Self::alloc_cstr
        /// [`alloc_cstr_from_str`]: Self::alloc_cstr_from_str
        ///
        /// [`alloc_zeroed_slice`]: crate::zerocopy_08::BumpAllocatorTypedScopeExt::alloc_zeroed_slice
        ///
        /// [`alloc_iter`]: Self::alloc_iter
        /// [`alloc_iter_exact`]: Self::alloc_iter_exact
        /// [`alloc_iter_mut`]: Self::alloc_iter_mut
        /// [`alloc_iter_mut_rev`]: Self::alloc_iter_mut_rev
        ///
        /// [`alloc_uninit`]: Self::alloc_uninit
        /// [`alloc_uninit_slice`]: Self::alloc_uninit_slice
        /// [`alloc_uninit_slice_for`]: Self::alloc_uninit_slice_for
        ///
        /// [`alloc_try_with`]: Self::alloc_try_with
        /// [`alloc_try_with_mut`]: Self::alloc_try_with_mut
        ///
        /// [`alloc_clone`]: Self::alloc_clone
        ///
        /// [`scoped`]: Self::scoped
        /// [`scoped_aligned`]: Self::scoped_aligned
        /// [`scope_guard`]: Self::scope_guard
        ///
        /// [`checkpoint`]: Self::checkpoint
        /// [`reset_to`]: Self::reset_to
        ///
        /// [`reset`]: Self::reset
        /// [`dealloc`]: Self::dealloc
        ///
        /// [`aligned`]: Self::aligned
        ///
        /// [`with_settings`]: Self::with_settings
        /// [`borrow_with_settings`]: Self::borrow_with_settings
        /// [`borrow_mut_with_settings`]: Self::borrow_with_settings
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
        /// [`as_mut_scope`]: Self::as_mut_scope
        #[repr(transparent)]
        pub struct Bump<$($allocator_parameter)*, S = BumpSettings>
        where
            A: Allocator,
            S: BumpAllocatorSettings,
        {
            pub(crate) chunk: Cell<RawChunk<A, S>>,
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
    A: Allocator + UnwindSafe,
    S: BumpAllocatorSettings,
{
}

impl<A, S> Drop for Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    fn drop(&mut self) {
        let Some(chunk) = self.chunk.get().guaranteed_allocated() else {
            return;
        };

        unsafe {
            chunk.for_each_prev(|chunk| chunk.deallocate());
            chunk.for_each_next(|chunk| chunk.deallocate());
            chunk.deallocate();
        }
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
    #[inline(always)]
    fn default() -> Self {
        Self::new_in(Default::default())
    }
}

unsafe impl<A, S> Allocator for Bump<A, S>
where
    A: BaseAllocator<S::GuaranteedAllocated>,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.as_scope().allocate(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.as_scope().deallocate(ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.as_scope().grow(ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.as_scope().grow_zeroed(ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.as_scope().shrink(ptr, old_layout, new_layout) }
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
    /// This is equivalent to <code>[with_size](Bump::with_size)(512)</code>.
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
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn new() -> Self {
        panic_on_error(Self::generic_new())
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[try_with_size](Bump::try_with_size)(512)</code>.
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
    #[inline(always)]
    pub fn try_new() -> Result<Self, AllocError> {
        Self::generic_new()
    }

    #[inline]
    pub(crate) fn generic_new<E: ErrorBehavior>() -> Result<Self, E> {
        Self::generic_new_in(Default::default())
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

    /// Constructs a new `Bump` with at least enough space for `layout`.
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

    /// Constructs a new `Bump` with at least enough space for `layout`.
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

/// Methods for a [*guaranteed allocated*](crate#what-does-guaranteed-allocated-mean) `Bump`.
impl<A, S> Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings<GuaranteedAllocated = True>,
{
    /// Calls `f` with a new child scope.
    ///
    /// # Examples
    ///
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
    #[inline(always)]
    pub fn scoped<R>(&mut self, f: impl FnOnce(BumpScope<A, S>) -> R) -> R {
        let mut guard = self.scope_guard();
        f(guard.scope())
    }

    /// Calls `f` with a new child scope of a new minimum alignment.
    ///
    /// # Examples
    ///
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    ///
    /// // bump starts off by being aligned to 16
    /// assert!(bump.stats().current_chunk().bump_position().is_aligned_to(16));
    ///
    /// // allocate one byte
    /// bump.alloc(1u8);
    ///
    /// // now the bump is only aligned to 1
    /// // (if our `MIN_ALIGN` was higher, it would be that)
    /// assert!(bump.stats().current_chunk().bump_position().addr().get() % 2 == 1);
    /// assert_eq!(bump.stats().allocated(), 1);
    ///
    /// bump.scoped_aligned::<8, ()>(|bump| {
    ///    // in here, the bump will have the specified minimum alignment of 8
    ///    assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    ///    assert_eq!(bump.stats().allocated(), 8);
    ///
    ///    // allocating a value with its size being a multiple of 8 will no longer have
    ///    // to align the bump pointer before allocation
    ///    bump.alloc(1u64);
    ///    assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    ///    assert_eq!(bump.stats().allocated(), 16);
    ///    
    ///    // allocating a value smaller than the minimum alignment must align the bump pointer
    ///    // after the allocation, resulting in some wasted space
    ///    bump.alloc(1u8);
    ///    assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    ///    assert_eq!(bump.stats().allocated(), 24);
    /// });
    ///
    /// assert_eq!(bump.stats().allocated(), 1);
    /// ```
    #[inline(always)]
    pub fn scoped_aligned<const NEW_MIN_ALIGN: usize, R>(
        &mut self,
        f: impl FnOnce(BumpScope<A, S::WithMinimumAlignment<NEW_MIN_ALIGN>>) -> R,
    ) -> R
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.as_mut_scope().scoped_aligned::<NEW_MIN_ALIGN, R>(f)
    }

    /// Calls `f` with this scope but with a new minimum alignment.
    ///
    /// # Examples
    ///
    /// Increase the minimum alignment:
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_scope();
    ///
    /// // here we're allocating with a `MIN_ALIGN` of `1`
    /// let foo = bump.alloc_str("foo");
    /// assert_eq!(bump.stats().allocated(), 3);
    ///
    /// let bar = bump.aligned::<8, _>(|bump| {
    ///     // in here the bump position has been aligned to `8`
    ///     assert_eq!(bump.stats().allocated(), 8);
    ///     assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
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
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::{Bump, alloc::Global, settings::{BumpSettings, BumpAllocatorSettings}};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithMinimumAlignment<8>;
    ///
    /// let mut bump: Bump<Global, Settings> = Bump::new();
    /// let bump = bump.as_mut_scope();
    ///
    /// // make some allocations that benefit from the `MIN_ALIGN` of `8`
    /// let foo = bump.alloc(0u64);
    ///
    /// let bar = bump.aligned::<1, _>(|bump| {
    ///     // make some allocations that benefit from the lower `MIN_ALIGN` of `1`
    ///     let bar = bump.alloc(0u8);
    ///
    ///     // the bump position will not get aligned to `8` in here
    ///     assert_eq!(bump.stats().allocated(), 8 + 1);
    ///
    ///     bar
    /// });
    ///
    /// // after `aligned()`, the bump position will be aligned to `8` again
    /// // to satisfy our `MIN_ALIGN`
    /// assert!(bump.stats().current_chunk().bump_position().is_aligned_to(8));
    /// assert_eq!(bump.stats().allocated(), 16);
    ///
    /// // continue making allocations that benefit from the `MIN_ALIGN` of `8`
    /// let baz = bump.alloc(0u64);
    ///
    /// dbg!(foo, bar, baz);
    /// ```
    #[inline(always)]
    pub fn aligned<'a, const NEW_MIN_ALIGN: usize, R>(
        &'a mut self,
        f: impl FnOnce(BumpScope<'a, A, S::WithMinimumAlignment<NEW_MIN_ALIGN>>) -> R,
    ) -> R
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.as_mut_scope().aligned(f)
    }

    /// Creates a new [`BumpScopeGuardRoot`].
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
    #[must_use]
    #[inline(always)]
    pub fn scope_guard(&mut self) -> BumpScopeGuardRoot<'_, A, S> {
        BumpScopeGuardRoot::new(self)
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        self.as_scope().allocator()
    }
}

/// Methods for a **not** [*guaranteed allocated*](crate#what-does-guaranteed-allocated-mean) `Bump`.
impl<A, S> Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings<GuaranteedAllocated = False>,
{
    /// Constructs a new `Bump` without allocating a chunk.
    ///
    /// See [*What does guaranteed allocated mean?*](crate#what-does-guaranteed-allocated-mean).
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
    #[must_use]
    pub const fn unallocated() -> Self {
        Self {
            chunk: Cell::new(RawChunk::UNALLOCATED),
        }
    }
}

/// Methods that are always available.
impl<A, S> Bump<A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn get_allocator(&self) -> Option<&A> {
        self.as_scope().get_allocator()
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[with_size_in](Bump::with_size_in)(512, allocator)</code>.
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
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn new_in(allocator: A) -> Self {
        panic_on_error(Self::generic_new_in(allocator))
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[try_with_size_in](Bump::try_with_size_in)(512, allocator)</code>.
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
    #[inline(always)]
    pub fn try_new_in(allocator: A) -> Result<Self, AllocError> {
        Self::generic_new_in(allocator)
    }

    #[inline]
    pub(crate) fn generic_new_in<E: ErrorBehavior>(allocator: A) -> Result<Self, E> {
        Ok(Self {
            chunk: Cell::new(RawChunk::new_in(ChunkSize::<A, S::Up>::DEFAULT, None, allocator)?),
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
            chunk: Cell::new(RawChunk::new_in(
                ChunkSize::<A, S::Up>::from_hint(size).ok_or_else(E::capacity_overflow)?,
                None,
                allocator,
            )?),
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
            chunk: Cell::new(RawChunk::new_in(
                ChunkSize::<A, S::Up>::from_capacity(layout).ok_or_else(E::capacity_overflow)?,
                None,
                allocator,
            )?),
        })
    }

    // This needs `&mut self` to make sure that no allocations are alive.
    /// Deallocates every chunk but the newest, which is also the biggest.
    ///
    /// ```
    /// use bump_scope::Bump;
    ///
    /// let mut bump: Bump = Bump::new();
    ///
    /// // won't fit in the default sized first chunk
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
        let Some(mut chunk) = self.chunk.get().guaranteed_allocated() else {
            return;
        };

        unsafe {
            chunk.for_each_prev(|chunk| chunk.deallocate());

            while let Some(next) = chunk.next() {
                chunk.deallocate();
                chunk = next;
            }
        }

        chunk.set_prev(None);
        chunk.reset();

        // SAFETY: casting from guaranteed-allocated to non-guaranteed-allocated is safe
        self.chunk.set(unsafe { chunk.cast() });
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'_, A, S> {
        self.as_scope().stats()
    }

    /// Returns this `&Bump` as a `&BumpScope`.
    #[inline(always)]
    pub fn as_scope(&self) -> &BumpScope<'_, A, S> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &*ptr::from_ref(self).cast() }
    }

    /// Returns this `&mut Bump` as a `&mut BumpScope`.
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut BumpScope<'_, A, S> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &mut *ptr::from_mut(self).cast() }
    }

    /// Converts this `Bump` into a `Bump` with new settings.
    ///
    /// Not every setting can be converted to. This function will fail to compile when:
    /// - the bump direction differs
    /// - the new setting is guaranteed-allocated when the old one isn't
    ///   (use [`into_guaranteed_allocated`](Self::into_guaranteed_allocated) to do this conversion)
    #[inline]
    pub fn with_settings<NewS>(mut self) -> Bump<A, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        self.as_mut_scope().claim_mut().with_settings::<NewS>();
        unsafe { transmute(self) }
    }

    /// Borrows this `Bump` with new settings.
    ///
    /// Not every settings can be converted to. This function will fail to compile when:
    /// - the bump direction differs
    /// - the new setting is guaranteed-allocated when the old one isn't
    ///   (use [`as_guaranteed_allocated`](Self::as_guaranteed_allocated) to do this conversion)
    /// - the minimum alignment differs
    #[inline]
    pub fn borrow_with_settings<NewS>(&self) -> &Bump<A, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        self.as_scope().borrow_with_settings::<NewS>();
        unsafe { transmute_ref(self) }
    }

    /// Borrows this `Bump` mutably with new settings.
    ///
    /// Not every settings can be converted to. This function will fail to compile when:
    /// - the bump direction differs
    /// - the guaranteed-allocated property differs
    /// - the new minimum alignment is less than the old one
    #[inline]
    pub fn borrow_mut_with_settings<NewS>(&mut self) -> &mut Bump<A, NewS>
    where
        NewS: BumpAllocatorSettings,
    {
        self.as_mut_scope().borrow_mut_with_settings::<NewS>();
        unsafe { transmute_mut(self) }
    }

    /// Converts this `Bump` into a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `Bump`.
    ///
    /// If this `Bump` is not yet allocated, `f` will be called to allocate it.
    ///
    /// # Panics
    ///
    /// Panics if the closure panics.
    ///
    /// # Examples
    ///
    /// Creating scopes with a non-`GUARANTEED_ALLOCATED` bump is not possible.
    /// ```compile_fail,E0599
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    ///
    /// let mut bump: Bump<Global, Settings> = Bump::unallocated();
    ///
    /// bump.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// ```
    ///
    /// Using this function you can make a `Bump` guaranteed allocated and create scopes.
    /// ```
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    ///
    /// let bump: Bump<Global, Settings> = Bump::unallocated();
    /// let mut bump = bump.into_guaranteed_allocated(Bump::new);
    ///
    /// bump.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// ```
    ///
    /// Initialize an unallocated `Bump` with a custom size or capacity:
    /// ```
    /// # use core::alloc::Layout;
    /// # use bump_scope::{Bump, alloc::Global};
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// # type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    /// # let bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.into_guaranteed_allocated(|| {
    ///     Bump::with_size(2048)
    /// });
    ///
    /// // or
    ///
    /// # let bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.into_guaranteed_allocated(|| {
    ///     Bump::with_capacity(Layout::new::<[i32; 1024]>())
    /// });
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn into_guaranteed_allocated(
        self,
        f: impl FnOnce() -> Bump<A, S::WithGuaranteedAllocated<true>>,
    ) -> Bump<A, S::WithGuaranteedAllocated<true>> {
        self.as_scope().ensure_allocated(f);
        unsafe { transmute(self) }
    }

    /// Converts this `Bump` into a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `Bump`.
    ///
    /// If this `Bump` is not yet allocated, `f` will be called to allocate it.
    ///
    /// # Errors
    ///
    /// Errors if the closure fails.
    ///
    /// # Examples
    ///
    /// Creating scopes with a non-`GUARANTEED_ALLOCATED` bump is not possible.
    /// ```compile_fail,E0599
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    ///
    /// let mut bump: Bump<Global, Settings> = Bump::unallocated();
    ///
    /// bump.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// ```
    ///
    /// Using this function you can make a `Bump` guaranteed allocated and create scopes.
    /// ```
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    /// let bump: Bump<Global, Settings> = Bump::unallocated();
    /// let mut bump = bump.try_into_guaranteed_allocated(Bump::try_new)?;
    ///
    /// bump.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Initialize an unallocated `Bump` with a custom size or capacity:
    /// ```
    /// # use core::alloc::Layout;
    /// # use bump_scope::{Bump, alloc::Global};
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// # type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    /// # let bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.try_into_guaranteed_allocated(|| {
    ///     Bump::try_with_size(2048)
    /// })?;
    ///
    /// // or
    ///
    /// # let bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.try_into_guaranteed_allocated(|| {
    ///     Bump::try_with_capacity(Layout::new::<[i32; 1024]>())
    /// })?;
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_into_guaranteed_allocated(
        self,
        f: impl FnOnce() -> Result<Bump<A, S::WithGuaranteedAllocated<true>>, AllocError>,
    ) -> Result<Bump<A, S::WithGuaranteedAllocated<true>>, AllocError> {
        self.as_scope().try_ensure_allocated(f)?;
        Ok(unsafe { transmute(self) })
    }

    /// Borrows `Bump` as a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `Bump`.
    ///
    /// If this `Bump` is not yet allocated, `f` will be called to allocate it.
    ///
    /// # Panics
    ///
    /// Panics if the closure panics.
    ///
    /// # Examples
    ///
    /// Initialize an unallocated `Bump` with a custom size or capacity:
    /// ```
    /// # use core::alloc::Layout;
    /// # use bump_scope::{Bump, alloc::Global};
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// # type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    /// # let bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.as_guaranteed_allocated(|| {
    ///     Bump::with_size(2048)
    /// });
    /// # _ = bump;
    ///
    /// // or
    ///
    /// # let bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.as_guaranteed_allocated(|| {
    ///     Bump::with_capacity(Layout::new::<[i32; 1024]>())
    /// });
    /// # _ = bump;
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn as_guaranteed_allocated(
        &self,
        f: impl FnOnce() -> Bump<A, S::WithGuaranteedAllocated<true>>,
    ) -> &Bump<A, S::WithGuaranteedAllocated<true>> {
        self.as_scope().ensure_allocated(f);
        unsafe { transmute_ref(self) }
    }

    /// Borrows `Bump` as an [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `Bump`.
    ///
    /// If this `Bump` is not yet allocated, `f` will be called to allocate it.
    ///
    /// # Errors
    ///
    /// Errors if the closure fails.
    ///
    /// [`try_as_mut_guaranteed_allocated`]: Self::try_as_mut_guaranteed_allocated
    ///
    /// # Examples
    ///
    /// Initialize an unallocated `Bump` with a custom size or capacity:
    /// ```
    /// # use core::alloc::Layout;
    /// # use bump_scope::{Bump, alloc::Global};
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// # type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    /// # let bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.try_as_guaranteed_allocated(|| {
    ///     Bump::try_with_size(2048)
    /// })?;
    /// # _ = bump;
    ///
    /// // or
    ///
    /// let bump = bump.try_as_guaranteed_allocated(|| {
    ///     Bump::try_with_capacity(Layout::new::<[i32; 1024]>())
    /// })?;
    /// # _ = bump;
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_as_guaranteed_allocated(
        &self,
        f: impl FnOnce() -> Result<Bump<A, S::WithGuaranteedAllocated<true>>, AllocError>,
    ) -> Result<&Bump<A, S::WithGuaranteedAllocated<true>>, AllocError> {
        self.as_scope().try_ensure_allocated(f)?;
        Ok(unsafe { transmute_ref(self) })
    }

    /// Mutably borrows `Bump` as a [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `Bump`.
    ///
    /// If this `Bump` is not yet allocated, `f` will be called to allocate it.
    ///
    /// # Panics
    ///
    /// Panics if the closure panics.
    ///
    /// # Examples
    ///
    /// Creating scopes with a non-`GUARANTEED_ALLOCATED` bump is not possible.
    /// ```compile_fail,E0599
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    ///
    /// let mut bump: Bump<Global, Settings> = Bump::unallocated();
    ///
    /// bump.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// ```
    ///
    /// Using this function you can make a `Bump` guaranteed allocated and create scopes.
    /// ```
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    ///
    /// let mut bump: Bump<Global, Settings> = Bump::unallocated();
    ///
    /// bump.as_mut_guaranteed_allocated(Bump::new).scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// ```
    ///
    /// Initialize an unallocated `Bump` with a custom size or capacity:
    /// ```
    /// # use core::alloc::Layout;
    /// # use bump_scope::{Bump, alloc::Global};
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// # type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    /// # let mut bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.as_mut_guaranteed_allocated(|| {
    ///     Bump::with_size(2048)
    /// });
    /// # _ = bump;
    ///
    /// // or
    ///
    /// # let mut bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.as_mut_guaranteed_allocated(|| {
    ///     Bump::with_capacity(Layout::new::<[i32; 1024]>())
    /// });
    /// # _ = bump;
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn as_mut_guaranteed_allocated(
        &mut self,
        f: impl FnOnce() -> Bump<A, S::WithGuaranteedAllocated<true>>,
    ) -> &mut Bump<A, S::WithGuaranteedAllocated<true>> {
        self.as_scope().ensure_allocated(f);
        unsafe { transmute_mut(self) }
    }

    /// Mutably borrows `Bump` as an [guaranteed allocated](crate#what-does-guaranteed-allocated-mean) `Bump`.
    ///
    /// If this `Bump` is not yet allocated, `f` will be called to allocate it.
    ///
    /// # Errors
    ///
    /// Errors if the closure fails.
    ///
    /// # Examples
    ///
    /// Creating scopes with a non-`GUARANTEED_ALLOCATED` bump is not possible.
    /// ```compile_fail,E0599
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    ///
    /// let mut bump: Bump<Global, Settings> = Bump::unallocated();
    ///
    /// bump.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// ```
    ///
    /// Using this function you can make a `Bump` guaranteed allocated and create scopes.
    /// ```
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    ///
    /// let mut bump: Bump<Global, Settings> = Bump::unallocated();
    ///
    /// bump.try_as_mut_guaranteed_allocated(Bump::try_new)?.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Initialize an unallocated `Bump` with a custom size or capacity:
    /// ```
    /// # use core::alloc::Layout;
    /// # use bump_scope::{Bump, alloc::Global};
    /// # use bump_scope::settings::{BumpSettings, BumpAllocatorSettings};
    /// # type Settings = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
    /// # let mut bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.try_as_mut_guaranteed_allocated(|| {
    ///     Bump::try_with_size(2048)
    /// })?;
    /// # _ = bump;
    ///
    /// // or
    ///
    /// # let mut bump: Bump<Global, Settings> = Bump::unallocated();
    /// let bump = bump.try_as_mut_guaranteed_allocated(|| {
    ///     Bump::try_with_capacity(Layout::new::<[i32; 1024]>())
    /// })?;
    /// # _ = bump;
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_as_mut_guaranteed_allocated(
        &mut self,
        f: impl FnOnce() -> Result<Bump<A, S::WithGuaranteedAllocated<true>>, AllocError>,
    ) -> Result<&mut Bump<A, S::WithGuaranteedAllocated<true>>, AllocError> {
        self.as_scope().try_ensure_allocated(f)?;
        Ok(unsafe { transmute_mut(self) })
    }

    /// Converts this `Bump` into a raw pointer.
    ///
    /// ```
    /// use bump_scope::Bump;
    ///
    /// let bump: Bump = Bump::new();
    /// let ptr = bump.into_raw();
    /// let bump: Bump = unsafe { Bump::from_raw(ptr) };
    ///
    /// bump.alloc_str("Why did i do this?");
    /// ```
    #[inline]
    #[must_use]
    pub fn into_raw(self) -> NonNull<()> {
        let this = ManuallyDrop::new(self);
        this.chunk.get().header().cast()
    }

    /// Converts the raw pointer that was created with [`into_raw`](Bump::into_raw) back into a `Bump`.
    ///
    /// # Safety
    /// - `ptr` must have been created with `Self::into_raw`.
    /// - This function must only be called once with this `ptr`.
    #[inline]
    #[must_use]
    pub unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        Self {
            chunk: Cell::new(unsafe { RawChunk::from_header(ptr.cast()) }),
        }
    }
}

impl<'b, A, S> From<&'b Bump<A, S>> for &'b BumpScope<'b, A, S>
where
    A: Allocator,
    S: BumpAllocatorSettings,
{
    #[inline(always)]
    fn from(value: &'b Bump<A, S>) -> Self {
        value.as_scope()
    }
}

impl<'b, A, S> From<&'b mut Bump<A, S>> for &'b mut BumpScope<'b, A, S>
where
    A: Allocator,
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

/// Additional `alloc` methods that are not from traits.
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
    /// # let bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
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
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(result.unwrap(), 123);
    /// assert_eq!(bump.stats().allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[cfg_attr(feature = "nightly-tests", doc = "```")]
    #[cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
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
