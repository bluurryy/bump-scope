#![no_std]
#![expect(non_snake_case, mismatched_lifetime_syntaxes, clippy::missing_safety_doc)]
extern crate alloc;

use core::{alloc::Layout, ffi::CStr, fmt, mem::MaybeUninit, ptr::NonNull};

use alloc::boxed::Box;

use bump_scope::{
    alloc::{AllocError, Allocator, Global},
    settings::BumpSettings,
    traits::BumpAllocatorTyped,
    zerocopy_08::{BumpExt, VecExt},
    BumpBox, FixedBumpString, FixedBumpVec,
};

type Result<T = (), E = AllocError> = core::result::Result<T, E>;

macro_rules! type_definitions {
    ($up:literal) => {
        type Bump<const MIN_ALIGN: usize = 1> = bump_scope::Bump<Global, BumpSettings<MIN_ALIGN, $up, true, true>>;
        type BumpScope<'a, const MIN_ALIGN: usize = 1> =
            bump_scope::BumpScope<'a, Global, BumpSettings<MIN_ALIGN, $up, true, true>>;
        type BumpScopeGuard<'a, const MIN_ALIGN: usize = 1> =
            bump_scope::BumpScopeGuard<'a, Global, BumpSettings<MIN_ALIGN, $up>>;
        type BumpScopeGuardRoot<'a, const MIN_ALIGN: usize = 1> =
            bump_scope::BumpScopeGuardRoot<'a, Global, BumpSettings<MIN_ALIGN, $up>>;
        type BumpVec<'a, T, const MIN_ALIGN: usize = 1> = bump_scope::BumpVec<T, &'a Bump>;
        type BumpString<'a, const MIN_ALIGN: usize = 1> = bump_scope::BumpString<&'a Bump>;
        type MutBumpVec<'a, T, const MIN_ALIGN: usize = 1> = bump_scope::MutBumpVec<T, &'a mut Bump<MIN_ALIGN>>;
        type MutBumpString<'a, const MIN_ALIGN: usize = 1> = bump_scope::MutBumpString<&'a mut Bump<MIN_ALIGN>>;
        type MutBumpVecRev<'a, T, const MIN_ALIGN: usize = 1> = bump_scope::MutBumpVecRev<T, &'a mut Bump<MIN_ALIGN>>;
    };
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

up_and_down! {
    pub fn Bump_drop(bump: Bump) {
        drop(bump)
    }

    pub fn Bump_reset(bump: &mut Bump) {
        bump.reset()
    }

    pub fn Bump_scoped(bump: &mut Bump, f: Box<dyn FnOnce(BumpScope)>) {
        bump.scoped(f)
    }

    pub fn Bump_aligned_inc(bump: &mut Bump, f: Box<dyn FnOnce(BumpScope<8>)>) {
        bump.aligned(f)
    }

    pub fn Bump_aligned_dec(bump: &mut Bump<8>, f: Box<dyn FnOnce(BumpScope)>) {
        bump.aligned(f)
    }

    pub fn Bump_scope_guard(bump: &mut Bump) -> BumpScopeGuardRoot {
        bump.scope_guard()
    }

    pub fn Bump_into_aligned(bump: Bump) -> Bump<4> {
        bump.into_aligned()
    }

    pub fn Bump_as_mut_aligned(bump: &mut Bump) -> &mut Bump<4> {
        bump.as_mut_aligned()
    }

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

    pub fn Bump_try_alloc(bump: &Bump, value: u32) -> Result<BumpBox<u32>> {
        bump.try_alloc(value)
    }

    pub fn Bump_try_alloc_default(bump: &Bump) -> Result<BumpBox<u32>> {
        bump.try_alloc_default()
    }

    pub fn Bump_try_alloc_zeroed(bump: &Bump) -> Result<BumpBox<u32>> {
        bump.try_alloc_zeroed()
    }

    pub fn Bump_try_alloc_str<'a>(bump: &'a Bump, value: &str) -> Result<BumpBox<'a, str>> {
        bump.try_alloc_str(value)
    }

    pub fn Bump_try_alloc_fmt<'a>(bump: &'a Bump, args: fmt::Arguments) -> Result<BumpBox<'a, str>> {
        bump.try_alloc_fmt(args)
    }

    pub fn Bump_try_alloc_fmt_mut<'a>(bump: &'a mut Bump, args: fmt::Arguments) -> Result<BumpBox<'a, str>> {
        bump.try_alloc_fmt_mut(args)
    }

    pub fn Bump_try_alloc_cstr<'a>(bump: &'a Bump, value: &CStr) -> Result<&'a CStr> {
        bump.try_alloc_cstr(value)
    }

    pub fn Bump_try_alloc_cstr_from_str<'a>(bump: &'a Bump, value: &str) -> Result<&'a CStr> {
        bump.try_alloc_cstr_from_str(value)
    }

    pub fn Bump_try_alloc_cstr_fmt<'a>(bump: &'a Bump, value: fmt::Arguments) -> Result<&'a CStr> {
        bump.try_alloc_cstr_fmt(value)
    }

    pub fn Bump_try_alloc_cstr_fmt_mut<'a>(bump: &'a mut Bump, value: fmt::Arguments) -> Result<&'a CStr> {
        bump.try_alloc_cstr_fmt_mut(value)
    }

    pub fn Bump_try_alloc_iter(bump: &Bump, value: core::ops::Range<u32>) -> Result<BumpBox<[u32]>> {
        bump.try_alloc_iter(value)
    }

    pub fn Bump_try_alloc_iter_exact(bump: &Bump, value: core::ops::Range<u32>) -> Result<BumpBox<[u32]>> {
        bump.try_alloc_iter_exact(value)
    }

    pub fn Bump_try_alloc_iter_mut(bump: &mut Bump, value: core::ops::Range<u32>) -> Result<BumpBox<[u32]>> {
        bump.try_alloc_iter_mut(value)
    }

    pub fn Bump_try_alloc_iter_mut_rev(bump: &mut Bump, value: core::ops::Range<u32>) -> Result<BumpBox<[u32]>> {
        bump.try_alloc_iter_mut_rev(value)
    }

    pub fn Bump_try_allocate_layout(bump: &Bump, layout: Layout) -> Result<NonNull<u8>> {
        bump.try_allocate_layout(layout)
    }

    pub fn Bump_try_alloc_slice_move<'a>(bump: &'a Bump, value: [u32; 4]) -> Result<BumpBox<'a, [u32]>> {
        bump.try_alloc_slice_move(value)
    }

    pub fn Bump_try_alloc_slice_copy<'a>(bump: &'a Bump, value: &[u32]) -> Result<BumpBox<'a, [u32]>> {
        bump.try_alloc_slice_copy(value)
    }

    pub fn Bump_try_alloc_slice_clone<'a>(bump: &'a Bump, value: &[u32]) -> Result<BumpBox<'a, [u32]>> {
        bump.try_alloc_slice_clone(value)
    }

    pub fn Bump_try_alloc_slice_fill(bump: &Bump, len: usize, value: u32) -> Result<BumpBox<[u32]>> {
        bump.try_alloc_slice_fill(len, value)
    }

    pub fn Bump_try_alloc_slice_fill_with<'a>(bump: &'a Bump, len: usize, f: &mut dyn FnMut() -> u32) -> Result<BumpBox<'a, [u32]>> {
        bump.try_alloc_slice_fill_with(len, f)
    }

    pub fn Bump_try_alloc_zeroed_slice(bump: &Bump, len: usize) -> Result<BumpBox<[u32]>> {
        bump.try_alloc_zeroed_slice(len)
    }

    pub fn Bump_try_alloc_uninit(bump: &Bump) -> Result<BumpBox<MaybeUninit<u32>>> {
        bump.try_alloc_uninit()
    }

    pub fn Bump_try_alloc_uninit_slice(bump: &Bump, len: usize) -> Result<BumpBox<[MaybeUninit<u32>]>> {
        bump.try_alloc_uninit_slice(len)
    }

    pub fn Bump_try_alloc_uninit_slice_for<'a>(bump: &'a Bump, slice: &[u32]) -> Result<BumpBox<'a, [MaybeUninit<u32>]>> {
        bump.try_alloc_uninit_slice_for(slice)
    }

    pub fn Bump_try_alloc_try_with(bump: &Bump, f: fn() -> Result<u32, u32>) -> Result<Result<BumpBox<u32>, u32>> {
        bump.try_alloc_try_with(f)
    }

    pub fn Bump_try_alloc_try_with_mut(bump: &mut Bump, f: fn() -> Result<u32, u32>) -> Result<Result<BumpBox<u32>, u32>> {
        bump.try_alloc_try_with_mut(f)
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

    pub fn Bump_try_reserve_bytes(bump: &Bump, additional: usize) -> Result {
        bump.try_reserve_bytes(additional)
    }

    pub fn BumpScope__scope_guard<'b>(bump: &'b mut BumpScope) -> BumpScopeGuard<'b> {
        bump.scope_guard()
    }

    pub fn MutBumpVec_try_append(vec: &mut MutBumpVec<u32>, array: [u32; 24]) -> Result {
        vec.try_append(array)
    }

    pub fn MutBumpVec_try_extend_from_slice_clone(vec: &mut MutBumpVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn MutBumpVec_try_extend_from_slice_copy(vec: &mut MutBumpVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn MutBumpVec_try_extend_zeroed(vec: &mut MutBumpVec<u32>, additional: usize) -> Result {
        vec.try_extend_zeroed(additional)
    }

    pub fn MutBumpVec_try_from_iter_in<'a>(iter: core::slice::Iter<u32>, bump: &'a mut Bump) -> Result<MutBumpVec<'a, u32>> {
        MutBumpVec::try_from_iter_in(iter.copied(), bump)
    }

    pub fn MutBumpVec__try_from_owned_slice_in(array: [u32; 24], bump: &mut Bump) -> Result<MutBumpVec<u32>> {
        MutBumpVec::try_from_owned_slice_in(array, bump)
    }

    pub fn MutBumpVec__try_from_elem_in(value: u32, count: usize, bump: &mut Bump) -> Result<MutBumpVec<u32>> {
        MutBumpVec::try_from_elem_in(value, count, bump)
    }

    pub fn MutBumpVec_try_insert(vec: &mut MutBumpVec<u32>, index: usize, value: u32) -> Result {
        if index < vec.len() {
            vec.try_insert(index, value)
        } else {
            Ok(())
        }
    }

    pub fn MutBumpVec_try_push(vec: &mut MutBumpVec<u32>, value: u32) -> Result {
        vec.try_push(value)
    }

    pub fn MutBumpVec_try_reserve(vec: &mut MutBumpVec<u32>, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn MutBumpVec_try_reserve_exact(vec: &mut MutBumpVec<u32>, amount: usize) -> Result {
        vec.try_reserve_exact(amount)
    }

    pub fn MutBumpVec_try_resize(vec: &mut MutBumpVec<u32>, new_len: usize, value: u32) -> Result {
        vec.try_resize(new_len, value)
    }

    pub fn MutBumpVec__try_with_capacity_in(capacity: usize, bump: &mut Bump) -> Result<MutBumpVec<u32>> {
        MutBumpVec::try_with_capacity_in(capacity, bump)
    }

    pub fn MutBumpVecRev_try_append(vec: &mut MutBumpVecRev<u32>, array: [u32; 24]) -> Result {
        vec.try_append(array)
    }

    pub fn MutBumpVecRev_try_extend_from_slice_clone(vec: &mut MutBumpVecRev<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn MutBumpVecRev_try_extend_from_slice_copy(vec: &mut MutBumpVecRev<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn MutBumpVecRev_try_from_iter_in<'a>(iter: core::slice::Iter<u32>, bump: &'a mut Bump) -> Result<MutBumpVecRev<'a, u32>> {
        MutBumpVecRev::try_from_iter_in(iter.copied(), bump)
    }

    pub fn MutBumpVecRev__try_from_owned_slice_in(array: [u32; 24], bump: &mut Bump) -> Result<MutBumpVecRev<u32>> {
        MutBumpVecRev::try_from_owned_slice_in(array, bump)
    }

    pub fn MutBumpVecRev__try_from_elem_in(value: u32, count: usize, bump: &mut Bump) -> Result<MutBumpVecRev<u32>> {
        MutBumpVecRev::try_from_elem_in(value, count, bump)
    }

    pub fn MutBumpVecRev_try_insert(vec: &mut MutBumpVecRev<u32>, index: usize, value: u32) -> Result {
        if index < vec.len() {
            vec.try_insert(index, value)
        } else {
            Ok(())
        }
    }

    pub fn MutBumpVecRev_try_push(vec: &mut MutBumpVecRev<u32>, value: u32) -> Result {
        vec.try_push(value)
    }

    pub fn MutBumpVecRev_try_reserve(vec: &mut MutBumpVecRev<u32>, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn MutBumpVecRev_try_reserve_exact(vec: &mut MutBumpVecRev<u32>, amount: usize) -> Result {
        vec.try_reserve_exact(amount)
    }

    pub fn MutBumpVecRev_try_resize(vec: &mut MutBumpVecRev<u32>, new_len: usize, value: u32) -> Result {
        vec.try_resize(new_len, value)
    }

    pub fn MutBumpVecRev__try_with_capacity_in(capacity: usize, bump: &mut Bump) -> Result<MutBumpVecRev<u32>> {
        MutBumpVecRev::try_with_capacity_in(capacity, bump)
    }

    pub fn MutBumpString__try_from_str_in<'b>(string: &str, bump: &'b mut Bump) -> Result<MutBumpString<'b>> {
        MutBumpString::try_from_str_in(string, bump)
    }

    pub fn MutBumpString_try_push(string: &mut MutBumpString, value: char) -> Result {
        string.try_push(value)
    }

    pub fn MutBumpString_try_push_str(string: &mut MutBumpString, value: &str) -> Result {
        string.try_push_str(value)
    }

    pub fn MutBumpString_try_reserve(string: &mut MutBumpString, amount: usize) -> Result {
        string.try_reserve(amount)
    }

    pub fn MutBumpString_try_reserve_exact(string: &mut MutBumpString, amount: usize) -> Result {
        string.try_reserve_exact(amount)
    }

    pub fn MutBumpString__try_with_capacity_in(capacity: usize, bump: &mut Bump) -> Result<MutBumpString> {
        MutBumpString::try_with_capacity_in(capacity, bump)
    }

    pub fn MutBumpString_try_into_cstr(string: MutBumpString) -> Result<&CStr> {
        string.try_into_cstr()
    }

    pub fn BumpVec_try_append(vec: &mut BumpVec<u32>, array: [u32; 24]) -> Result {
        vec.try_append(array)
    }

    pub fn BumpVec_try_extend_from_slice_clone(vec: &mut BumpVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn BumpVec_try_extend_from_slice_copy(vec: &mut BumpVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn BumpVec_try_extend_zeroed(vec: &mut BumpVec<u32>, additional: usize) -> Result {
        vec.try_extend_zeroed(additional)
    }

    pub fn BumpVec_try_from_iter_in<'a>(iter: core::slice::Iter<u32>, bump: &'a Bump) -> Result<BumpVec<'a, u32>> {
        BumpVec::try_from_iter_in(iter.copied(), bump)
    }

    pub fn BumpVec_try_from_iter_exact_in<'a>(iter: core::slice::Iter<u32>, bump: &'a Bump) -> Result<BumpVec<'a, u32>> {
        BumpVec::try_from_iter_exact_in(iter.copied(), bump)
    }

    pub fn BumpVec__try_from_owned_slice_in(array: [u32; 24], bump: &Bump) -> Result<BumpVec<u32>> {
        BumpVec::try_from_owned_slice_in(array, bump)
    }

    pub fn BumpVec__try_from_elem_in(value: u32, count: usize, bump: &Bump) -> Result<BumpVec<u32>> {
        BumpVec::try_from_elem_in(value, count, bump)
    }

    pub fn BumpVec_try_insert(vec: &mut BumpVec<u32>, index: usize, value: u32) -> Result {
        if index < vec.len() {
            vec.try_insert(index, value)
        } else {
            Ok(())
        }
    }

    pub fn BumpVec_try_push(vec: &mut BumpVec<u32>, value: u32) -> Result {
        vec.try_push(value)
    }

    pub fn BumpVec_try_reserve(vec: &mut BumpVec<u32>, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn BumpVec_try_reserve_exact(vec: &mut BumpVec<u32>, amount: usize) -> Result {
        vec.try_reserve_exact(amount)
    }

    pub fn BumpVec_try_resize(bump: &mut BumpVec<u32>, new_len: usize, value: u32) -> Result {
        bump.try_resize(new_len, value)
    }

    pub fn BumpVec__try_with_capacity_in(capacity: usize, bump: &Bump) -> Result<BumpVec<u32>> {
        BumpVec::try_with_capacity_in(capacity, bump)
    }

    pub fn BumpVec_try_map(bump: BumpVec<u32>, f: fn(u32) -> i16) -> Result<BumpVec<i16>> {
        bump.try_map(f)
    }

    pub fn BumpString__try_from_str_in<'a>(string: &str, bump: &'a Bump) -> Result<BumpString<'a>> {
        BumpString::try_from_str_in(string, bump)
    }

    pub fn BumpString_try_push(string: &mut BumpString, value: char) -> Result {
        string.try_push(value)
    }

    pub fn BumpString_try_push_str(string: &mut BumpString, value: &str) -> Result {
        string.try_push_str(value)
    }

    pub fn BumpString_try_extend_zeroed(vec: &mut BumpString, additional: usize) -> Result {
        vec.try_extend_zeroed(additional)
    }

    pub fn BumpString_try_reserve(vec: &mut BumpString, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn BumpString_try_reserve_exact(vec: &mut BumpString, amount: usize) -> Result {
        vec.try_reserve_exact(amount)
    }

    pub fn BumpString__try_with_capacity_in(capacity: usize, bump: &Bump) -> Result<BumpString> {
        BumpString::try_with_capacity_in(capacity, bump)
    }

    pub fn BumpString_try_into_cstr(string: BumpString) -> Result<&CStr> {
        string.try_into_cstr()
    }

    pub fn FixedBumpVec__new(capacity: usize, bump: &Bump) -> Result<FixedBumpVec<u32>> {
        FixedBumpVec::try_with_capacity_in(capacity, bump)
    }

    pub fn FixedBumpVec_try_from_iter_in<'a>(iter: core::slice::Iter<u32>, bump: &'a Bump) -> Result<FixedBumpVec<'a, u32>> {
        FixedBumpVec::try_from_iter_in(iter.copied(), bump)
    }

    pub fn FixedBumpVec_try_append(vec: &mut FixedBumpVec<u32>, array: [u32; 24]) -> Result {
        vec.try_append(array)
    }

    pub fn FixedBumpVec_try_extend_from_slice_clone(vec: &mut FixedBumpVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn FixedBumpVec_try_extend_from_slice_copy(vec: &mut FixedBumpVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn FixedBumpVec_try_extend_zeroed(vec: &mut FixedBumpVec<u32>, additional: usize) -> Result {
        vec.try_extend_zeroed(additional)
    }

    pub fn FixedBumpVec_try_insert(vec: &mut FixedBumpVec<u32>, index: usize, value: u32) -> Result {
        if index < vec.len() {
            vec.try_insert(index, value)
        } else {
            Ok(())
        }
    }

    pub fn FixedBumpVec_try_push(vec: &mut FixedBumpVec<u32>, value: u32) -> Result {
        vec.try_push(value)
    }

    pub fn FixedBumpVec_try_resize(vec: &mut FixedBumpVec<u32>, new_len: usize, value: u32) -> Result {
        vec.try_resize(new_len, value)
    }

    pub fn FixedBumpString__new(capacity: usize, bump: &Bump) -> Result<FixedBumpString> {
        FixedBumpString::try_with_capacity_in(capacity, bump)
    }

    pub fn FixedBumpString_try_push(string: &mut FixedBumpString, value: char) -> Result {
        string.try_push(value)
    }

    pub fn FixedBumpString_try_push_str(string: &mut FixedBumpString, value: &str) -> Result {
        string.try_push_str(value)
    }

    pub fn FixedBumpString_try_extend_zeroed(vec: &mut FixedBumpString, additional: usize) -> Result {
        vec.try_extend_zeroed(additional)
    }
}
