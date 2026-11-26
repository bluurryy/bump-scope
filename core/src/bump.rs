use core::{
    alloc::{AllocError, Allocator, Layout},
    cell::Cell,
    fmt,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::{
    ErrorMode,
    base_allocator_storage::BaseAllocatorStorage,
    bump_scope::BumpScope,
    chunk_size::{ChunkLayout, ChunkSizeCapacity, ChunkSizeDefault, ChunkSizeHint},
    chunk_types::{AllocatedChunk, Chunk},
    config::{BumpConfig, config},
    destructure::destructure,
    util::{const_assert_upwards_matches, transmute_mut, transmute_ref, transmute_value},
};

/// The bump allocator.
///
/// # Generic parameters
/// - **`C`** — the bump allocator config, see [`config`](crate::config)
/// - **`A`** — the base allocator, defaults to `Global`
///
/// # Constructors
/// - without allocating a chunk: [`const_new_in`]
/// - with the [`default_initial_chunk_size`]: [`new_in`]
/// - with a custom initial chunk size: [`with_size_in`]
/// - with capacity for a layout: [`with_capacity_in`]
///
/// [`const_new_in`]: Bump::const_new_in
/// [`new_in`]: Bump::new_in
/// [`with_size_in`]: Bump::with_size_in
/// [`with_capacity_in`]: Bump::with_capacity_in
///
/// [`default_initial_chunk_size`]: crate::config::BumpConfigBuilder::default_initial_chunk_size
pub struct Bump<const C: BumpConfig, A: Allocator> {
    scope: BumpScope<C, A>,
}

/// Constructors.
impl<const C: BumpConfig, A: Allocator> Bump<C, A> {
    /// Constructs a new `Bump` without allocating a chunk.
    ///
    /// # Panics
    /// Panics when not [`const_constructible`].
    ///
    /// # Examples
    /// ```
    /// # extern crate alloc;
    /// use alloc::alloc::Global;
    /// use bump_scope_core::config::DEFAULT_BUMP_CONFIG;
    ///
    /// type Bump = bump_scope_core::Bump<DEFAULT_BUMP_CONFIG, Global>;
    ///
    /// let bump = Bump::const_new_in(Global);
    /// # _ = bump;
    /// ```
    ///
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    #[inline(always)]
    pub const fn const_new_in(allocator: A) -> Self {
        assert!(config!(const_constructible));

        Self {
            scope: BumpScope {
                chunk: Cell::new(Chunk::CONST_CONSTRUCTED),
                allocator: BaseAllocatorStorage::new(allocator),
            },
        }
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[with_size_in][]([default_initial_chunk_size], allocator)</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # extern crate alloc;
    /// use alloc::alloc::Global;
    /// use bump_scope_core::config::DEFAULT_BUMP_CONFIG;
    ///
    /// type Bump = bump_scope_core::Bump<DEFAULT_BUMP_CONFIG, Global>;
    ///
    /// let bump = Bump::new_in(Global)?;
    /// # _ = bump;
    /// # Ok::<(), !>(())
    /// ```
    ///
    /// [with_size_in]: Bump::with_size_in
    /// [default_initial_chunk_size]: crate::config::BumpConfigBuilder::default_initial_chunk_size
    #[inline(always)]
    pub fn new_in<E: ErrorMode>(allocator: A) -> Result<Self, E> {
        Self::allocated_new(ChunkSizeDefault, allocator)
    }

    /// Constructs a new `Bump` with a size hint for the first chunk.
    ///
    /// If you want to ensure a specific capacity, use [`with_capacity_in`](Self::with_capacity_in) instead.
    ///
    /// The layout size that is finally requested from the base allocator may be larger due to alignment and because
    /// we currently round it up to either the next power of two or the next multiple of `0x1000`, whichever is smaller.
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
    /// # extern crate alloc;
    /// use alloc::alloc::Global;
    /// use bump_scope_core::config::DEFAULT_BUMP_CONFIG;
    ///
    /// type Bump = bump_scope_core::Bump<DEFAULT_BUMP_CONFIG, Global>;
    ///
    /// // `Bump` with a 1 Mebibyte sized chunk
    /// let bump_1mib = Bump::with_size_in(1024 * 1024, Global)?;
    /// # _ = bump_1mib;
    /// # Ok::<(), !>(())
    /// ```
    #[inline(always)]
    pub fn with_size_in<E: ErrorMode>(size: usize, allocator: A) -> Result<Self, E> {
        Self::allocated_new(ChunkSizeHint(size), allocator)
    }

    /// Constructs a new `Bump` with at least enough space for `layout`.
    ///
    /// To construct a `Bump` with a size hint use <code>[with_size_in](Bump::with_size_in)</code> instead.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # extern crate alloc;
    /// use alloc::alloc::{Global, Layout};
    /// use bump_scope_core::config::DEFAULT_BUMP_CONFIG;
    ///
    /// type Bump = bump_scope_core::Bump<DEFAULT_BUMP_CONFIG, Global>;
    ///
    /// let layout = Layout::array::<u8>(1234).unwrap();
    /// let bump: Bump = Bump::with_capacity_in(layout, Global)?;
    /// assert!(bump.stats().capacity() >= layout.size());
    /// # Ok::<(), !>(())
    /// ```
    #[inline(always)]
    pub fn with_capacity_in<E: ErrorMode>(layout: Layout, allocator: A) -> Result<Self, E> {
        Self::allocated_new(ChunkSizeCapacity(layout), allocator)
    }

    #[inline(always)]
    fn allocated_new<E: ErrorMode>(size: impl ChunkLayout, allocator: A) -> Result<Self, E> {
        Ok(Self {
            scope: BumpScope {
                chunk: Cell::new(Chunk::from_allocated(AllocatedChunk::new_first::<E, A>(size, &allocator)?)),
                allocator: BaseAllocatorStorage::new(allocator),
            },
        })
    }
}

impl<const C: BumpConfig, A: Allocator> Drop for Bump<C, A> {
    fn drop(&mut self) {
        let Some(mut chunk) = self.chunk().as_allocated() else {
            return;
        };

        while let Some(prev) = chunk.prev() {
            chunk = prev;
        }

        while let Some(next) = chunk.next() {
            let ptr = chunk.chunk_start();
            let layout = chunk.layout();
            unsafe { self.allocator.deallocate(ptr, layout) };
            chunk = next;
        }

        let ptr = chunk.chunk_start();
        let layout = chunk.layout();
        unsafe { self.allocator.deallocate(ptr, layout) };
    }
}

impl<const C: BumpConfig, A: Allocator> fmt::Debug for Bump<C, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.stats().debug_format("Bump", f)
    }
}

/// # Checkpoint and reset.
impl<const C: BumpConfig, A: Allocator> Bump<C, A> {
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
    /// use bump_scope::{Bump, BumpAllocator};
    ///
    /// let mut scratch_allocator: Bump = Bump::new();
    /// let items = [1, 2, 3];
    ///
    /// for item in items {
    ///     // the scratch allocator is empty
    ///     assert_eq!(scratch_allocator.allocated(), 0);
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
    /// use bump_scope::{Bump, BumpAllocator};
    ///
    /// let mut bump: Bump = Bump::with_size(512);
    ///
    /// // won't fit in the first chunk, will be allocated in another chunk
    /// bump.alloc_uninit_slice::<u8>(600).into_ref();
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
    pub fn reset(&mut self) {
        let Some(mut chunk) = self.chunk().as_allocated() else {
            return;
        };

        // go to the first chunk
        while let Some(prev) = chunk.prev() {
            chunk = prev;
        }

        // deallocate all chunks except the last
        while let Some(next) = chunk.next() {
            let ptr = chunk.chunk_start();
            let layout = chunk.layout();
            unsafe { self.allocator.deallocate(ptr, layout) };
            chunk = next;
        }

        chunk.mut_clear_prev();
        chunk.reset();
        self.set_chunk(Chunk::from_allocated(chunk));
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
    /// use bump_scope::{Bump, BumpAllocator};
    ///
    /// let mut scratch_allocator: Bump = Bump::new();
    /// let items = [1, 2, 3];
    ///
    /// for item in items {
    ///     // the scratch allocator is empty
    ///     assert_eq!(scratch_allocator.allocated(), 0);
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
    /// use bump_scope::{Bump, BumpAllocator};
    ///
    /// let mut bump: Bump = Bump::with_size(512);
    ///
    /// // won't fit in the first chunk, will be allocated in another chunk
    /// bump.alloc_uninit_slice::<u8>(600).into_ref();
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
    /// assert_eq!(bump.allocated(), 0);
    ///
    /// // we are back at the first chunk
    /// assert!(bump.stats().current_chunk().unwrap().prev().is_none());
    /// ```
    pub fn reset_to_start(&mut self) {
        // SAFETY: by taking a mutable reference to self, we invalidate all memory blocks
        //         see: https://doc.rust-lang.org/nightly/std/alloc/trait.Allocator.html#invalidating-memory-blocks
        unsafe { self.scope.reset_to_start() }
    }
}

/// # Change config.
impl<const C: BumpConfig, A: Allocator> Bump<C, A> {
    /// Converts this `Bump` into a `Bump` with a new config.
    ///
    /// # Panics
    /// - Panics if [`claimable`] becomes false and [`self.is_claimed()`]
    /// - Panics if [`const_constructible`] becomes false and `self` has no chunks
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`self.is_claimed()`]: crate::BumpScope::is_claimed
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    #[inline]
    pub fn change_config<const NEW_C: BumpConfig>(self) -> Bump<NEW_C, A> {
        const_assert_upwards_matches::<C, NEW_C>();

        self.scope.assert_convertible_claimable::<NEW_C>();
        self.scope.assert_convertible_const_constructible::<NEW_C>();
        self.scope.align_to(config!(NEW_C, min_align));

        unsafe { transmute_value(self) }
    }

    /// Borrows this `Bump` with a new config.
    ///
    /// # Panics
    /// - Panics if [`const_constructible`] becomes false and `self` has no chunks
    ///
    /// # Compile-Time Assertions
    /// - Fails to compile if [`upwards`] changes
    /// - Fails to compile if [`claimable`] changes
    /// - Fails to compile if [`min_align`] changes
    ///
    /// [`upwards`]: crate::config::BumpConfigBuilder::upwards
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    /// [`min_align`]: crate::config::BumpConfigBuilder::min_align
    #[inline]
    pub fn ref_change_config<const NEW_C: BumpConfig>(&self) -> &Bump<NEW_C, A> {
        self.scope.ensure_satisfies_config_for_borrow::<NEW_C>();
        unsafe { transmute_ref(self) }
    }

    /// Borrows this `Bump` mutably with a new config.
    ///
    /// # Compile-Time Assertions
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
    pub fn mut_change_config<const NEW_C: BumpConfig>(&mut self) -> &mut Bump<NEW_C, A> {
        self.scope.ensure_satisfies_config_for_borrow_mut::<NEW_C>();
        unsafe { transmute_mut(self) }
    }

    /// Forwards to [`BumpScope::with_config`].
    #[inline(always)]
    pub fn with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<NEW_C, A>) -> R) -> R {
        self.scope.with_config(f)
    }

    /// Forwards to [`BumpScope::scoped_with_config`].
    #[inline(always)]
    pub fn scoped_with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<NEW_C, A>) -> R) -> R {
        self.scope.scoped_with_config::<NEW_C, R>(f)
    }
}

/// # Very low level apis.
impl<const C: BumpConfig, A: Allocator> Bump<C, A> {
    /// Converts this `Bump` into a raw pointer.
    ///
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// bump.alloc_str("Hello, ").into_ref();
    ///
    /// let (ptr, alloc) = bump.into_raw_parts();
    /// let bump: Bump = unsafe { Bump::from_raw_parts(ptr, alloc) };
    ///
    /// bump.alloc_str("World!").into_ref();
    /// # assert_eq!(bump.allocated(), 13);
    /// ```
    #[inline]
    #[must_use]
    pub fn into_raw_parts(self) -> (NonNull<()>, A) {
        destructure!(let Self { scope } = self);
        destructure!(let BumpScope<C, A> { chunk, allocator } = scope);
        let ptr = chunk.get().as_ptr();
        // SAFETY: `Bump` does store the base allocator
        let alloc = unsafe { allocator.into_inner() };
        (ptr, alloc)
    }

    /// Converts the raw pointer that was created with [`into_raw_parts`] back into a `Bump`.
    ///
    /// # Safety
    /// - `ptr` and `allocator` must come from the same call to [`into_raw_parts`].
    /// - This function must only be called once with this `ptr`.
    /// - The config must match the original one.
    ///
    /// [`into_raw_parts`]: Bump::into_raw_parts
    #[inline]
    #[must_use]
    pub unsafe fn from_raw_parts(ptr: NonNull<()>, allocator: A) -> Self {
        Self {
            scope: BumpScope {
                chunk: Cell::new(unsafe { Chunk::from_ptr(ptr.cast()) }),
                allocator: BaseAllocatorStorage::new(allocator),
            },
        }
    }
}

impl<const C: BumpConfig, A: Allocator> Deref for Bump<C, A> {
    type Target = BumpScope<C, A>;

    fn deref(&self) -> &Self::Target {
        &self.scope
    }
}

impl<const C: BumpConfig, A: Allocator> DerefMut for Bump<C, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.scope
    }
}

unsafe impl<const C: BumpConfig, A: Allocator> Allocator for Bump<C, A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.scope.allocate(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe {
            self.scope.deallocate(ptr, layout);
        };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.scope.grow(ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.scope.grow_zeroed(ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.scope.shrink(ptr, old_layout, new_layout) }
    }
}
