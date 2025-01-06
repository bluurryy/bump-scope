use core::{
    alloc::Layout,
    cell::Cell,
    fmt::{self, Debug},
    mem::{self, transmute, ManuallyDrop},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::NonNull,
};

use allocator_api2::alloc::AllocError;

use crate::{
    bump_common_methods,
    chunk_size::ChunkSize,
    error_behavior_generic_methods_allocation_failure,
    polyfill::{pointer, transmute_mut, transmute_ref},
    unallocated_chunk_header, BaseAllocator, BumpScope, BumpScopeGuardRoot, Checkpoint, ErrorBehavior, MinimumAlignment,
    RawChunk, Stats, SupportedMinimumAlignment,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

macro_rules! bump_declaration {
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
        /// - slices: [`alloc_slice_clone`], [`alloc_slice_copy`], [`alloc_slice_fill`], [`alloc_slice_fill_with`], [`alloc_zeroed_slice`]
        /// - slices from an iterator: [`alloc_iter`], [`alloc_iter_exact`], [`alloc_iter_mut`], [`alloc_iter_mut_rev`]
        /// - uninitialized values: [`alloc_uninit`], [`alloc_uninit_slice`], [`alloc_uninit_slice_for`]
        ///
        ///   which can then be conveniently initialized by the [`init*` methods of `BumpBox`](crate::BumpBox#bumpbox-has-a-lot-of-methods).
        /// - fixed collections: [`alloc_fixed_vec`], [`alloc_fixed_string`]
        /// - results: [`alloc_try_with`], [`alloc_try_with_mut`]
        ///
        /// ## Collections
        /// A `Bump` (and [`BumpScope`]) can be used to allocate collections of this crate...
        /// ```
        /// use bump_scope::{ Bump, BumpString };
        /// let bump: Bump = Bump::new();
        ///
        /// let mut string = BumpString::new_in(&bump);
        /// string.push_str("Hello,");
        /// string.push_str(" world!");
        /// ```
        ///
        /// ... and collections from crates that use `allocator_api2`'s [`Allocator`](allocator_api2::alloc::Allocator) like [hashbrown](https://docs.rs/hashbrown)'s [`HashMap`](https://docs.rs/hashbrown/latest/hashbrown/struct.HashMap.html):
        /// ```
        /// use bump_scope::{ Bump, BumpString };
        /// use hashbrown::HashMap;
        ///
        /// let bump: Bump = Bump::new();
        /// let mut map = HashMap::new_in(&bump);
        /// map.insert("tau", 6.283);
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
        /// use std::collections::{ VecDeque, BTreeMap, LinkedList };
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
        /// [`alloc_zeroed`]: Self::alloc_zeroed
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
        /// [`alloc_slice_clone`]: Self::alloc_slice_clone
        /// [`alloc_slice_copy`]: Self::alloc_slice_copy
        /// [`alloc_slice_fill`]: Self::alloc_slice_fill
        /// [`alloc_slice_fill_with`]: Self::alloc_slice_fill_with
        /// [`alloc_zeroed_slice`]: Self::alloc_zeroed_slice
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
        /// [`alloc_fixed_vec`]: Self::alloc_fixed_vec
        /// [`alloc_fixed_string`]: Self::alloc_fixed_string
        ///
        /// [`alloc_try_with`]: Self::alloc_try_with
        /// [`alloc_try_with_mut`]: Self::alloc_try_with_mut
        ///
        /// ## Scopes and Checkpoints
        ///
        /// See [Scopes and Checkpoints](crate#scopes-and-checkpoints).
        ///
        /// # Gotchas
        ///
        /// Allocating directly on a `Bump` is not compatible with entering bump scopes at the same time:
        ///
        /// ```compile_fail,E0502
        /// # use bump_scope::Bump;
        /// let mut bump: Bump = Bump::new();
        ///
        /// let one = bump.alloc(1);
        ///
        /// bump.scoped(|bump| {
        ///     // whatever
        /// });
        /// ```
        /// Instead convert it to a [`BumpScope`] first:
        /// ```
        /// # use bump_scope::Bump;
        /// let mut bump: Bump = Bump::new();
        /// let bump = bump.as_mut_scope();
        ///
        /// let one = bump.alloc(1);
        ///
        /// bump.scoped(|bump| {
        ///     // whatever
        /// });
        /// ```
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

crate::maybe_default_allocator!(bump_declaration);

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
        if self.is_unallocated() {
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
        self.stats().debug_format("Bump", f)
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

impl<A, const MIN_ALIGN: usize, const UP: bool> Bump<A, MIN_ALIGN, UP, false>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<false>,
{
    /// Constructs a new `Bump` without doing any allocations.
    ///
    /// The resulting `Bump` will have its [`GUARANTEED_ALLOCATED` parameter set to `false`](crate#guaranteed_allocated-parameter).
    /// Such a `Bump` is unable to create a scope with `scoped` or `scope_guard`.
    /// It has to first be converted into a guaranteed allocated `Bump` using [`guaranteed_allocated`](Bump::guaranteed_allocated), [`guaranteed_allocated_ref`](Bump::guaranteed_allocated_ref) or [`guaranteed_allocated_mut`](Bump::guaranteed_allocated_mut).
    ///
    /// **This function is `const` starting from rust version 1.83.**
    #[must_use]
    #[rustversion::attr(since(1.83), const)]
    pub fn unallocated() -> Self {
        Self {
            chunk: Cell::new(unsafe { RawChunk::from_header(unallocated_chunk_header().cast()) }),
        }
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED> + Default,
{
    error_behavior_generic_methods_allocation_failure! {
        /// Constructs a new `Bump` with a default size hint for the first chunk.
        ///
        impl
        /// This is equivalent to <code>[with_size](Bump::with_size)(512)</code>.
        #[must_use]
        for fn new
        /// This is equivalent to <code>[try_with_size](Bump::try_with_size)(512)</code>.
        for fn try_new
        #[inline]
        use fn generic_new() -> Self {
            Self::generic_new_in(Default::default())
        }

        /// Constructs a new `Bump` with a size hint for the first chunk.
        ///
        impl
        #[must_use]
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
        do examples
        /// ```
        /// use bump_scope::Bump;
        ///
        /// // `Bump` with a roughly 1 Mebibyte sized chunk
        /// let bump_1mib: Bump = Bump::with_size(1024 * 1024);
        /// ```
        for fn with_size
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
        do examples
        /// ```
        /// use bump_scope::Bump;
        ///
        /// // `Bump` with a roughly 1 Mebibyte sized chunk
        /// let bump_1mib: Bump = Bump::try_with_size(1024 * 1024)?;
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_with_size
        #[inline]
        use fn generic_with_size(size: usize) -> Self {
            Self::generic_with_size_in(size, Default::default())
        }

        impl
        #[must_use]
        /// Constructs a new `Bump` with at least enough space for `layout`.
        ///
        /// To construct a `Bump` with a size hint use <code>[with_size](Bump::with_size)</code> instead.
        do examples
        /// ```
        /// use bump_scope::Bump;
        /// use core::alloc::Layout;
        ///
        /// let layout = Layout::array::<u8>(1234).unwrap();
        /// let bump: Bump = Bump::with_capacity(layout);
        /// assert!(bump.stats().capacity() >= layout.size());
        /// ```
        for fn with_capacity
        /// Constructs a new `Bump` with at least enough space for `layout`.
        ///
        /// To construct a `Bump` with a size hint use <code>[try_with_size](Bump::try_with_size)</code> instead.
        do examples
        /// ```
        /// use bump_scope::Bump;
        /// use core::alloc::Layout;
        ///
        /// let layout = Layout::array::<u8>(1234).unwrap();
        /// let bump: Bump = Bump::try_with_capacity(layout)?;
        /// assert!(bump.stats().capacity() >= layout.size());
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_with_capacity
        #[inline]
        use fn generic_with_capacity(layout: Layout) -> Self {
            Self::generic_with_capacity_in(layout, Default::default())
        }
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
    /// ```
    /// # #![feature(pointer_is_aligned_to)]
    /// # use bump_scope::{ Bump, Stats };
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
    /// The [`scoped` Gotcha][gotcha] applies to `aligned` too,
    /// so you might want to turn this into a [`BumpScope`] using [`as_mut_scope`] before calling `aligned`.
    ///
    /// See [`BumpScope::aligned`] for examples.
    ///
    /// [gotcha]: Self#gotchas
    /// [`as_mut_scope`]: Self::as_mut_scope
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
    pub fn scope_guard(&mut self) -> BumpScopeGuardRoot<A, MIN_ALIGN, UP> {
        BumpScopeGuardRoot::new(self)
    }

    /// Creates a checkpoint of the current bump position.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// let mut bump: Bump = Bump::new();
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.stats().allocated(), 5);
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint::new(self.chunk.get())
    }

    /// Resets the bump position to a previously created checkpoint. The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    /// - the checkpoint must have been created by this bump allocator
    /// - the bump allocator must not have been [`reset`](crate::Bump::reset) since creation of this checkpoint
    /// - there must be no references to allocations made since creation of this checkpoint
    #[inline]
    pub unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        self.as_scope().reset_to(checkpoint);
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool>
    Bump<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    error_behavior_generic_methods_allocation_failure! {
        /// Constructs a new `Bump` with a default size hint for the first chunk.
        ///
        impl
        /// This is equivalent to <code>[with_size_in](Bump::with_size_in)(512, allocator)</code>.
        for fn new_in
        /// This is equivalent to <code>[try_with_size_in](Bump::try_with_size_in)(512, allocator)</code>.
        for fn try_new_in
        #[inline]
        use fn generic_new_in(allocator: A) -> Self {
            Ok(Self {
                chunk: Cell::new(RawChunk::new_in(
                    ChunkSize::DEFAULT_START,
                    None,
                    allocator,
                )?),
            })
        }

        /// Constructs a new `Bump` with a size hint for the first chunk.
        ///
        impl
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
        do examples
        /// ```
        /// use bump_scope::Bump;
        /// use bump_scope::allocator_api2::alloc::Global;
        ///
        /// // `Bump` with a roughly 1 Mebibyte sized chunk
        /// let bump_1mib: Bump = Bump::with_size_in(1024 * 1024, Global);
        /// ```
        for fn with_size_in
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
        do examples
        /// ```
        /// use bump_scope::Bump;
        /// use bump_scope::allocator_api2::alloc::Global;
        ///
        /// // `Bump` with a roughly 1 Mebibyte sized chunk
        /// let bump_1mib: Bump = Bump::try_with_size_in(1024 * 1024, Global)?;
        /// # Ok::<(), bump_scope::allocator_api2::alloc::AllocError>(())
        /// ```
        for fn try_with_size_in
        #[inline]
        use fn generic_with_size_in(size: usize, allocator: A) -> Self {
            Ok(Self {
                chunk: Cell::new(RawChunk::new_in(
                    ChunkSize::new(size).ok_or_else(B::capacity_overflow)?,
                    None,
                    allocator,
                )?),
            })
        }

        impl
        /// Constructs a new `Bump` with at least enough space for `layout`.
        ///
        /// To construct a `Bump` with a size hint use <code>[with_size_in](Bump::with_size_in)</code> instead.
        for fn with_capacity_in
        /// Constructs a new `Bump` with at least enough space for `layout`.
        ///
        /// To construct a `Bump` with a size hint use <code>[try_with_size_in](Bump::try_with_size_in)</code> instead.
        for fn try_with_capacity_in
        #[inline]
        use fn generic_with_capacity_in(layout: Layout, allocator: A) -> Self {
            Ok(Self {
                chunk: Cell::new(RawChunk::new_in(
                    ChunkSize::for_capacity(layout).ok_or_else(B::capacity_overflow)?,
                    None,
                    allocator,
                )?),
            })
        }
    }

    // This needs `&mut self` to make sure that no allocations are alive.
    /// Deallocates every chunk but the newest, which is also the biggest.
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

    bump_common_methods!(false);

    /// Returns this `&Bump` as a `&BumpScope`.
    #[inline(always)]
    pub fn as_scope(&self) -> &BumpScope<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &*pointer::from_ref(self).cast() }
    }

    /// Returns this `&mut Bump` as a `&mut BumpScope`.
    #[inline(always)]
    pub fn as_mut_scope(&mut self) -> &mut BumpScope<A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED> {
        // SAFETY: `Bump` and `BumpScope` both have the layout of `Cell<RawChunk>`
        //         `BumpScope`'s api is a subset of `Bump`'s
        unsafe { &mut *pointer::from_mut(self).cast() }
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
    /// **This can not decrease the alignment.** Trying to decrease alignment will result in a compile error.
    /// You can use [`aligned`](Self::aligned) or [`scoped_aligned`](Self::scoped_aligned) to decrease the alignment."
    ///
    /// To decrease alignment we need to ensure that we return to our original alignment.
    /// That can only be guaranteed by a function taking a closure like the ones mentioned above.
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
        &mut *pointer::from_mut(self).cast::<Bump<A, NEW_MIN_ALIGN, UP, GUARANTEED_ALLOCATED>>()
    }

    /// Converts this `Bump` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn guaranteed_allocated(self) -> Bump<A, MIN_ALIGN, UP> {
        panic_on_error(self.generic_guaranteed_allocated())
    }

    /// Converts this `Bump` into a [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
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
    /// # Panics
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn guaranteed_allocated_ref(&self) -> &Bump<A, MIN_ALIGN, UP> {
        panic_on_error(self.generic_guaranteed_allocated_ref())
    }

    /// Borrows `Bump` as an [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
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
    /// Panics if the allocation fails.
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn guaranteed_allocated_mut(&mut self) -> &mut Bump<A, MIN_ALIGN, UP> {
        panic_on_error(self.generic_guaranteed_allocated_mut())
    }

    /// Mutably borrows `Bump` as an [guaranteed allocated](crate#guaranteed_allocated-parameter) `Bump`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
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
        this.chunk.get().header_ptr().cast()
    }

    /// Converts the raw pointer that was created with [`into_raw`](Bump::into_raw) back into a `Bump`.
    ///
    /// # Safety
    /// - `ptr` must have been created with `Self::into_raw`.
    /// - This function must only be called once with this `ptr`.
    #[inline]
    #[must_use]
    pub unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        let chunk = Cell::new(RawChunk::from_header(ptr.cast()));
        Self { chunk }
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
