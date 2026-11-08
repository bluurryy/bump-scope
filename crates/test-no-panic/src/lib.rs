#![no_std]
#![expect(clippy::missing_safety_doc)]
extern crate alloc;

use alloc::{
    alloc::{AllocError, Allocator},
    boxed::Box,
};
use core::{alloc::Layout, ffi::CStr, fmt, mem::MaybeUninit, ptr::NonNull};

use bump_scope::{
    Bump, BumpAllocation, BumpAllocator as _, BumpClaimGuard, BumpScope, BumpScopeGuard, Checkpoint, MutBumpAllocator as _,
    config::{BumpConfig, BumpConfigBuilder},
};

type Result<T = (), E = AllocError> = core::result::Result<T, E>;

// Comment the line below to test that panic detection works.
#[cfg(false)]
pub mod checking_that_panik_detection_works {
    pub fn panik() {
        panic!("oh no");
    }

    #[allow(clippy::eq_op, clippy::assertions_on_constants)]
    pub fn assert() {
        assert!(1 != 1, "oh no");
    }

    #[allow(clippy::eq_op)]
    pub fn assert_eq() {
        assert_ne!(1, 1, "oh no");
    }
}

const DEFAULT: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();

macro_rules! up_and_down {
    ($($tt:tt)*) => {
        pub mod upwards {
            #[allow(unused_imports)]
            use super::*;

            const DEFAULT: BumpConfig = BumpConfigBuilder(super::DEFAULT)
                .upwards(true)
                .build();

            $($tt)*
        }

        pub mod downwards {
            #[allow(unused_imports)]
            use super::*;

            const DEFAULT: BumpConfig = BumpConfigBuilder(super::DEFAULT)
                .upwards(false)
                .build();

            $($tt)*
        }
    };
}

up_and_down! {
    const MIN_ALIGN_4: BumpConfig = BumpConfigBuilder(DEFAULT).min_align(4).build();
    const NO_DUMMIES: BumpConfig = BumpConfigBuilder(DEFAULT).const_constructible(false).claimable(false).build();
    const CONST_CONSTRUCTIBLE: BumpConfig = BumpConfigBuilder(NO_DUMMIES).const_constructible(true).build();
    const CLAIMABLE: BumpConfig = BumpConfigBuilder(NO_DUMMIES).claimable(true).build();

    // CONSTRUCTORS

    #[inline(never)]
    pub fn bump_const_new() -> Bump<DEFAULT> {
        Bump::const_new()
    }

    #[inline(never)]
    pub fn bump_default() -> Bump<DEFAULT> {
        Bump::default()
    }

    #[inline(never)]
    pub fn bump_try_new() -> Result<Bump<DEFAULT>> {
        Bump::try_new()
    }

    #[inline(never)]
    pub fn bump_try_with_size(size: usize) -> Result<Bump<DEFAULT>> {
        Bump::try_with_size(size)
    }

    #[inline(never)]
    pub fn bump_try_capacity(layout: Layout) -> Result<Bump<DEFAULT>> {
        Bump::try_with_capacity(layout)
    }

    // ALLOCATOR

    #[inline(never)]
    pub fn bump_allocate(bump: &Bump<DEFAULT>, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        bump.allocate(layout)
    }

    #[inline(never)]
    pub fn bump_allocate_zeroed(bump: &Bump<DEFAULT>, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        bump.allocate_zeroed(layout)
    }

    #[inline(never)]
    pub unsafe fn bump_deallocate(bump: &Bump<DEFAULT>, ptr: NonNull<u8>, layout: Layout) {
        unsafe { bump.deallocate(ptr, layout) }
    }

    #[inline(never)]
    pub unsafe fn bump_grow(bump: &Bump<DEFAULT>, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { bump.grow(ptr, old_layout, new_layout) }
    }

    #[inline(never)]
    pub unsafe fn bump_grow_zeroed(bump: &Bump<DEFAULT>, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { bump.grow_zeroed(ptr, old_layout, new_layout) }
    }

    #[inline(never)]
    pub unsafe fn bump_shrink(bump: &Bump<DEFAULT>, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { bump.shrink(ptr, old_layout, new_layout) }
    }

    // CHANGE CONFIG

    #[inline(never)]
    pub fn bump_change_config_identity(bump: Bump<DEFAULT>) -> Bump<DEFAULT> {
        bump.change_config()
    }

    #[inline(never)]
    pub fn bump_change_config_increase_align(bump: Bump<DEFAULT>) -> Bump<MIN_ALIGN_4> {
        bump.change_config()
    }

    #[inline(never)]
    pub fn bump_change_config_decrease_align(bump: Bump<MIN_ALIGN_4>) -> Bump<DEFAULT> {
        bump.change_config()
    }

    #[inline(never)]
    pub fn bump_change_config_enable_dummies(bump: Bump<NO_DUMMIES>) -> Bump<DEFAULT> {
        bump.change_config()
    }

    // REF CHANGE CONFIG

    #[inline(never)]
    pub fn bump_ref_change_config_identity(bump: &Bump<DEFAULT>) -> &Bump<DEFAULT> {
        bump.ref_change_config()
    }

    #[inline(never)]
    pub fn bump_ref_change_config_enable_const_constructible(bump: &Bump<NO_DUMMIES>) -> &Bump<CONST_CONSTRUCTIBLE> {
        bump.ref_change_config()
    }

    // MUT CHANGE CONFIG

    #[inline(never)]
    pub fn bump_mut_change_config_identity(bump: &mut Bump<DEFAULT>) -> &mut Bump<DEFAULT> {
        bump.mut_change_config()
    }

    #[inline(never)]
    pub fn bump_mut_change_config_increase_align(bump: &mut Bump<DEFAULT>) -> &mut Bump<MIN_ALIGN_4> {
        bump.mut_change_config()
    }

    // SCOPE CHANGE CONFIG

    #[inline(never)]
    pub fn bump_scope_change_config_identity(bump: BumpScope<DEFAULT>) -> BumpScope<DEFAULT> {
        bump.change_config()
    }

    #[inline(never)]
    pub fn bump_scope_change_config_increase_align(bump: BumpScope<DEFAULT>) -> BumpScope<MIN_ALIGN_4> {
        bump.change_config()
    }

    #[inline(never)]
    pub fn bump_scope_change_config_enable_claimable(bump: BumpScope<NO_DUMMIES>) -> BumpScope<CLAIMABLE> {
        bump.change_config()
    }

    #[inline(never)]
    pub fn bump_scope_change_config_disable_const_constructible(bump: BumpScope<CONST_CONSTRUCTIBLE>) -> BumpScope<NO_DUMMIES> {
        bump.change_config()
    }

    #[inline(never)]
    pub fn bump_scope_change_config_enable_const_constructible(bump: BumpScope<NO_DUMMIES>) -> BumpScope<CONST_CONSTRUCTIBLE> {
        bump.change_config()
    }

    // SCOPE REF CHANGE CONFIG

    #[inline(never)]
    pub fn bump_scope_ref_change_config_enable_const_constructible<'b, 'a>(bump: &'b BumpScope<'a, NO_DUMMIES>) -> &'b BumpScope<'a, CONST_CONSTRUCTIBLE> {
        bump.ref_change_config()
    }

    // SCOPE MUT CHANGE CONFIG

    #[inline(never)]
    pub fn bump_scope_mut_change_config_increase_align<'b, 'a>(bump: &'b mut BumpScope<'a, DEFAULT>) -> &'b BumpScope<'a, MIN_ALIGN_4> {
        bump.mut_change_config()
    }

    // DROP

    #[inline(never)]
    pub fn bump_drop(bump: Bump<DEFAULT>) {
        drop(bump)
    }

    // OTHER API

    #[inline(never)]
    pub fn bump_reset(bump: &mut Bump<DEFAULT>) {
        bump.reset()
    }

    #[inline(never)]
    pub fn bump_claim(bump: &mut Bump<CLAIMABLE>) -> BumpClaimGuard<'_, '_, CLAIMABLE> {
        bump.claim()
    }

    #[inline(never)]
    pub fn bump_scope_guard(bump: &mut Bump<DEFAULT>) -> BumpScopeGuard<'_, DEFAULT> {
        bump.scope_guard()
    }

    #[inline(never)]
    pub fn bump_scoped(bump: &mut Bump<DEFAULT>, f: Box<dyn FnOnce(&mut BumpScope<DEFAULT>)>) {
        bump.scoped(f)
    }

    #[inline(never)]
    pub fn bump_scoped_with_config_increase_align(bump: &mut Bump<DEFAULT>, f: Box<dyn FnOnce(&mut BumpScope<MIN_ALIGN_4>)>) {
        bump.scoped_with_config(f)
    }

    #[inline(never)]
    pub fn bump_scoped_with_config_decrease_align(bump: &mut Bump<MIN_ALIGN_4>, f: Box<dyn FnOnce(&mut BumpScope<DEFAULT>)>) {
        bump.scoped_with_config(f)
    }

    // BUMP ALLOCATOR

    #[inline(never)]
    pub fn bump_try_alloc(bump: &Bump<DEFAULT>, value: u32) -> Result<BumpAllocation<u32, &Bump<DEFAULT>>, AllocError> {
        bump.try_alloc(value)
    }

    #[inline(never)]
    pub fn bump_try_alloc_default(bump: &Bump<DEFAULT>) -> Result<BumpAllocation<u32, &Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_default()
    }

    #[inline(never)]
    pub fn bump_try_alloc_clone<'a>(bump: &'a Bump<DEFAULT>, str: &str) -> Result<BumpAllocation<str, &'a Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_clone(str)
    }

    #[inline(never)]
    pub fn bump_try_alloc_uninit(bump: &Bump<DEFAULT>) -> Result<BumpAllocation<MaybeUninit<u32>, &Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_uninit()
    }

    #[inline(never)]
    pub fn bump_try_alloc_slice_move(bump: &Bump<DEFAULT>, slice: Box<[u32]>) -> Result<BumpAllocation<[u32], &Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_slice_move(slice)
    }

    #[inline(never)]
    pub fn bump_try_alloc_slice_copy<'a>(bump: &'a Bump<DEFAULT>, slice: &[u32]) -> Result<BumpAllocation<[u32], &'a Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_slice_copy(slice)
    }

    #[inline(never)]
    pub fn bump_try_alloc_slice_clone<'a>(bump: &'a Bump<DEFAULT>, slice: &[u32]) -> Result<BumpAllocation<[u32], &'a Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_slice_clone(slice)
    }

    #[inline(never)]
    pub fn bump_try_alloc_slice_fill(bump: &Bump<DEFAULT>, len: usize, value: u32) -> Result<BumpAllocation<[u32], &Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_slice_fill(len, value)
    }

    #[inline(never)]
    pub fn bump_try_alloc_slice_fill_with<'a>(bump: &'a Bump<DEFAULT>, len: usize, f: &mut dyn FnMut() -> u32) -> Result<BumpAllocation<[u32], &'a Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_slice_fill_with(len, f)
    }

    #[inline(never)]
    pub fn bump_try_alloc_uninit_slice(bump: &Bump<DEFAULT>, len: usize) -> Result<BumpAllocation<[MaybeUninit<u32>], &Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_uninit_slice(len)
    }

    #[inline(never)]
    pub fn bump_try_alloc_uninit_slice_for<'a>(bump: &'a Bump<DEFAULT>, slice: &[u32]) -> Result<BumpAllocation<[MaybeUninit<u32>], &'a Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_uninit_slice_for(slice)
    }

    #[inline(never)]
    pub fn bump_try_alloc_str<'a>(bump: &'a Bump<DEFAULT>, str: &str) -> Result<BumpAllocation<str, &'a Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_str(str)
    }

    #[inline(never)]
    pub fn bump_try_alloc_fmt<'a>(bump: &'a Bump<DEFAULT>, args: fmt::Arguments) -> Result<BumpAllocation<str, &'a Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_fmt(args)
    }

    #[inline(never)]
    pub fn bump_try_alloc_cstr<'a>(bump: &'a Bump<DEFAULT>, str: &CStr) -> Result<&'a CStr, AllocError> {
        Ok(bump.try_alloc_cstr(str)?.into_ref())
    }

    #[inline(never)]
    pub fn bump_try_alloc_cstr_from_str<'a>(bump: &'a Bump<DEFAULT>, str: &str) -> Result<&'a CStr, AllocError> {
        Ok(bump.try_alloc_cstr_from_str(str)?.into_ref())
    }

    #[inline(never)]
    pub fn bump_try_alloc_cstr_fmt<'a>(bump: &'a Bump<DEFAULT>, args: fmt::Arguments) -> Result<&'a CStr, AllocError> {
        Ok(bump.try_alloc_cstr_fmt(args)?.into_ref())
    }

    #[inline(never)]
    pub fn bump_try_alloc_iter<'a>(bump: &'a Bump<DEFAULT>, iter: &mut dyn Iterator<Item = u32>) -> Result<BumpAllocation<[u32], &'a Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_iter(iter)
    }

    #[inline(never)]
    pub fn bump_try_alloc_iter_exact<'a>(bump: &'a Bump<DEFAULT>, iter: &mut dyn ExactSizeIterator<Item = u32>) -> Result<BumpAllocation<[u32], &'a Bump<DEFAULT>>, AllocError> {
        bump.try_alloc_iter_exact(iter)
    }

    #[inline(never)]
    pub fn bump_try_alloc_try_with<'a>(bump: &'a Bump<DEFAULT>, f: &dyn Fn() -> Result<u32, i32>) -> Result<Result<BumpAllocation<u32, &'a Bump<DEFAULT>>, i32>, AllocError> {
        bump.try_alloc_try_with(f)
    }

    #[inline(never)]
    pub fn bump_checkpoint(bump: &Bump<DEFAULT>) -> Checkpoint {
        bump.checkpoint()
    }

    #[inline(never)]
    pub unsafe fn bump_reset_to(bump: &Bump<DEFAULT>, checkpoint: Checkpoint) {
        unsafe { bump.reset_to(checkpoint) }
    }

    #[inline(never)]
    pub fn bump_is_claimed(bump: &Bump<DEFAULT>) -> bool {
        bump.is_claimed()
    }

    #[inline(never)]
    pub fn bump_try_reserve(bump: &Bump<DEFAULT>, additional: usize) -> Result<(), AllocError> {
        bump.try_reserve(additional)
    }

    // MUT BUMP ALLOCATOR

    #[inline(never)]
    pub fn bump_try_mut_alloc_iter<'a>(bump: &'a mut Bump<DEFAULT>, iter: &mut dyn Iterator<Item = u32>) -> Result<BumpAllocation<[u32], &'a Bump<DEFAULT>>, AllocError> {
        bump.try_mut_alloc_iter(iter)
    }

    #[inline(never)]
    pub fn bump_try_mut_alloc_fmt<'a>(bump: &'a mut Bump<DEFAULT>, args: fmt::Arguments) -> Result<BumpAllocation<str, &'a Bump<DEFAULT>>, AllocError> {
        bump.try_mut_alloc_fmt(args)
    }

    #[inline(never)]
    pub fn bump_try_mut_alloc_cstr_fmt<'a>(bump: &'a mut Bump<DEFAULT>, args: fmt::Arguments) -> Result<&'a CStr, AllocError> {
        Ok(bump.try_mut_alloc_cstr_fmt(args)?.into_ref())
    }

    #[inline(never)]
    pub fn bump_try_mut_alloc_try_with<'a>(bump: &'a mut Bump<DEFAULT>, f: &dyn Fn() -> Result<u32, i32>) -> Result<Result<BumpAllocation<u32, &'a Bump<DEFAULT>>, i32>, AllocError> {
        bump.try_mut_alloc_try_with(f)
    }

    // ZEROCOPY

    #[inline(never)]
    pub fn zerocopy_08_bump_try_alloc_zeroed(bump: &Bump<DEFAULT>) -> Result<BumpAllocation<u32, &Bump<DEFAULT>>, AllocError> {
        use bump_scope::zerocopy_08::BumpAllocatorExt;
        bump.try_alloc_zeroed()
    }

    #[inline(never)]
    pub fn zerocopy_08_bump_try_alloc_zeroed_slice(bump: &Bump<DEFAULT>, len: usize) -> Result<BumpAllocation<[u32], &Bump<DEFAULT>>, AllocError> {
        use bump_scope::zerocopy_08::BumpAllocatorExt;
        bump.try_alloc_zeroed_slice(len)
    }

    // BYTEMUCK

    #[inline(never)]
    pub fn bytemuck_bump_try_alloc_zeroed(bump: &Bump<DEFAULT>) -> Result<BumpAllocation<u32, &Bump<DEFAULT>>, AllocError> {
        use bump_scope::bytemuck::BumpAllocatorExt;
        bump.try_alloc_zeroed()
    }

    #[inline(never)]
    pub fn bytemuck_bump_try_alloc_zeroed_slice(bump: &Bump<DEFAULT>, len: usize) -> Result<BumpAllocation<[u32], &Bump<DEFAULT>>, AllocError> {
        use bump_scope::bytemuck::BumpAllocatorExt;
        bump.try_alloc_zeroed_slice(len)
    }
}
