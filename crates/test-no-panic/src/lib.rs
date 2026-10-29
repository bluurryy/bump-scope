#![no_std]
#![expect(non_snake_case, mismatched_lifetime_syntaxes, clippy::missing_safety_doc)]
extern crate alloc;

use alloc::{
    alloc::{AllocError, Allocator, Global},
    boxed::Box,
};
use core::{alloc::Layout, ffi::CStr, fmt, mem::MaybeUninit, ptr::NonNull};

use bump_scope::{
    config::{BumpConfig, BumpConfigBuilder},
    zerocopy_08::BumpAllocatorExt,
    BumpAllocator as _, MutBumpAllocator as _,
};

type Result<T = (), E = AllocError> = core::result::Result<T, E>;

macro_rules! type_definitions {
    ($upwards:literal) => {
        #[allow(unused)]
        const DEFAULT_CLAIMABLE: bool = true;
        #[allow(unused)]
        const DEFAULT_GUARANTEED_ALLOCATED: bool = true;

        #[allow(unused)]
        const DEFAULT_C: BumpConfig = BumpConfigBuilder::new().upwards($upwards).build();

        #[allow(unused)]
        type Bump<const C: BumpConfig = DEFAULT_C> = bump_scope::Bump<Global, C>;

        #[allow(unused)]
        type BumpScope<'a, const C: BumpConfig = DEFAULT_C> = bump_scope::BumpScope<'a, Global, C>;

        #[allow(unused)]
        type BumpScopeGuard<'a, const C: BumpConfig = DEFAULT_C> = bump_scope::BumpScopeGuard<'a, Global, C>;

        #[allow(unused)]
        type BumpClaimGuard<'b, 'a, const C: BumpConfig = DEFAULT_C> = bump_scope::BumpClaimGuard<'b, 'a, Global, C>;

        #[allow(unused)]
        type BumpAllocation<'a, T, const C: BumpConfig = DEFAULT_C> = bump_scope::BumpAllocation<T, &'a Bump<C>>;
    };
}

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

macro_rules! up_and_down {
    (
        $($tt:tt)*
    ) => {
        pub mod up {
            use super::*;
            type_definitions!(true);
            $($tt)*
        }

        pub mod down {
            use super::*;
            type_definitions!(false);
            $($tt)*
        }
    };
}

pub mod test_constructors {
    use super::*;

    up_and_down! {
        const CONST_CONSTRUCTIBLE: BumpConfig = BumpConfigBuilder(DEFAULT_C).const_constructible(true).build();

        pub fn Bump_const_new() -> Bump<CONST_CONSTRUCTIBLE> {
            Bump::const_new()
        }

        pub fn Bump_default() -> Bump<CONST_CONSTRUCTIBLE> {
            Bump::default()
        }

        pub fn Bump_try_new() -> Result<Bump> {
            Bump::try_new()
        }

        pub fn Bump_try_with_size(size: usize) -> Result<Bump> {
            Bump::try_with_size(size)
        }

        pub fn Bump_try_with_capacity(layout: Layout) -> Result<Bump> {
            Bump::try_with_capacity(layout)
        }

        pub fn Bump_try_new_in() -> Result<Bump> {
            Bump::try_new_in(Global)
        }

        pub fn Bump_try_with_size_in(size: usize) -> Result<Bump> {
            Bump::try_with_size_in(size, Global)
        }

        pub fn Bump_try_with_capacity_in(layout: Layout) -> Result<Bump> {
            Bump::try_with_capacity_in(layout, Global)
        }
    }
}

pub mod test_change_config {
    use super::*;

    up_and_down! {
        const UNCLAIMABLE: BumpConfig = BumpConfigBuilder(DEFAULT_C).claimable(false).build();
        const CONST_CONSTRUCTIBLE: BumpConfig = BumpConfigBuilder(DEFAULT_C).const_constructible(true).build();
        const MIN_ALIGN_4: BumpConfig = BumpConfigBuilder(DEFAULT_C).min_align(4).build();

        pub fn Bump_change_config_identity(bump: Bump) -> Bump {
            bump.change_config()
        }

        pub fn Bump_change_config_increase_alignment(bump: Bump) -> Bump<MIN_ALIGN_4> {
            bump.change_config()
        }

        pub fn Bump_change_config_decrease_alignment(bump: Bump<MIN_ALIGN_4>) -> Bump {
            bump.change_config()
        }

        pub fn Bump_change_config_enable_claimable(bump: Bump<UNCLAIMABLE>) -> Bump {
            bump.change_config()
        }

        pub fn Bump_change_config_enable_const_constructible(bump: Bump) -> Bump<CONST_CONSTRUCTIBLE> {
            bump.change_config()
        }

        pub fn Bump_ref_change_config_enable_const_constructible(bump: &Bump) -> &Bump<CONST_CONSTRUCTIBLE> {
            bump.ref_change_config()
        }

        pub fn Bump_mut_change_config_increase_alignment(bump: &mut Bump) -> &mut Bump<MIN_ALIGN_4> {
            bump.mut_change_config()
        }

        pub fn BumpScope_change_config_identity(bump: BumpScope) -> BumpScope {
            bump.change_config()
        }

        pub fn BumpScope_change_config_increase_alignment(bump: BumpScope) -> BumpScope<MIN_ALIGN_4> {
            bump.change_config()
        }

        pub fn BumpScope_change_config_enable_claimable(bump: BumpScope<UNCLAIMABLE>) -> BumpScope {
            bump.change_config()
        }

        pub fn BumpScope_change_config_disable_const_constructible(bump: BumpScope<CONST_CONSTRUCTIBLE>) -> BumpScope {
            bump.change_config()
        }

        pub fn BumpScope_change_config_enable_const_constructible(bump: BumpScope) -> BumpScope<CONST_CONSTRUCTIBLE> {
            bump.change_config()
        }

        pub fn BumpScope_ref_change_config_enable_const_constructible<'b, 'a>(bump: &'b BumpScope<'a>) -> &'b BumpScope<'a, CONST_CONSTRUCTIBLE> {
            bump.ref_change_config()
        }

        pub fn BumpScope_mut_change_config_increase_alignment<'b, 'a>(bump: &'b mut BumpScope<'a>) -> &'b mut BumpScope<'a, MIN_ALIGN_4> {
            bump.mut_change_config()
        }
    }
}

pub mod test_other {
    use super::*;

    up_and_down! {
        pub fn Bump_drop(bump: Bump) {
            drop(bump)
        }

        pub fn Bump_reset(bump: &mut Bump) {
            bump.reset()
        }
    }
}

pub mod test_forwarded_methods {
    use super::*;

    up_and_down! {
        const MIN_ALIGN_4: BumpConfig = BumpConfigBuilder(DEFAULT_C).min_align(4).build();

        pub fn Bump_claim(bump: &Bump) -> BumpClaimGuard<'_, '_> {
            bump.claim()
        }

        pub fn BumpScope_claim<'b, 'a>(bump: &'b BumpScope<'a>) -> BumpClaimGuard<'b, 'a> {
            bump.claim()
        }

        pub fn Bump_is_claimed(bump: &Bump) -> bool {
            bump.is_claimed()
        }

        pub fn Bump_scope_guard(bump: &mut Bump) -> BumpScopeGuard {
            bump.scope_guard()
        }

        pub fn Bump_scoped(bump: &mut Bump, f: Box<dyn FnOnce(&mut BumpScope)>) {
            bump.scoped(f)
        }

        pub fn Bump_scoped_with_config(bump: &mut Bump, f: Box<dyn FnOnce(&mut BumpScope<MIN_ALIGN_4>)>) {
            bump.scoped_with_config(f)
        }

        pub fn Bump_with_config_min_align_inc(bump: &mut Bump, f: Box<dyn FnOnce(&mut BumpScope<MIN_ALIGN_4>)>) {
            bump.with_config(f)
        }

        pub fn Bump_with_config_min_align_dec(bump: &mut Bump<MIN_ALIGN_4>, f: Box<dyn FnOnce(&mut BumpScope)>) {
            bump.with_config(f)
        }

        pub fn Bump_try_alloc(bump: &Bump, value: u32) -> Result<BumpAllocation<u32>> {
            bump.try_alloc(value)
        }

        pub fn Bump_try_alloc_default(bump: &Bump) -> Result<BumpAllocation<u32>> {
            bump.try_alloc_default()
        }

        #[cfg(feature = "nightly")]
        pub fn Bump_try_alloc_clone<'a>(bump: &'a Bump, value: &str) -> Result<BumpAllocation<'a, str>> {
            bump.try_alloc_clone(value)
        }

        pub fn Bump_try_alloc_slice_move<'a>(bump: &'a Bump, slice: Box<[u32]>) -> Result<BumpAllocation<'a, [u32]>> {
            bump.try_alloc_slice_move(slice)
        }

        pub fn Bump_try_alloc_slice_copy<'a>(bump: &'a Bump, value: &[u32]) -> Result<BumpAllocation<'a, [u32]>> {
            bump.try_alloc_slice_copy(value)
        }

        pub fn Bump_try_alloc_slice_clone<'a>(bump: &'a Bump, value: &[u32]) -> Result<BumpAllocation<'a, [u32]>> {
            bump.try_alloc_slice_clone(value)
        }

        pub fn Bump_try_alloc_slice_fill(bump: &Bump, len: usize, value: u32) -> Result<BumpAllocation<[u32]>> {
            bump.try_alloc_slice_fill(len, value)
        }

        pub fn Bump_try_alloc_slice_fill_with<'a>(bump: &'a Bump, len: usize, f: &mut dyn FnMut() -> u32) -> Result<BumpAllocation<'a, [u32]>> {
            bump.try_alloc_slice_fill_with(len, f)
        }

        pub fn Bump_try_alloc_str<'a>(bump: &'a Bump, value: &str) -> Result<BumpAllocation<'a, str>> {
            bump.try_alloc_str(value)
        }

        pub fn Bump_try_alloc_fmt<'a>(bump: &'a Bump, args: fmt::Arguments) -> Result<BumpAllocation<'a, str>> {
            bump.try_alloc_fmt(args)
        }

        pub fn Bump_try_alloc_fmt_mut<'a>(bump: &'a mut Bump, args: fmt::Arguments) -> Result<BumpAllocation<'a, str>> {
            bump.try_alloc_fmt_mut(args)
        }

        pub fn Bump_try_alloc_cstr<'a>(bump: &'a Bump, value: &CStr) -> Result<&'a CStr> {
            Ok(bump.try_alloc_cstr(value)?.into_ref())
        }

        pub fn Bump_try_alloc_cstr_from_str<'a>(bump: &'a Bump, value: &str) -> Result<&'a CStr> {
            Ok(bump.try_alloc_cstr_from_str(value)?.into_ref())
        }

        pub fn Bump_try_alloc_cstr_fmt<'a>(bump: &'a Bump, value: fmt::Arguments) -> Result<&'a CStr> {
            Ok(bump.try_alloc_cstr_fmt(value)?.into_ref())
        }

        pub fn Bump_try_alloc_cstr_fmt_mut<'a>(bump: &'a mut Bump, value: fmt::Arguments) -> Result<&'a CStr> {
            Ok(bump.try_alloc_cstr_fmt_mut(value)?.into_ref())
        }

        // pub fn Bump_try_alloc_iter(bump: &Bump, value: core::ops::Range<u32>) -> Result<BumpAllocation<[u32]>> {
        //     bump.try_alloc_iter(value)
        // }

        // pub fn Bump_try_alloc_iter_mut(bump: &mut Bump, value: core::ops::Range<u32>) -> Result<BumpAllocation<[u32]>> {
        //     bump.try_alloc_iter_mut(value)
        // }

        pub fn Bump_try_alloc_uninit(bump: &Bump) -> Result<BumpAllocation<MaybeUninit<u32>>> {
            bump.try_alloc_uninit()
        }

        pub fn Bump_try_alloc_uninit_slice(bump: &Bump, len: usize) -> Result<BumpAllocation<[MaybeUninit<u32>]>> {
            bump.try_alloc_uninit_slice(len)
        }

        pub fn Bump_try_alloc_uninit_slice_for<'a>(bump: &'a Bump, slice: &[u32]) -> Result<BumpAllocation<'a, [MaybeUninit<u32>]>> {
            bump.try_alloc_uninit_slice_for(slice)
        }

        pub fn Bump_try_reserve(bump: &Bump, additional: usize) -> Result {
            bump.try_reserve(additional)
        }
    }
}

pub mod test_allocator_trait {
    use super::*;

    up_and_down! {
        pub fn Bump_allocate(bump: &Bump, layout: Layout) -> Result<NonNull<[u8]>> {
            bump.allocate(layout)
        }

        pub unsafe fn Bump_deallocate(bump: &Bump, ptr: NonNull<u8>, layout: Layout) {
            bump.deallocate(ptr, layout)
        }

        pub fn Bump_allocate_zeroed(bump: &Bump, layout: Layout) -> Result<NonNull<[u8]>> {
            bump.allocate_zeroed(layout)
        }

        pub unsafe fn Bump_grow(bump: &Bump, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>> {
            bump.grow(ptr, old_layout, new_layout)
        }

        pub unsafe fn Bump_grow_zeroed(bump: &Bump, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>> {
            bump.grow_zeroed(ptr, old_layout, new_layout)
        }

        pub unsafe fn Bump_shrink(bump: &Bump, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>> {
            bump.shrink(ptr, old_layout, new_layout)
        }
    }
}

pub mod test_extension_traits {
    use super::*;

    up_and_down! {
        pub fn Bump_try_alloc_zeroed(bump: &Bump) -> Result<BumpAllocation<u32>> {
            bump.try_alloc_zeroed()
        }

        pub fn Bump_try_alloc_zeroed_slice(bump: &Bump, len: usize) -> Result<BumpAllocation<[u32]>> {
            bump.try_alloc_zeroed_slice(len)
        }
    }
}

pub mod test_alloc_try_with {
    use super::*;

    up_and_down! {
        pub fn Bump_try_alloc_try_with(bump: &Bump, f: fn() -> Result<u32, u32>) -> Result<Result<BumpAllocation<u32>, u32>> {
            bump.try_alloc_try_with(f)
        }

        pub fn Bump_try_alloc_try_with_mut(bump: &mut Bump, f: fn() -> Result<u32, u32>) -> Result<Result<BumpAllocation<u32>, u32>> {
            bump.try_alloc_try_with_mut(f)
        }
    }
}
