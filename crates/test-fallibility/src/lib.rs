#![no_std]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]
extern crate alloc;

use core::{alloc::Layout, fmt, mem::MaybeUninit, ptr::NonNull};

use alloc::boxed::Box as StdBox;

use bump_scope::{
    allocator_api2::alloc::{AllocError, Allocator, Global},
    Box, FixedString, FixedVec,
};

type Result<T = (), E = AllocError> = core::result::Result<T, E>;

macro_rules! type_definitions {
    ($up:literal) => {
        type Bump<const MIN_ALIGN: usize = 1> = bump_scope::Bump<Global, MIN_ALIGN, $up>;
        type BumpScope<'a, const MIN_ALIGN: usize = 1> = bump_scope::BumpScope<'a, Global, MIN_ALIGN, $up>;
        type ScopeGuard<'a, const MIN_ALIGN: usize = 1> = bump_scope::ScopeGuard<'a, Global, MIN_ALIGN, $up>;
        type ScopeGuardRoot<'a, const MIN_ALIGN: usize = 1> = bump_scope::ScopeGuardRoot<'a, Global, MIN_ALIGN, $up>;
        type Vec<'b, 'a, T, const MIN_ALIGN: usize = 1> = bump_scope::Vec<'b, 'a, T, Global, MIN_ALIGN, $up>;
        type String<'b, 'a, const MIN_ALIGN: usize = 1> = bump_scope::String<'b, 'a, Global, MIN_ALIGN, $up>;
        type MutVec<'b, 'a, T, const MIN_ALIGN: usize = 1> = bump_scope::MutVec<'b, 'a, T, Global, MIN_ALIGN, $up>;
        type MutVecRev<'b, 'a, T, const MIN_ALIGN: usize = 1> = bump_scope::MutVecRev<'b, 'a, T, Global, MIN_ALIGN, $up>;
        type MutString<'b, 'a, const MIN_ALIGN: usize = 1> = bump_scope::MutString<'b, 'a, Global, MIN_ALIGN, $up>;
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

    pub fn Bump_scoped(bump: &mut Bump, f: StdBox<dyn FnOnce(BumpScope)>) {
        bump.scoped(f)
    }

    pub fn Bump_aligned_inc(bump: &mut Bump, f: StdBox<dyn FnOnce(BumpScope<8>)>) {
        bump.aligned(f)
    }

    pub fn Bump_aligned_dec(bump: &mut Bump<8>, f: StdBox<dyn FnOnce(BumpScope)>) {
        bump.aligned(f)
    }

    pub fn Bump_scope_guard(bump: &mut Bump) -> ScopeGuardRoot {
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

    pub fn Bump_try_alloc(bump: &Bump, value: u32) -> Result<Box<u32>> {
        bump.try_alloc(value)
    }

    pub fn Bump_try_alloc_default(bump: &Bump) -> Result<Box<u32>> {
        bump.try_alloc_default()
    }

    pub fn Bump_try_alloc_fmt<'a>(bump: &'a Bump, args: fmt::Arguments) -> Result<Box<'a, str>> {
        bump.try_alloc_fmt(args)
    }

    pub fn Bump_try_alloc_iter(bump: &Bump, value: core::ops::Range<u32>) -> Result<Box<[u32]>> {
        bump.try_alloc_iter(value)
    }

    pub fn Bump_try_alloc_iter_exact(bump: &Bump, value: core::ops::Range<u32>) -> Result<Box<[u32]>> {
        bump.try_alloc_iter_exact(value)
    }

    pub fn Bump_try_alloc_iter_mut(bump: &mut Bump, value: core::ops::Range<u32>) -> Result<Box<[u32]>> {
        bump.try_alloc_iter_mut(value)
    }

    pub fn Bump_try_alloc_iter_mut_rev(bump: &mut Bump, value: core::ops::Range<u32>) -> Result<Box<[u32]>> {
        bump.try_alloc_iter_mut_rev(value)
    }

    pub fn Bump_try_alloc_layout(bump: &Bump, layout: Layout) -> Result<NonNull<u8>> {
        bump.try_alloc_layout(layout)
    }

    pub fn Bump_try_alloc_slice_clone<'a>(bump: &'a Bump, value: &[u32]) -> Result<Box<'a, [u32]>> {
        bump.try_alloc_slice_copy(value)
    }

    pub fn Bump_try_alloc_slice_copy<'a>(bump: &'a Bump, value: &[u32]) -> Result<Box<'a, [u32]>> {
        bump.try_alloc_slice_copy(value)
    }

    pub fn Bump_try_alloc_slice_fill(bump: &Bump, len: usize, value: u32) -> Result<Box<[u32]>> {
        bump.try_alloc_slice_fill(len, value)
    }

    pub fn Bump_try_alloc_slice_fill_with<'a>(bump: &'a Bump, len: usize, f: &mut dyn FnMut() -> u32) -> Result<Box<'a, [u32]>> {
        bump.try_alloc_slice_fill_with(len, f)
    }

    pub fn Bump_try_alloc_str<'a>(bump: &'a Bump, value: &str) -> Result<Box<'a, str>> {
        bump.try_alloc_str(value)
    }

    pub fn Bump_try_alloc_uninit(bump: &Bump) -> Result<Box<MaybeUninit<u32>>> {
        bump.try_alloc_uninit()
    }

    pub fn Bump_try_alloc_uninit_slice(bump: &Bump, len: usize) -> Result<Box<[MaybeUninit<u32>]>> {
        bump.try_alloc_uninit_slice(len)
    }

    pub fn Bump_try_alloc_uninit_slice_for<'a>(bump: &'a Bump, slice: &[u32]) -> Result<Box<'a, [MaybeUninit<u32>]>> {
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

    pub fn Scope__scope_guard<'b>(bump: &'b mut BumpScope) -> ScopeGuard<'b> {
        bump.scope_guard()
    }

    pub fn MutVec_try_extend_from_array(vec: &mut MutVec<u32>, array: [u32; 24]) -> Result {
        vec.try_extend_from_array(array)
    }

    pub fn MutVec_try_extend_from_slice_clone(vec: &mut MutVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn MutVec_try_extend_from_slice_copy(vec: &mut MutVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn MutVec__try_from_array_in(array: [u32; 24], bump: &mut Bump) -> Result<MutVec<u32>> {
        MutVec::try_from_array_in(array, bump)
    }

    pub fn MutVec__try_from_elem_in(value: u32, count: usize, bump: &mut Bump) -> Result<MutVec<u32>> {
        MutVec::try_from_elem_in(value, count, bump)
    }

    pub fn MutVec_try_insert(bump: &mut MutVec<u32>, index: usize, value: u32) -> Result {
        bump.try_insert(index, value)
    }

    pub fn MutVec_try_push(bump: &mut MutVec<u32>, value: u32) -> Result {
        bump.try_push(value)
    }

    pub fn MutVec_try_reserve(vec: &mut MutVec<u32>, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn MutVec_try_resize(bump: &mut MutVec<u32>, new_len: usize, value: u32) -> Result {
        bump.try_resize(new_len, value)
    }

    pub fn MutVec__try_with_capacity_in(capacity: usize, bump: &mut Bump) -> Result<MutVec<u32>> {
        MutVec::try_with_capacity_in(capacity, bump)
    }

    pub fn MutVecRev_try_extend_from_array(vec: &mut MutVecRev<u32>, array: [u32; 24]) -> Result {
        vec.try_extend_from_array(array)
    }

    pub fn MutVecRev_try_extend_from_slice_clone(vec: &mut MutVecRev<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn MutVecRev_try_extend_from_slice_copy(vec: &mut MutVecRev<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn MutVecRev__try_from_array_in(array: [u32; 24], bump: &mut Bump) -> Result<MutVecRev<u32>> {
        MutVecRev::try_from_array_in(array, bump)
    }

    pub fn MutVecRev__try_from_elem_in(value: u32, count: usize, bump: &mut Bump) -> Result<MutVecRev<u32>> {
        MutVecRev::try_from_elem_in(value, count, bump)
    }

    pub fn MutVecRev_try_insert(bump: &mut MutVecRev<u32>, index: usize, value: u32) -> Result {
        bump.try_insert(index, value)
    }

    pub fn MutVecRev_try_push(bump: &mut MutVecRev<u32>, value: u32) -> Result {
        bump.try_push(value)
    }

    pub fn MutVecRev_try_reserve(vec: &mut MutVecRev<u32>, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn MutVecRev_try_resize(bump: &mut MutVecRev<u32>, new_len: usize, value: u32) -> Result {
        bump.try_resize(new_len, value)
    }

    pub fn MutVecRev__try_with_capacity_in(capacity: usize, bump: &mut Bump) -> Result<MutVecRev<u32>> {
        MutVecRev::try_with_capacity_in(capacity, bump)
    }

    pub fn MutString__try_from_str_in<'b>(string: &str, bump: &'b mut Bump) -> Result<MutString<'b, 'b>> {
        MutString::try_from_str_in(string, bump)
    }

    pub fn MutString_try_push(bump: &mut MutString, value: char) -> Result {
        bump.try_push(value)
    }

    pub fn MutString_try_push_str(bump: &mut MutString, value: &str) -> Result {
        bump.try_push_str(value)
    }

    pub fn MutString_try_reserve(vec: &mut MutString, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn MutString__try_with_capacity_in(capacity: usize, bump: &mut Bump) -> Result<MutString> {
        MutString::try_with_capacity_in(capacity, bump)
    }

    pub fn Vec_try_extend_from_array(vec: &mut Vec<u32>, array: [u32; 24]) -> Result {
        vec.try_extend_from_array(array)
    }

    pub fn Vec_try_extend_from_slice_clone(vec: &mut Vec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn Vec_try_extend_from_slice_copy(vec: &mut Vec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn Vec__try_from_array_in(array: [u32; 24], bump: &Bump) -> Result<Vec<u32>> {
        Vec::try_from_array_in(array, bump)
    }

    pub fn Vec__try_from_elem_in(value: u32, count: usize, bump: &Bump) -> Result<Vec<u32>> {
        Vec::try_from_elem_in(value, count, bump)
    }

    pub fn Vec_try_insert(bump: &mut Vec<u32>, index: usize, value: u32) -> Result {
        bump.try_insert(index, value)
    }

    pub fn Vec_try_push(bump: &mut Vec<u32>, value: u32) -> Result {
        bump.try_push(value)
    }

    pub fn Vec_try_reserve(vec: &mut Vec<u32>, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn Vec_try_resize(bump: &mut Vec<u32>, new_len: usize, value: u32) -> Result {
        bump.try_resize(new_len, value)
    }

    pub fn Vec__try_with_capacity_in(capacity: usize, bump: &Bump) -> Result<Vec<u32>> {
        Vec::try_with_capacity_in(capacity, bump)
    }

    pub fn String__try_from_str_in<'b>(string: &str, bump: &'b Bump) -> Result<String<'b, 'b>> {
        String::try_from_str_in(string, bump)
    }

    pub fn String_try_push(bump: &mut String, value: char) -> Result {
        bump.try_push(value)
    }

    pub fn String_try_push_str(bump: &mut String, value: &str) -> Result {
        bump.try_push_str(value)
    }

    pub fn String_try_reserve(vec: &mut String, amount: usize) -> Result {
        vec.try_reserve(amount)
    }

    pub fn String__try_with_capacity_in(capacity: usize, bump: &Bump) -> Result<String> {
        String::try_with_capacity_in(capacity, bump)
    }

    pub fn FixedVec__new(capacity: usize, bump: &Bump) -> Result<FixedVec<u32>> {
        bump.try_alloc_fixed_vec(capacity)
    }

    pub fn FixedVec_try_extend_from_array(vec: &mut FixedVec<u32>, array: [u32; 24]) -> Result {
        vec.try_extend_from_array(array)
    }

    pub fn FixedVec_try_extend_from_slice_clone(vec: &mut FixedVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_clone(slice)
    }

    pub fn FixedVec_try_extend_from_slice_copy(vec: &mut FixedVec<u32>, slice: &[u32]) -> Result {
        vec.try_extend_from_slice_copy(slice)
    }

    pub fn FixedVec_try_insert(bump: &mut FixedVec<u32>, index: usize, value: u32) -> Result {
        bump.try_insert(index, value)
    }

    pub fn FixedVec_try_push(bump: &mut FixedVec<u32>, value: u32) -> Result {
        bump.try_push(value)
    }

    pub fn FixedVec_try_resize(bump: &mut FixedVec<u32>, new_len: usize, value: u32) -> Result {
        bump.try_resize(new_len, value)
    }

    pub fn FixedString__new(capacity: usize, bump: &Bump) -> Result<FixedString> {
        bump.try_alloc_fixed_string(capacity)
    }

    pub fn FixedString_try_push(bump: &mut FixedString, value: char) -> Result {
        bump.try_push(value)
    }

    pub fn FixedString_try_push_str(bump: &mut FixedString, value: &str) -> Result {
        bump.try_push_str(value)
    }
}
