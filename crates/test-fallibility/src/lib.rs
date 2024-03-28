#![no_std]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]
extern crate alloc;

use core::{alloc::Layout, fmt, mem::MaybeUninit, ptr::NonNull};

use alloc::boxed::Box;
use bump_scope::{
    allocator_api2::alloc::{AllocError, Allocator, Global},
    BumpBox,
};

type Result<T = (), E = AllocError> = core::result::Result<T, E>;

macro_rules! type_definitions {
    ($up:literal) => {
        type Bump<const MIN_ALIGN: usize = 1> = bump_scope::Bump<Global, MIN_ALIGN, $up>;
        type BumpScope<'a, const MIN_ALIGN: usize = 1> = bump_scope::BumpScope<'a, Global, MIN_ALIGN, $up>;
        type BumpScopeGuard<'a, const MIN_ALIGN: usize = 1> = bump_scope::BumpScopeGuard<'a, Global, MIN_ALIGN, $up>;
        type BumpScopeGuardRoot<'a, const MIN_ALIGN: usize = 1> = bump_scope::BumpScopeGuardRoot<'a, Global, MIN_ALIGN, $up>;
        type MutBumpVec<'b, 'a, T, const MIN_ALIGN: usize = 1> = bump_scope::MutBumpVec<'b, 'a, T, Global, MIN_ALIGN, $up>;
        type MutBumpVecRev<'b, 'a, T, const MIN_ALIGN: usize = 1> = bump_scope::MutBumpVecRev<'b, 'a, T, Global, MIN_ALIGN, $up>;
        type MutBumpString<'b, 'a, const MIN_ALIGN: usize = 1> = bump_scope::MutBumpString<'b, 'a, Global, MIN_ALIGN, $up>;
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

    pub fn Bump_as_aligned_mut(bump: &mut Bump) -> &mut Bump<4> {
        bump.as_aligned_mut()
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

    pub fn Bump_try_alloc_fmt<'a>(bump: &'a Bump, args: fmt::Arguments) -> Result<BumpBox<'a, str>> {
        bump.try_alloc_fmt(args)
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

    pub fn Bump_try_alloc_layout(bump: &Bump, layout: Layout) -> Result<NonNull<u8>> {
        bump.try_alloc_layout(layout)
    }

    pub fn Bump_try_alloc_slice_clone<'a>(bump: &'a Bump, value: &[u32]) -> Result<BumpBox<'a, [u32]>> {
        bump.try_alloc_slice_copy(value)
    }

    pub fn Bump_try_alloc_slice_copy<'a>(bump: &'a Bump, value: &[u32]) -> Result<BumpBox<'a, [u32]>> {
        bump.try_alloc_slice_copy(value)
    }

    pub fn Bump_try_alloc_slice_fill(bump: &Bump, len: usize, value: u32) -> Result<BumpBox<[u32]>> {
        bump.try_alloc_slice_fill(len, value)
    }

    pub fn Bump_try_alloc_slice_fill_with<'a>(bump: &'a Bump, len: usize, f: &mut dyn FnMut() -> u32) -> Result<BumpBox<'a, [u32]>> {
        bump.try_alloc_slice_fill_with(len, f)
    }

    pub fn Bump_try_alloc_str<'a>(bump: &'a Bump, value: &str) -> Result<BumpBox<'a, str>> {
        bump.try_alloc_str(value)
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

    pub fn BumpVec_try_extend_from_array(vec: &mut MutBumpVec<u32>, array: [u32; 24]) -> Result {
        vec.try_extend_from_array(array)
    }

    pub fn BumpVec_try_extend_from_slice_clone(vec: &mut MutBumpVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn BumpVec_try_extend_from_slice_copy(vec: &mut MutBumpVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn BumpVec__try_from_array_in(array: [u32; 24], bump: &mut Bump) -> Result<MutBumpVec<u32>> {
        MutBumpVec::try_from_array_in(array, bump)
    }

    pub fn BumpVec__try_from_elem_in(value: u32, count: usize, bump: &mut Bump) -> Result<MutBumpVec<u32>> {
        MutBumpVec::try_from_elem_in(value, count, bump)
    }

    pub fn BumpVec_try_insert(bump: &mut MutBumpVec<u32>, index: usize, value: u32) -> Result {
        bump.try_insert(index, value)
    }

    pub fn BumpVec_try_push(bump: &mut MutBumpVec<u32>, value: u32) -> Result {
        bump.try_push(value)
    }

    pub fn BumpVec_try_reserve(vec: &mut MutBumpVec<u32>, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn BumpVec_try_resize(bump: &mut MutBumpVec<u32>, new_len: usize, value: u32) -> Result {
        bump.try_resize(new_len, value)
    }

    pub fn BumpVec__try_with_capacity_in(capacity: usize, bump: &mut Bump) -> Result<MutBumpVec<u32>> {
        MutBumpVec::try_with_capacity_in(capacity, bump)
    }

    pub fn BumpVecRev_try_extend_from_array(vec: &mut MutBumpVecRev<u32>, array: [u32; 24]) -> Result {
        vec.try_extend_from_array(array)
    }

    pub fn BumpVecRev_try_extend_from_slice_clone(vec: &mut MutBumpVecRev<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn BumpVecRev_try_extend_from_slice_copy(vec: &mut MutBumpVecRev<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn BumpVecRev__try_from_array_in(array: [u32; 24], bump: &mut Bump) -> Result<MutBumpVecRev<u32>> {
        MutBumpVecRev::try_from_array_in(array, bump)
    }

    pub fn BumpVecRev__try_from_elem_in(value: u32, count: usize, bump: &mut Bump) -> Result<MutBumpVecRev<u32>> {
        MutBumpVecRev::try_from_elem_in(value, count, bump)
    }

    pub fn BumpVecRev_try_insert(bump: &mut MutBumpVecRev<u32>, index: usize, value: u32) -> Result {
        bump.try_insert(index, value)
    }

    pub fn BumpVecRev_try_push(bump: &mut MutBumpVecRev<u32>, value: u32) -> Result {
        bump.try_push(value)
    }

    pub fn BumpVecRev_try_reserve(vec: &mut MutBumpVecRev<u32>, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn BumpVecRev_try_resize(bump: &mut MutBumpVecRev<u32>, new_len: usize, value: u32) -> Result {
        bump.try_resize(new_len, value)
    }

    pub fn BumpVecRev__try_with_capacity_in(capacity: usize, bump: &mut Bump) -> Result<MutBumpVecRev<u32>> {
        MutBumpVecRev::try_with_capacity_in(capacity, bump)
    }

    pub fn BumpString__try_from_str_in<'b>(string: &str, bump: &'b mut Bump) -> Result<MutBumpString<'b, 'b>> {
        MutBumpString::try_from_str_in(string, bump)
    }

    pub fn BumpString_try_push(bump: &mut MutBumpString, value: char) -> Result {
        bump.try_push(value)
    }

    pub fn BumpString_try_push_str(bump: &mut MutBumpString, value: &str) -> Result {
        bump.try_push_str(value)
    }

    pub fn BumpString_try_reserve(vec: &mut MutBumpString, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn BumpString__try_with_capacity_in(capacity: usize, bump: &mut Bump) -> Result<MutBumpString> {
        MutBumpString::try_with_capacity_in(capacity, bump)
    }
}
