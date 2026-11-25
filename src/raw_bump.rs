use core::{
    alloc::{AllocError, Allocator, Layout},
    cell::Cell,
    num::NonZeroUsize,
    ops::Range,
    ptr::{self, NonNull},
};

use bump_scope_core::config::BumpConfig;

use crate::{
    Bump, BumpScope, Checkpoint, Unmanaged,
    error_mode::ErrorMode,
    polyfill::hint::unlikely,
    util::{SizedTypeProperties, align_pos, bump_down, up_align_usize_unchecked},
};

/// Provides bump allocation functionality for both `Bump` and `BumpScope`.
#[repr(C)]
pub(crate) struct RawBump<const C: BumpConfig, A> {
    /// Either a chunk allocated from `A`, or a [`CLAIMED`] or [`CONST_CONSTRUCTED`] dummy chunk.
    ///
    /// [`CLAIMED`]: crate::chunk_types::DummyChunk::CLAIMED
    /// [`CONST_CONSTRUCTED`]: crate::chunk_types::DummyChunk::CONST_CONSTRUCTED
    pub(crate) chunk: Cell<Chunk<C>>,
    pub(crate) allocator: BaseAllocatorStorage<A>,
}

impl<const C: BumpConfig, A: Allocator> RawBump<C, A> {
    #[inline(always)]
    pub(crate) fn align_to(&self, align: usize) {
        assert!(align.is_power_of_two() && align <= 16);

        if align > config!(min_align) {
            // a dummy chunk is always aligned
            if let Some(chunk) = self.chunk().as_allocated() {
                let pos = chunk.pos().addr().get();
                let addr = align_pos(config!(upwards), align, pos);
                unsafe { chunk.set_pos_addr(addr) };
            }
        }
    }

    /// # Safety
    /// The caller must satisfy the the invariants of `reset_to` and
    /// `checkpoint.chunk.is_dummy()` must be `true`.
    #[cold]
    #[inline(never)]
    unsafe fn reset_to_with_dummy_checkpoint(&self, checkpoint: Checkpoint) {
        debug_assert!(checkpoint.chunk.is_dummy());

        if unlikely(config!(claimable) && checkpoint.chunk.is_claimed()) {
            // We do nothing if the checkpoint is claimed.
            // This state is reachable in safe code when a `BumpClaimGuard` is forgotten.
            return;
        }

        // Now the chunk must be const constructed, because it is the only dummy chunk
        // variant other than "claimed".
        debug_assert!(checkpoint.chunk.is_const_constructed());

        // If the checkpoint was created when the bump allocator had no allocated chunk
        // then the chunk pointer will point to the const constructed chunk.
        //
        // In such cases we reset the bump pointer to the very start of the very first chunk.
        self.reset_to_start();
    }

    pub(crate) const fn const_assert_upwards_matches<const NEW_C: BumpConfig>() {
        const {
            let old = config!(upwards);
            let new = config!(NEW_C, upwards);
            assert!(old == new, "can't change bump direction");
        }
    }

    fn can_convert_claimable<const NEW_C: BumpConfig>(&self) -> bool {
        // either nothing changed, the new config is claimable, the old one isn't, or the bump is currently not claimed
        (const {
            let old = config!(claimable);
            let new = config!(NEW_C, claimable);
            old == new || new || !old
        }) || !self.is_claimed()
    }

    pub(crate) fn assert_convertible_claimable<const NEW_C: BumpConfig>(&self) {
        if !self.can_convert_claimable::<NEW_C>() {
            config::cant_change_due_to_claimable();
        }
    }

    fn can_convert_const_constructible<const NEW_C: BumpConfig>(&self) -> bool {
        // To be able to convert const_constructible, either:
        // - nothing changed
        // - the new config is const_constructible
        // - the old config isn't const_constructible
        // - the bump currently has an allocated chunk
        (const {
            let old = config!(const_constructible);
            let new = config!(NEW_C, const_constructible);
            old == new || new || !old
        }) || self.stats().current_chunk().is_some()
    }

    pub(crate) fn assert_convertible_const_constructible<const NEW_C: BumpConfig>(&self) {
        if !self.can_convert_const_constructible::<NEW_C>() {
            config::cant_change_due_to_const_constructible();
        }
    }

    pub(crate) fn ensure_satisfies_config_for_borrow<const NEW_C: BumpConfig>(&self) {
        Self::const_assert_upwards_matches::<NEW_C>();

        const {
            assert!(
                config!(NEW_C, claimable) == config!(claimable),
                "can't change `claimable` using `Bump(Scope)::ref_change_config`"
            );

            assert!(
                config!(NEW_C, min_align) == config!(min_align),
                "can't change `min_align` using `Bump(Scope)::ref_change_config`"
            );
        }

        // Can't make this bump allocator's chunk be the const constructed chunk
        // from a shared reference so this is enough.
        self.assert_convertible_const_constructible::<NEW_C>();
    }

    pub(crate) fn ensure_satisfies_config_for_borrow_mut<const NEW_C: BumpConfig>(&self) {
        Self::const_assert_upwards_matches::<NEW_C>();

        const {
            assert!(
                config!(NEW_C, min_align) >= config!(min_align),
                "can't decrease `min_align` using `Bump(Scope)::mut_change_config`"
            );

            assert!(
                config!(NEW_C, claimable) == config!(claimable),
                "can't change `claimable` using `Bump(Scope)::mut_change_config`"
            );

            assert!(
                config!(NEW_C, const_constructible) == config!(const_constructible),
                "can't change `const_constructible` using `Bump(Scope)::mut_change_config`"
            );
        }

        self.align_to(config!(NEW_C, min_align));
    }
}
