use core::{
    alloc::{AllocError, Allocator, Layout},
    cell::Cell,
    fmt::{self, Debug},
    panic::{RefUnwindSafe, UnwindSafe},
    ptr::{self, NonNull},
};

use alloc::alloc::Global;

use crate::{
    AsBumpScope as _, AsMutBumpScope as _, BumpClaimGuard, BumpScope, BumpScopeGuard,
    bump_imp::{AsBumpImp as _, BumpImp},
    chunk_size::{ChunkSize, ChunkSizeCapacity, ChunkSizeDefault, ChunkSizeHint},
    chunk_types::{AllocatedChunk, Chunk},
    config::{BumpConfig, DEFAULT_BUMP_CONFIG, config},
    destructure::destructure,
    error_mode::ErrorMode,
    global_or_system::GlobalOrSystem,
    stats::Stats,
    util::{transmute_mut, transmute_ref, transmute_value},
};

#[cfg(feature = "panic-on-alloc")]
use crate::error_mode::panic_on_error;

/// The bump allocator.
///
/// # Generic parameters
/// - **`C`** — the bump allocator config, see [`config`](crate::config)
/// - **`A`** — the base allocator, defaults to `Global` when the `alloc` feature is enabled
///
/// # Create a `Bump` ...
/// - with the [`default_initial_chunk_size`]: <code>[new]\([_in][new_in])</code>
/// - with a size: <code>[with_size]\([_in][with_size_in])</code>
/// - with a capacity: <code>[with_capacity]\([_in][with_capacity_in])</code>
/// - without allocating a chunk: <code>[const_new]\([_in][const_new_in])</code>
/// - with `default`, which acts like `const_new` if [`const_constructible`] or `new` if not
///
/// # Allocate ...
/// using the `alloc` methods of [`BumpAllocator`], which `Bump` implements.
///
/// # Free memory using ...
/// - scopes: [`scoped`], [`scope_guard`], [`scoped_with_config`]
/// - checkpoints: [`checkpoint`], [`reset_to`]
/// - reset: [`reset`], [`reset_to_start`]
///
/// # Change configuration values...
/// - [`with_config`], [`change_config`], [`ref_change_config`], [`mut_change_config`]
///
/// # Gotcha
///
/// Having live allocations and entering bump scopes at the same time requires a `BumpScope`.
/// This is due to the way lifetimes work, since `Bump` returns allocations with the lifetime
/// of its own borrow instead of a separate lifetime like `BumpScope` does.
///
/// So you can't do this:
/// ```compile_fail,E0502
/// use bump_scope::{Bump, BumpAllocator};
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
/// But you can make the code work by converting the `Bump` it to a [`BumpScope`] first using [`as_mut_bump_scope`]:
/// ```
/// use bump_scope::{Bump, BumpAllocator, AsMutBumpScope};
/// let mut bump: Bump = Bump::new();
/// let bump = bump.as_mut_bump_scope();
///
/// let one = bump.alloc(1).into_ref();
///
/// bump.scoped(|bump| {
///     // whatever
///     # _ = bump;
/// });
/// # _ = one;
/// ```
///
/// [`as_mut_bump_scope`]: Bump::as_mut_bump_scope
/// [`default_initial_chunk_size`]: crate::config::BumpConfigBuilder::default_initial_chunk_size
/// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
/// [`BumpAllocator`]: crate::BumpAllocator
///
/// [new]: Bump::new
/// [new_in]: Bump::new_in
/// [default]: Bump::default
/// [with_size]: Bump::with_size
/// [with_size_in]: Bump::with_size_in
/// [with_capacity]: Bump::with_capacity
/// [with_capacity_in]: Bump::with_capacity_in
/// [const_new]: Bump::const_new
/// [const_new_in]: Bump::const_new_in
///
/// [`scoped`]: crate::BumpScope::scoped
/// [`scoped_with_config`]: crate::BumpScope::scoped_with_config
/// [`scope_guard`]: crate::BumpScope::scope_guard
///
/// [`checkpoint`]: crate::traits::BumpAllocator::checkpoint
/// [`reset_to`]: crate::traits::BumpAllocator::reset_to
///
/// [`reset`]: Bump::reset
/// [`reset_to_start`]: Bump::reset_to_start
///
/// [`with_config`]: Bump::with_config
/// [`change_config`]: Bump::change_config
/// [`ref_change_config`]: Bump::ref_change_config
/// [`mut_change_config`]: Bump::ref_change_config
///
#[repr(C)]
pub struct Bump<const C: BumpConfig = DEFAULT_BUMP_CONFIG, A: Allocator = Global> {
    /// Either a chunk allocated from `A`, or a [`CLAIMED`] or [`CONST_CONSTRUCTED`] dummy chunk.
    ///
    /// [`CLAIMED`]: crate::chunk_types::DummyChunk::CLAIMED
    /// [`CONST_CONSTRUCTED`]: crate::chunk_types::DummyChunk::CONST_CONSTRUCTED
    pub(crate) chunk: Cell<Chunk<C>>,
    pub(crate) allocator_ptr: Cell<*const A>,
    pub(crate) allocator: A,
}

// Sending Bumps when nothing is allocated is fine.
// When something is allocated Bump is borrowed and sending is not possible.
unsafe impl<const C: BumpConfig, A> Send for Bump<C, A> where A: Send + Allocator {}

impl<const C: BumpConfig, A: Allocator + UnwindSafe + RefUnwindSafe> UnwindSafe for Bump<C, A> {}
impl<const C: BumpConfig, A: Allocator + RefUnwindSafe> RefUnwindSafe for Bump<C, A> {}

impl<const C: BumpConfig, A: Allocator> Drop for Bump<C, A> {
    fn drop(&mut self) {
        let Some(mut chunk) = self.chunk.get().as_allocated() else {
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

impl<const C: BumpConfig, A: Allocator> Debug for Bump<C, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.imp().stats().debug_format("Bump", f)
    }
}

impl<const C: BumpConfig, A: Allocator + Default> Default for Bump<C, A> {
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
            Self {
                chunk: Cell::new(Chunk::CONST_CONSTRUCTED),
                allocator_ptr: Cell::new(ptr::null()),
                allocator: A::default(),
            }
        } else {
            cfg_select! {
                feature = "panic-on-alloc" => Self::new(),
                _ => panic!(
                    "can't default construct a `Bump` without `const_constructible` and no \"panic-on-alloc\" feature"
                ),
            }
        }
    }
}

#[expect(private_bounds)]
impl<const C: BumpConfig, A: Allocator + GlobalOrSystem> Bump<C, A> {
    /// Constructs a new `Bump` without allocating a chunk.
    ///
    /// # Compile-Time Assertions
    /// Fails to compile when not [`const_constructible`].
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, config::DEFAULT_BUMP_CONFIG};
    ///
    /// let bump: Bump<DEFAULT_BUMP_CONFIG> = Bump::const_new();
    /// # _ = bump;
    /// ```
    ///
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    #[must_use]
    pub const fn const_new() -> Self {
        const { assert!(config!(const_constructible)) };
        Self {
            chunk: Cell::new(Chunk::CONST_CONSTRUCTED),
            allocator_ptr: Cell::new(ptr::null()),
            allocator: A::CONST_DEFAULT,
        }
    }
}

impl<const C: BumpConfig, A: Allocator + Default> Bump<C, A> {
    /// Constructs a new `Bump` without allocating a chunk.
    ///
    /// # Compile-Time Assertions
    /// Fails to compile when not [`const_constructible`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::alloc::System as MyAllocator;
    /// use bump_scope::{Bump, config::DEFAULT_BUMP_CONFIG};
    ///
    /// let bump: Bump<DEFAULT_BUMP_CONFIG, MyAllocator> = Bump::const_new_in(MyAllocator);
    /// # _ = bump;
    /// ```
    ///
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    #[must_use]
    pub const fn const_new_in(allocator: A) -> Self {
        const { assert!(config!(const_constructible)) };
        Self {
            chunk: Cell::new(Chunk::CONST_CONSTRUCTED),
            allocator_ptr: Cell::new(ptr::null()),
            allocator,
        }
    }
}

/// Methods for a `Bump` with a default base allocator.
impl<const C: BumpConfig, A: Allocator + Default> Bump<C, A> {
    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[with_size][]([default_initial_chunk_size])</code>.
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
    /// [default_initial_chunk_size]: crate::config::BumpConfigBuilder::default_initial_chunk_size
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn new() -> Self {
        Self::new_in(A::default())
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[try_with_size][]([default_initial_chunk_size])</code>.
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
    /// [default_initial_chunk_size]: crate::config::BumpConfigBuilder::default_initial_chunk_size
    #[inline(always)]
    pub fn try_new() -> Result<Self, AllocError> {
        Self::try_new_in(A::default())
    }

    /// Constructs a new `Bump` with a size for the first chunk.
    ///
    /// If you want to ensure a specific capacity, use [`with_capacity`](Self::with_capacity) instead.
    ///
    /// The layout size that is finally requested from the base allocator may be larger due to alignment and because
    /// we currently round it up to either the next power of two or the next multiple of `0x1000`, whichever is smaller.
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
    /// assert_eq!(bump_1mib.stats().size(), 1024 * 1024);
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
    /// use bump_scope::Bump;
    ///
    /// // `Bump` with a 1 Mebibyte sized chunk
    /// let bump_1mib: Bump = Bump::try_with_size(1024 * 1024)?;
    /// assert_eq!(bump_1mib.stats().size(), 1024 * 1024);
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
impl<const C: BumpConfig, A: Allocator> Bump<C, A> {
    #[inline(always)]
    pub(crate) fn new_allocated<E: ErrorMode>(size: impl ChunkSize, allocator: A) -> Result<Self, E> {
        Ok(Self {
            chunk: Cell::new(Chunk::from_allocated(AllocatedChunk::new_first::<E, A>(size, &allocator)?)),
            allocator_ptr: Cell::new(ptr::null()),
            allocator,
        })
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[with_size_in][]([default_initial_chunk_size], allocator)</code>.
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
    /// [default_initial_chunk_size]: crate::config::BumpConfigBuilder::default_initial_chunk_size
    #[must_use]
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    pub fn new_in(allocator: A) -> Self {
        panic_on_error(Self::new_allocated(ChunkSizeDefault, allocator))
    }

    /// Constructs a new `Bump` with a default size hint for the first chunk.
    ///
    /// This is equivalent to <code>[try_with_size_in][]([default_initial_chunk_size], allocator)</code>.
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
    /// [default_initial_chunk_size]: crate::config::BumpConfigBuilder::default_initial_chunk_size
    #[inline(always)]
    pub fn try_new_in(allocator: A) -> Result<Self, AllocError> {
        Self::new_allocated(ChunkSizeDefault, allocator)
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
        panic_on_error(Self::new_allocated(ChunkSizeHint(size), allocator))
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
        Self::new_allocated(ChunkSizeHint(size), allocator)
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
        panic_on_error(Self::new_allocated(ChunkSizeCapacity(layout), allocator))
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
        Self::new_allocated(ChunkSizeCapacity(layout), allocator)
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
    #[inline(always)]
    pub fn reset(&mut self) {
        let Some(mut chunk) = self.chunk.get().as_allocated() else {
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
        self.chunk.set(Chunk::from_allocated(chunk));
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
    #[inline(always)]
    pub fn reset_to_start(&mut self) {
        self.imp().reset_to_start();
    }

    /// Returns a type which provides statistics about the memory usage of the bump allocator.
    #[must_use]
    #[inline(always)]
    pub fn stats(&self) -> Stats<'_, C, A> {
        self.imp().stats()
    }

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
    /// [`self.is_claimed()`]: crate::traits::BumpAllocator::is_claimed
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    #[inline]
    pub fn change_config<const NEW_C: BumpConfig>(self) -> Bump<NEW_C, A> {
        BumpImp::<C, A, false>::const_assert_upwards_matches::<NEW_C>();

        self.imp().assert_convertible_claimable::<NEW_C>();
        self.imp().assert_convertible_const_constructible::<NEW_C>();
        self.imp().align_to(config!(NEW_C, min_align));

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
        self.imp().ensure_satisfies_config_for_borrow::<NEW_C>();
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
        self.imp().ensure_satisfies_config_for_borrow_mut::<NEW_C>();
        unsafe { transmute_mut(self) }
    }

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
        destructure!(let Self { chunk, allocator } = self);
        (chunk.get().as_ptr(), allocator)
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
            chunk: Cell::new(unsafe { Chunk::from_ptr(ptr.cast()) }),
            allocator_ptr: Cell::new(ptr::null()),
            allocator,
        }
    }
}

/// Additional `alloc` methods that are not available in traits.
impl<const C: BumpConfig, A: Allocator> Bump<C, A> {
    /// Forwards to [`BumpScope::allocator`].
    #[must_use]
    #[inline(always)]
    pub fn allocator(&self) -> &A {
        self.imp().allocator()
    }

    /// Forwards to [`BumpScope::claim`].
    #[inline(always)]
    pub fn claim(&self) -> BumpClaimGuard<'_, '_, C, A> {
        self.as_bump_scope().claim()
    }

    /// Forwards to [`BumpScope::scope_guard`].
    #[inline(always)]
    pub fn scope_guard(&mut self) -> BumpScopeGuard<'_, C, A> {
        self.as_mut_bump_scope().scope_guard()
    }

    /// Forwards to [`BumpScope::scoped`].
    #[inline(always)]
    pub fn scoped<R>(&mut self, f: impl FnOnce(&mut BumpScope<C, A>) -> R) -> R {
        self.as_mut_bump_scope().scoped(f)
    }

    /// Forwards to [`BumpScope::scoped_with_config`].
    #[inline(always)]
    pub fn scoped_with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<NEW_C, A>) -> R) -> R {
        self.as_mut_bump_scope().scoped_with_config::<NEW_C, R>(f)
    }

    /// Forwards to [`BumpScope::with_config`].
    #[inline(always)]
    pub fn with_config<const NEW_C: BumpConfig, R>(&mut self, f: impl FnOnce(&mut BumpScope<'_, NEW_C, A>) -> R) -> R {
        self.as_mut_bump_scope().with_config(f)
    }
}

unsafe impl<const C: BumpConfig, A: Allocator> Allocator for Bump<C, A> {
    #[inline(always)]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.imp().allocate(layout)
    }

    #[inline(always)]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.imp().deallocate(ptr, layout) };
    }

    #[inline(always)]
    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.imp().grow(ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.imp().grow_zeroed(ptr, old_layout, new_layout) }
    }

    #[inline(always)]
    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.imp().shrink(ptr, old_layout, new_layout) }
    }
}
