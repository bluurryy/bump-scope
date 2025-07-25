use core::{
    alloc::Layout,
    cell::Cell,
    ffi::CStr,
    fmt::{self, Debug},
    mem::{self, ManuallyDrop, MaybeUninit, transmute},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::{self, NonNull},
};

use crate::{
    BaseAllocator, BumpBox, BumpScope, BumpScopeGuardRoot, Checkpoint, ErrorBehavior, FixedBumpString, FixedBumpVec,
    MinimumAlignment, RawChunk, SupportedMinimumAlignment,
    alloc::{AllocError, Allocator},
    chunk_size::ChunkSize,
    maybe_default_allocator,
    owned_slice::OwnedSlice,
    polyfill::{transmute_mut, transmute_ref},
    stats::{AnyStats, Stats},
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

macro_rules! make_type {
    ($($allocator_parameter:tt)*) => {
        /// The bump allocator.
        ///
        /// # Overview
        /// All of the mentioned methods that do allocations panic if the base allocator returned an error.
        /// For every such panicking method, there is a corresponding `try_`-prefixed version that returns a `Result` instead.
        ///
        /// ## Create a `Bump` ...
        /// - with a default size hint: <code>[new]\([_in][new_in])</code> / <code>[default]</code>
        /// - provide a size hint: <code>[with_size]\([_in][with_size_in])</code>
        /// - provide a minimum capacity: <code>[with_capacity]\([_in][with_capacity_in])</code>
        /// - without allocation: <code>[unallocated]</code>
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
        /// ## Allocate ...
        /// - sized values: [`alloc`], [`alloc_with`], [`alloc_default`], [`alloc_zeroed`]
        /// - strings: [`alloc_str`], [`alloc_fmt`], [`alloc_fmt_mut`]
        /// - c strings: [`alloc_cstr`], [`alloc_cstr_from_str`] [`alloc_cstr_fmt`], [`alloc_cstr_fmt_mut`]
        /// - slices: [`alloc_slice_copy`], [`alloc_slice_clone`], [`alloc_slice_move`], [`alloc_slice_fill`], [`alloc_slice_fill_with`], [`alloc_zeroed_slice`]
        /// - slices from an iterator: [`alloc_iter`], [`alloc_iter_exact`], [`alloc_iter_mut`], [`alloc_iter_mut_rev`]
        /// - uninitialized values: [`alloc_uninit`], [`alloc_uninit_slice`], [`alloc_uninit_slice_for`]
        ///
        ///   which can then be conveniently initialized by the [`init*` methods of `BumpBox`](crate::BumpBox#bumpbox-has-a-lot-of-methods).
        /// - results: [`alloc_try_with`], [`alloc_try_with_mut`]
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
        /// *This requires the `allocator_api2_02` feature OR the `nightly-allocator-api` feature along with hashbrown's `nightly` feature.*
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
        /// On nightly and with the feature `"nightly-allocator-api"` you can also allocate collections from `std` that have an allocator parameter:
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
        /// [`alloc_zeroed`]: crate::zerocopy_08::BumpExt::alloc_zeroed
        ///
        /// [`alloc_str`]: Self::alloc_str
        /// [`alloc_fmt`]: Self::alloc_fmt
        /// [`alloc_fmt_mut`]: Self::alloc_fmt_mut
        ///
        /// [`alloc_cstr`]: Self::alloc_cstr
        /// [`alloc_cstr_from_str`]: Self::alloc_cstr_from_str
        /// [`alloc_cstr_fmt`]: Self::alloc_cstr_fmt
        /// [`alloc_cstr_fmt_mut`]: Self::alloc_cstr_fmt_mut
        ///
        /// [`alloc_slice_copy`]: Self::alloc_slice_copy
        /// [`alloc_slice_clone`]: Self::alloc_slice_clone
        /// [`alloc_slice_move`]: Self::alloc_slice_move
        /// [`alloc_slice_fill`]: Self::alloc_slice_fill
        /// [`alloc_slice_fill_with`]: Self::alloc_slice_fill_with
        /// [`alloc_zeroed_slice`]: crate::zerocopy_08::BumpExt::alloc_zeroed_slice
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
        /// ## Scopes and Checkpoints
        ///
        /// See [Scopes and Checkpoints](crate#scopes-and-checkpoints).
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
        pub struct Bump<
            $($allocator_parameter)*,
            const MIN_ALIGN: usize = 1,
            const UP: bool = true,
            const GUARANTEED_ALLOCATED: bool = true,
        > where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
            A: BaseAllocator<GUARANTEED_ALLOCATED>,
        {
            pub(crate) chunk: Cell<RawChunk<UP, A>>,
        }
    };
}

maybe_default_allocator!(make_type);

// Sending Bumps when nothing is allocated is fine.
// When something is allocated Bump is borrowed and sending is not possible.
unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Send
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> UnwindSafe
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + UnwindSafe,
{
}

impl<const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool, A> RefUnwindSafe
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + UnwindSafe,
{
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Drop
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn drop(&mut self) {
        if self.as_scope().is_unallocated() {
            return;
        }

        unsafe {
            let chunk = self.chunk.get();
            chunk.for_each_prev(|chunk| chunk.deallocate());
            chunk.for_each_next(|chunk| chunk.deallocate());
            chunk.deallocate();
        }
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Debug
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AnyStats::from(self.stats()).debug_format("Bump", f)
    }
}

#[cfg(feature = "panic-on-alloc")]
impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Default
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + Default,
{
    #[inline(always)]
    fn default() -> Self {
        Self::new_in(Default::default())
    }
}

unsafe impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Allocator
    for Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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

impl<A, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP, false>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<false>,
{
    /// Constructs a new `Bump` without doing any allocations.
    ///
    /// The resulting `Bump` will have its [`GUARANTEED_ALLOCATED`](crate#guaranteed_allocated-parameter) parameter set to `false`.
    /// Such a `Bump` is unable to create a scope with `scoped` or `scope_guard`.
    /// It has to first be converted into a guaranteed allocated `Bump` using
    /// <code>[guaranteed_allocated](Bump::guaranteed_allocated)([_ref](Bump::guaranteed_allocated_ref)/[_mut](Bump::guaranteed_allocated_mut))</code>.
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::Bump;
    /// use bump_scope::alloc::Global;
    ///
    /// let bump: Bump<Global, 1, true, false> = Bump::unallocated();
    /// # _ = bump;
    /// ```
    #[must_use]
    pub const fn unallocated() -> Self {
        Self {
            chunk: Cell::new(RawChunk::UNALLOCATED),
        }
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> Option<&A> {
        self.as_scope().allocator()
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + Default,
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

/// These functions are only available if the `Bump` is [guaranteed allocated](crate#guaranteed_allocated-parameter).
impl<A, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
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
    pub fn scoped<R>(&mut self, f: impl FnOnce(BumpScope<A, MIN_ALIGN, UP>) -> R) -> R {
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
        f: impl FnOnce(BumpScope<A, NEW_MIN_ALIGN, UP>) -> R,
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
    /// # use bump_scope::{Bump, alloc::Global};
    /// let mut bump: Bump<Global, 8> = Bump::new();
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
        f: impl FnOnce(BumpScope<'a, A, NEW_MIN_ALIGN, UP>) -> R,
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
    pub fn scope_guard(&mut self) -> BumpScopeGuardRoot<'_, A, MIN_ALIGN, UP> {
        BumpScopeGuardRoot::new(self)
    }

    /// Returns a reference to the base allocator.
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        self.as_scope().allocator()
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    /// Creates a checkpoint of the current bump position.
    ///
    /// The bump position can be reset to this checkpoint with [`reset_to`].
    ///
    /// [`reset_to`]: Self::reset_to
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.stats().allocated(), 5);
    ///     # _ = hello;
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk.get())
    }

    /// Resets the bump position to a previously created checkpoint.
    /// The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    ///
    /// - the checkpoint must have been created by this bump allocator
    /// - the bump allocator must not have been [`reset`] since creation of this checkpoint
    /// - there must be no references to allocations made since creation of this checkpoint
    /// - the checkpoint must not have been created by an`!GUARANTEED_ALLOCATED` when self is `GUARANTEED_ALLOCATED`
    ///
    /// [`reset`]: crate::Bump::reset
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::new();
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.stats().allocated(), 5);
    ///     # _ = hello;
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline]
    pub unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe { self.as_scope().reset_to(checkpoint) };
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
            chunk: Cell::new(RawChunk::new_in(ChunkSize::DEFAULT, None, allocator)?),
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
                ChunkSize::from_hint(size).ok_or_else(E::capacity_overflow)?,
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
                ChunkSize::from_capacity(layout).ok_or_else(E::capacity_overflow)?,
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
        let mut chunk = self.chunk.get();

        unsafe {
            chunk.for_each_prev(|chunk| chunk.deallocate());

            while let Some(next) = chunk.next() {
                chunk.deallocate();
                chunk = next;
            }
        }

        chunk.set_prev(None);
        chunk.reset();
        self.chunk.set(chunk);
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'_, A, UP, GUARANTEED_ALLOCATED> {
        self.as_scope().stats()
    }

    /// Returns this `&Bump` as a `&BumpScope`.
    #[inline(always)]
    pub fn as_scope(&self) -> &BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &*ptr::from_ref(self).cast() }
    }

    /// Returns this `&mut Bump` as a `&mut BumpScope`.
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut BumpScope<'_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &mut *ptr::from_mut(self).cast() }
    }

    /// Converts this `Bump` into a `Bump` with a new minimum alignment.
    #[inline(always)]
    pub fn into_aligned<const NEW_MIN_ALIGN: usize>(self) -> Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.as_scope().align::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align() }
    }

    /// Mutably borrows `Bump` with a new minimum alignment.
    ///
    /// **This cannot decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment.
    ///
    /// When decreasing the alignment we need to make sure that the bump position is realigned to the original alignment.
    /// That can only be ensured by having a function that takes a closure, like the methods mentioned above do.
    #[inline(always)]
    pub fn as_aligned_mut<const NEW_MIN_ALIGN: usize>(&mut self) -> &mut Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        self.as_scope().must_align_more::<NEW_MIN_ALIGN>();
        unsafe { self.cast_align_mut() }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align<const NEW_MIN_ALIGN: usize>(self) -> Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        let chunk = self.chunk.get();
        mem::forget(self);

        Bump { chunk: Cell::new(chunk) }
    }

    #[inline(always)]
    pub(crate) unsafe fn cast_align_mut<const NEW_MIN_ALIGN: usize>(
        &mut self,
    ) -> &mut Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
    where
        MinimumAlignment<NEW_MIN_ALIGN>: SupportedMinimumAlignment,
    {
        unsafe { &mut *ptr::from_mut(self).cast::<Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>() }
    }

    /// Converts this `Bump` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// Creating scopes with a non-`GUARANTEED_ALLOCATED` bump is not possible.
    /// ```compile_fail,E0599
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// let mut bump: Bump<Global, 1, true, false> = Bump::unallocated();
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
    /// let bump: Bump<Global, 1, true, false> = Bump::unallocated();
    /// let mut bump = bump.guaranteed_allocated();
    ///
    /// bump.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn guaranteed_allocated(self) -> Bump<A, MIN_ALIGN, UP> {
        panic_on_error(self.generic_guaranteed_allocated())
    }

    /// Converts this `Bump` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    ///
    /// # Examples
    ///
    /// Creating scopes with a non-`GUARANTEED_ALLOCATED` bump is not possible.
    /// ```compile_fail,E0599
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// let mut bump: Bump<Global, 1, true, false> = Bump::unallocated();
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
    /// let bump: Bump<Global, 1, true, false> = Bump::unallocated();
    /// let mut bump = bump.try_guaranteed_allocated()?;
    ///
    /// bump.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_guaranteed_allocated(self) -> Result<Bump<A, MIN_ALIGN, UP>, AllocError> {
        self.generic_guaranteed_allocated()
    }

    #[inline(always)]
    fn generic_guaranteed_allocated<E: ErrorBehavior>(self) -> Result<Bump<A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { transmute(self) })
    }

    /// Borrows `Bump` as a [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// Unlike [`guaranteed_allocated_mut`] this method is usually not very useful, since an ***immutable***
    /// guaranteed allocated `Bump` does not provide any new api.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// [`guaranteed_allocated_mut`]: Self::guaranteed_allocated_mut
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn guaranteed_allocated_ref(&self) -> &Bump<A, MIN_ALIGN, UP> {
        panic_on_error(self.generic_guaranteed_allocated_ref())
    }

    /// Borrows `Bump` as an [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// Unlike [`try_guaranteed_allocated_mut`] this method is usually not very useful, since an ***immutable***
    /// guaranteed allocated `Bump` does not provide any new api.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// [`try_guaranteed_allocated_mut`]: Self::try_guaranteed_allocated_mut
    #[inline(always)]
    pub fn try_guaranteed_allocated_ref(&self) -> Result<&Bump<A, MIN_ALIGN, UP>, AllocError> {
        self.generic_guaranteed_allocated_ref()
    }

    #[inline(always)]
    fn generic_guaranteed_allocated_ref<E: ErrorBehavior>(&self) -> Result<&Bump<A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { transmute_ref(self) })
    }

    /// Mutably borrows `Bump` as a [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// Creating scopes with a non-`GUARANTEED_ALLOCATED` bump is not possible.
    /// ```compile_fail,E0599
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// let mut bump: Bump<Global, 1, true, false> = Bump::unallocated();
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
    /// let mut bump: Bump<Global, 1, true, false> = Bump::unallocated();
    ///
    /// bump.guaranteed_allocated_mut().scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn guaranteed_allocated_mut(&mut self) -> &mut Bump<A, MIN_ALIGN, UP> {
        panic_on_error(self.generic_guaranteed_allocated_mut())
    }

    /// Mutably borrows `Bump` as an [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Errors
    ///
    /// Errors if the allocation fails.
    ///
    /// # Examples
    ///
    /// Creating scopes with a non-`GUARANTEED_ALLOCATED` bump is not possible.
    /// ```compile_fail,E0599
    /// # use bump_scope::Bump;
    /// # use bump_scope::alloc::Global;
    /// let mut bump: Bump<Global, 1, true, false> = Bump::unallocated();
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
    /// let mut bump: Bump<Global, 1, true, false> = Bump::unallocated();
    ///
    /// bump.try_guaranteed_allocated_mut()?.scoped(|bump| {
    ///     // ...
    ///     # _ = bump;
    /// });
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_guaranteed_allocated_mut(&mut self) -> Result<&mut Bump<A, MIN_ALIGN, UP>, AllocError> {
        self.generic_guaranteed_allocated_mut()
    }

    #[inline(always)]
    fn generic_guaranteed_allocated_mut<E: ErrorBehavior>(&mut self) -> Result<&mut Bump<A, MIN_ALIGN, UP>, E> {
        self.as_scope().ensure_allocated()?;
        Ok(unsafe { transmute_mut(self) })
    }

    /// Converts this `BumpScope` into a ***not*** [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    #[inline(always)]
    pub fn not_guaranteed_allocated(self) -> Bump<A, MIN_ALIGN, UP, false>
    where
        A: Default,
    {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute(self) }
    }

    /// Borrows `Bump` as a ***not*** [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// Note that it's not possible to mutably borrow as a not guaranteed allocated bump allocator. That's because
    /// a user could `mem::swap` it with an actual unallocated bump allocator which in turn would make `&mut self`
    /// unallocated.
    #[inline(always)]
    pub fn not_guaranteed_allocated_ref(&self) -> &Bump<A, MIN_ALIGN, UP, false>
    where
        A: Default,
    {
        // SAFETY: it's always valid to interpret a guaranteed allocated as a non guaranteed allocated
        unsafe { transmute_ref(self) }
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

impl<'b, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    From<&'b Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>> for &'b BumpScope<'b, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn from(value: &'b Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        value.as_scope()
    }
}

impl<'b, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    From<&'b mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>
    for &'b mut BumpScope<'b, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    #[inline(always)]
    fn from(value: &'b mut Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>) -> Self {
        value.as_mut_scope()
    }
}

/// Functions to allocate. Available as fallible or infallible.
impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
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
    pub fn alloc<T>(&self, value: T) -> BumpBox<'_, T> {
        self.as_scope().alloc(value)
    }

    /// Allocate an object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc(123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc<T>(&self, value: T) -> Result<BumpBox<'_, T>, AllocError> {
        self.as_scope().try_alloc(value)
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
    pub fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> BumpBox<'_, T> {
        self.as_scope().alloc_with(f)
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
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_with(|| 123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<'_, T>, AllocError> {
        self.as_scope().try_alloc_with(f)
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[alloc_with](Self::alloc_with)(T::default)</code>.
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
    pub fn alloc_default<T: Default>(&self) -> BumpBox<'_, T> {
        self.as_scope().alloc_default()
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[try_alloc_with](Self::try_alloc_with)(T::default)</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_default()?;
    /// assert_eq!(allocated, 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_default<T: Default>(&self) -> Result<BumpBox<'_, T>, AllocError> {
        self.as_scope().try_alloc_default()
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
    pub fn alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> BumpBox<'_, [T]> {
        self.as_scope().alloc_slice_move(slice)
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
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
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> Result<BumpBox<'_, [T]>, AllocError> {
        self.as_scope().try_alloc_slice_move(slice)
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
    pub fn alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> BumpBox<'_, [T]> {
        self.as_scope().alloc_slice_copy(slice)
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_copy(&[1, 2, 3])?;
    /// assert_eq!(allocated, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> Result<BumpBox<'_, [T]>, AllocError> {
        self.as_scope().try_alloc_slice_copy(slice)
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
    pub fn alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> BumpBox<'_, [T]> {
        self.as_scope().alloc_slice_clone(slice)
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_clone(&[String::from("a"), String::from("b")])?;
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'_, [T]>, AllocError> {
        self.as_scope().try_alloc_slice_clone(slice)
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
    pub fn alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> BumpBox<'_, [T]> {
        self.as_scope().alloc_slice_fill(len, value)
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_fill(3, "ho")?;
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> Result<BumpBox<'_, [T]>, AllocError> {
        self.as_scope().try_alloc_slice_fill(len, value)
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`alloc_slice_fill`](Self::alloc_slice_fill). If you want to use the [`Default`]
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
    pub fn alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> BumpBox<'_, [T]> {
        self.as_scope().alloc_slice_fill_with(len, f)
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`try_alloc_slice_fill`](Self::try_alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_slice_fill_with::<i32>(3, Default::default)?;
    /// assert_eq!(allocated, [0, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<'_, [T]>, AllocError> {
        self.as_scope().try_alloc_slice_fill_with(len, f)
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
    pub fn alloc_str(&self, src: &str) -> BumpBox<'_, str> {
        self.as_scope().alloc_str(src)
    }

    /// Allocate a `str`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_str("Hello, world!")?;
    /// assert_eq!(allocated, "Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_str(&self, src: &str) -> Result<BumpBox<'_, str>, AllocError> {
        self.as_scope().try_alloc_str(src)
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`alloc_fmt_mut`](Self::alloc_fmt_mut)
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
    pub fn alloc_fmt(&self, args: fmt::Arguments) -> BumpBox<'_, str> {
        self.as_scope().alloc_fmt(args)
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_fmt_mut`](Self::try_alloc_fmt_mut)
    /// instead for better performance.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_fmt(&self, args: fmt::Arguments) -> Result<BumpBox<'_, str>, AllocError> {
        self.as_scope().try_alloc_fmt(args)
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`alloc_fmt`](Self::alloc_fmt).
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
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fmt_mut(&mut self, args: fmt::Arguments) -> BumpBox<'_, str> {
        self.as_mut_scope().alloc_fmt_mut(args)
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_fmt`](Self::try_alloc_fmt).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_fmt_mut(&mut self, args: fmt::Arguments) -> Result<BumpBox<'_, str>, AllocError> {
        self.as_mut_scope().try_alloc_fmt_mut(args)
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
    pub fn alloc_cstr(&self, src: &CStr) -> &CStr {
        self.as_scope().alloc_cstr(src)
    }

    /// Allocate a `CStr`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_cstr(c"Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr(&self, src: &CStr) -> Result<&CStr, AllocError> {
        self.as_scope().try_alloc_cstr(src)
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
    pub fn alloc_cstr_from_str(&self, src: &str) -> &CStr {
        self.as_scope().alloc_cstr_from_str(src)
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
    /// # let bump: Bump = Bump::try_new()?;
    /// let allocated = bump.try_alloc_cstr_from_str("Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.try_alloc_cstr_from_str("abc\0def")?;
    /// assert_eq!(allocated, c"abc");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr_from_str(&self, src: &str) -> Result<&CStr, AllocError> {
        self.as_scope().try_alloc_cstr_from_str(src)
    }
    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`alloc_cstr_fmt_mut`](Self::alloc_cstr_fmt_mut)
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
    pub fn alloc_cstr_fmt(&self, args: fmt::Arguments) -> &CStr {
        self.as_scope().alloc_cstr_fmt(args)
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_cstr_fmt_mut`](Self::try_alloc_cstr_fmt_mut)
    /// instead for better performance.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr_fmt(&self, args: fmt::Arguments) -> Result<&CStr, AllocError> {
        self.as_scope().try_alloc_cstr_fmt(args)
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`alloc_cstr_fmt`](Self::alloc_cstr_fmt).
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
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> &CStr {
        self.as_mut_scope().alloc_cstr_fmt_mut(args)
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_cstr_fmt`](Self::try_alloc_cstr_fmt).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt_mut(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> Result<&CStr, AllocError> {
        self.as_mut_scope().try_alloc_cstr_fmt_mut(args)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`alloc_iter_mut`].
    ///
    /// [`alloc_iter_exact`]: Self::alloc_iter_exact
    /// [`alloc_iter_mut`]: Self::alloc_iter_mut
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
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> BumpBox<'_, [T]> {
        self.as_scope().alloc_iter(iter)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`try_alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`try_alloc_iter_mut`].
    ///
    /// [`try_alloc_iter_exact`]: Self::try_alloc_iter_exact
    /// [`try_alloc_iter_mut`]: Self::try_alloc_iter_mut
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<'_, [T]>, AllocError> {
        self.as_scope().try_alloc_iter(iter)
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_exact([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter_exact<T, I>(&self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> BumpBox<'_, [T]>
    where
        I: ExactSizeIterator<Item = T>,
    {
        self.as_scope().alloc_iter_exact(iter)
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_exact([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_iter_exact<T, I>(
        &self,
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> Result<BumpBox<'_, [T]>, AllocError>
    where
        I: ExactSizeIterator<Item = T>,
    {
        self.as_scope().try_alloc_iter_exact(iter)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`alloc_iter`](Self::alloc_iter).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`](Self::alloc_iter_mut_rev) instead.
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
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter_mut<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<'_, [T]> {
        self.as_mut_scope().alloc_iter_mut(iter)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_iter`](Self::try_alloc_iter).
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`](Self::alloc_iter_mut_rev) instead.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_mut([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_iter_mut<T>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<'_, [T]>, AllocError> {
        self.as_mut_scope().try_alloc_iter_mut(iter)
    }

    /// Allocate elements of an iterator into a slice in reverse order.
    ///
    /// Compared to [`alloc_iter_mut`] this function is more performant
    /// for downwards bumping allocators as the allocation for the vector can be shrunk in place
    /// without any `ptr::copy`.
    ///
    /// The reverse is true when upwards allocating. In that case it's better to use [`alloc_iter_mut`] to prevent
    /// the `ptr::copy`.
    ///
    /// [`alloc_iter_mut`]: Self::alloc_iter_mut
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut_rev([1, 2, 3]);
    /// assert_eq!(slice, [3, 2, 1]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_iter_mut_rev<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<'_, [T]> {
        self.as_mut_scope().alloc_iter_mut_rev(iter)
    }

    /// Allocate elements of an iterator into a slice in reverse order.
    ///
    /// Compared to [`try_alloc_iter_mut`] this function is more performant
    /// for downwards bumping allocators as the allocation for the vector can be shrunk in place
    /// without any `ptr::copy`.
    ///
    /// The reverse is true when upwards allocating. In that case it's better to use [`try_alloc_iter_mut`] to prevent
    /// the `ptr::copy`.
    ///
    /// [`try_alloc_iter_mut`]: Self::try_alloc_iter_mut
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_mut_rev([1, 2, 3])?;
    /// assert_eq!(slice, [3, 2, 1]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_iter_mut_rev<T>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<'_, [T]>, AllocError> {
        self.as_mut_scope().try_alloc_iter_mut_rev(iter)
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`init`](BumpBox::init) or unsafely with [`assume_init`](BumpBox::assume_init).
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
    pub fn alloc_uninit<T>(&self) -> BumpBox<'_, MaybeUninit<T>> {
        self.as_scope().alloc_uninit()
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`init`](BumpBox::init) or unsafely with [`assume_init`](BumpBox::assume_init).
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
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut uninit = bump.try_alloc_uninit()?;
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_uninit<T>(&self) -> Result<BumpBox<'_, MaybeUninit<T>>, AllocError> {
        self.as_scope().try_alloc_uninit()
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
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
    pub fn alloc_uninit_slice<T>(&self, len: usize) -> BumpBox<'_, [MaybeUninit<T>]> {
        self.as_scope().alloc_uninit_slice(len)
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
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
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
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
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_uninit_slice<T>(&self, len: usize) -> Result<BumpBox<'_, [MaybeUninit<T>]>, AllocError> {
        self.as_scope().try_alloc_uninit_slice(len)
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// This is just like [`alloc_uninit_slice`](Self::alloc_uninit_slice) but uses a `slice` to provide the `len`.
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
    pub fn alloc_uninit_slice_for<T>(&self, slice: &[T]) -> BumpBox<'_, [MaybeUninit<T>]> {
        self.as_scope().alloc_uninit_slice_for(slice)
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// This is just like [`try_alloc_uninit_slice`](Self::try_alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// let slice = &[1, 2, 3];
    /// let uninit_slice = bump.try_alloc_uninit_slice_for(slice)?;
    /// assert_eq!(uninit_slice.len(), 3);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_alloc_uninit_slice_for<T>(&self, slice: &[T]) -> Result<BumpBox<'_, [MaybeUninit<T>]>, AllocError> {
        self.as_scope().try_alloc_uninit_slice_for(slice)
    }

    /// Allocate a [`FixedBumpVec`] with the given `capacity`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// # #[allow(deprecated)]
    /// let mut values = bump.alloc_fixed_vec(3);
    /// values.push(1);
    /// values.push(2);
    /// values.push(3);
    /// assert_eq!(values, [1, 2, 3])
    /// ```
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpVec::with_capacity_in()` instead"]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fixed_vec<T>(&self, capacity: usize) -> FixedBumpVec<'_, T> {
        #[allow(deprecated)]
        self.as_scope().alloc_fixed_vec(capacity)
    }

    /// Allocate a [`FixedBumpVec`] with the given `capacity`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// # #[allow(deprecated)]
    /// let mut values = bump.try_alloc_fixed_vec(3)?;
    /// values.push(1);
    /// values.push(2);
    /// values.push(3);
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpVec::try_with_capacity_in()` instead"]
    #[inline(always)]
    pub fn try_alloc_fixed_vec<T>(&self, capacity: usize) -> Result<FixedBumpVec<'_, T>, AllocError> {
        #[allow(deprecated)]
        self.as_scope().try_alloc_fixed_vec(capacity)
    }

    /// Allocate a [`FixedBumpString`] with the given `capacity` in bytes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// # #[allow(deprecated)]
    /// let mut string = bump.alloc_fixed_string(13);
    /// string.push_str("Hello,");
    /// string.push_str(" world!");
    /// assert_eq!(string, "Hello, world!");
    /// ```
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpString::with_capacity_in()` instead"]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_fixed_string(&self, capacity: usize) -> FixedBumpString<'_> {
        #[allow(deprecated)]
        self.as_scope().alloc_fixed_string(capacity)
    }

    /// Allocate a [`FixedBumpString`] with the given `capacity` in bytes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::try_new()?;
    /// # #[allow(deprecated)]
    /// let mut string = bump.try_alloc_fixed_string(13)?;
    /// string.push_str("Hello,");
    /// string.push_str(" world!");
    /// assert_eq!(string, "Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[doc(hidden)]
    #[deprecated = "use `FixedBumpString::try_with_capacity_in()` instead"]
    #[inline(always)]
    pub fn try_alloc_fixed_string(&self, capacity: usize) -> Result<FixedBumpString<'_>, AllocError> {
        #[allow(deprecated)]
        self.as_scope().try_alloc_fixed_string(capacity)
    }

    /// Allocates memory as described by the given `Layout`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn alloc_layout(&self, layout: Layout) -> NonNull<u8> {
        self.as_scope().alloc_layout(layout)
    }

    /// Allocates memory as described by the given `Layout`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    #[inline(always)]
    pub fn try_alloc_layout(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        self.as_scope().try_alloc_layout(layout)
    }

    /// Drops an allocated value and attempts to free its memory.
    ///
    /// The memory can only be freed if this is the last allocation.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let boxed = bump.alloc(3i32);
    /// assert_eq!(bump.stats().allocated(), 4);
    /// bump.dealloc(boxed);
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    pub fn dealloc<T: ?Sized>(&self, boxed: BumpBox<T>) {
        self.as_scope().dealloc(boxed);
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_bytes`, <code>self.[stats](Self::stats)().[remaining](Stats::remaining)()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that these additional bytes are not necessarily in one contiguous region but
    /// might be spread out among many chunks.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump};
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve_bytes(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn reserve_bytes(&self, additional: usize) {
        self.as_scope().reserve_bytes(additional);
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_bytes`, <code>self.[stats](Self::stats)().[remaining](Stats::remaining)()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let bump: Bump = Bump::try_new()?;
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.try_reserve_bytes(4096)?;
    /// assert!(bump.stats().capacity() >= 4096);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    pub fn try_reserve_bytes(&self, additional: usize) -> Result<(), AllocError> {
        self.as_scope().try_reserve_bytes(additional)
    }
}

/// Functions to allocate. Available as fallible or infallible.
///
/// These require a [guaranteed allocated](crate#guaranteed_allocated-parameter) bump allocator.
impl<A, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator,
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
    #[allow(clippy::missing_errors_doc)]
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
    #[allow(clippy::missing_errors_doc)]
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
