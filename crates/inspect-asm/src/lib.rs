#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
#![allow(unused_imports)]

use std::{alloc::Layout, ptr::NonNull};

use bump_scope::allocator_api2::alloc::{AllocError, Allocator, Global};

#[derive(Clone, Copy)]
#[repr(align(512))]
#[allow(dead_code)]
pub struct big([u8; 512]);

type vec3 = [u32; 3];

#[derive(Clone, Copy)]
pub struct zst;

trait BumpaloExt {
    fn try_alloc_str(&self, value: &str) -> Result<&mut str, bumpalo::AllocErr>;
    fn try_alloc_slice_copy<T: Copy>(&self, value: &[T]) -> Result<&mut [T], bumpalo::AllocErr>;
    fn try_alloc_slice_clone<T: Clone>(&self, value: &[T]) -> Result<&mut [T], bumpalo::AllocErr>;
}

type Bump<const MIN_ALIGN: usize, const UP: bool> = bump_scope::Bump<Global, MIN_ALIGN, UP>;
type MutBumpVec<'b, 'a, T, const MIN_ALIGN: usize, const UP: bool> =
    bump_scope::MutBumpVec<'b, 'a, T, Global, MIN_ALIGN, UP>;
type MutBumpVecRev<'b, 'a, T, const MIN_ALIGN: usize, const UP: bool> =
    bump_scope::MutBumpVecRev<'b, 'a, T, Global, MIN_ALIGN, UP>;

impl BumpaloExt for bumpalo::Bump {
    fn try_alloc_str(&self, value: &str) -> Result<&mut str, bumpalo::AllocErr> {
        let slice = self.try_alloc_slice_copy(value.as_bytes())?;
        unsafe { Ok(core::str::from_utf8_unchecked_mut(slice)) }
    }

    fn try_alloc_slice_copy<T: Copy>(&self, value: &[T]) -> Result<&mut [T], bumpalo::AllocErr> {
        let layout = Layout::for_value(value);
        let ptr = self.try_alloc_layout(layout)?;
        let len = value.len();

        unsafe {
            let src = value.as_ptr();
            let dst = ptr.cast::<T>().as_ptr();

            core::ptr::copy_nonoverlapping(src, dst, len);
            Ok(core::slice::from_raw_parts_mut(dst, len))
        }
    }

    fn try_alloc_slice_clone<T: Clone>(&self, value: &[T]) -> Result<&mut [T], bumpalo::AllocErr> {
        let layout = Layout::for_value(value);
        let ptr = self.try_alloc_layout(layout)?;
        let len = value.len();

        unsafe {
            let dst = ptr.cast::<T>().as_ptr();

            for (i, val) in value.iter().enumerate() {
                dst.add(i).write(val.clone());
            }

            Ok(core::slice::from_raw_parts_mut(dst, len))
        }
    }
}

macro_rules! cases {
    (
        $(
            mod $mod:ident fn($bump:ident, $value:ident: $value_ty:ty) -> &$($lifetime:lifetime)? mut $ty:ty { $body:expr } in { $try_body:expr }
        )*
    ) => {

        $(
            pub mod $mod {
                use super::*;

                pub fn up$(<$lifetime>)?($bump: &$($lifetime)? Bump<1, true>, $value: $value_ty) -> &$($lifetime)? mut $ty {
                    $body.into_mut()
                }

                pub fn down$(<$lifetime>)?($bump: &$($lifetime)? Bump<1, false>, $value: $value_ty) -> &$($lifetime)? mut $ty {
                    $body.into_mut()
                }

                pub fn up_a$(<$lifetime>)?($bump: &$($lifetime)? Bump<4, true>, $value: $value_ty) -> &$($lifetime)? mut $ty {
                    $body.into_mut()
                }

                pub fn down_a$(<$lifetime>)?($bump: &$($lifetime)? Bump<4, false>, $value: $value_ty) -> &$($lifetime)? mut $ty {
                    $body.into_mut()
                }

                pub fn try_up$(<$lifetime>)?($bump: &$($lifetime)? Bump<1, true>, $value: $value_ty) -> Option<&$($lifetime)? mut $ty> {
                    $try_body.ok().map(|x| x.into_mut())
                }

                pub fn try_down$(<$lifetime>)?($bump: &$($lifetime)? Bump<1, false>, $value: $value_ty) -> Option<&$($lifetime)? mut $ty> {
                    $try_body.ok().map(|x| x.into_mut())
                }

                pub fn try_up_a$(<$lifetime>)?($bump: &$($lifetime)? Bump<4, true>, $value: $value_ty) -> Option<&$($lifetime)? mut $ty> {
                    $try_body.ok().map(|x| x.into_mut())
                }

                pub fn try_down_a$(<$lifetime>)?($bump: &$($lifetime)? Bump<4, false>, $value: $value_ty) -> Option<&$($lifetime)? mut $ty> {
                    $try_body.ok().map(|x| x.into_mut())
                }

                pub fn bumpalo$(<$lifetime>)?($bump: &$($lifetime)? bumpalo::Bump, $value: $value_ty) -> &$($lifetime)? mut $ty {
                    $body
                }

                pub fn try_bumpalo$(<$lifetime>)?($bump: &$($lifetime)? bumpalo::Bump, $value: $value_ty) -> Option<&$($lifetime)? mut $ty> {
                    $try_body.ok()
                }
            }
        )*
    };
}

pub mod alloc_layout {
    use super::*;

    pub fn up(bump: &Bump<1, true>, layout: Layout) -> NonNull<u8> {
        bump.alloc_layout(layout)
    }

    pub fn down(bump: &Bump<1, false>, layout: Layout) -> NonNull<u8> {
        bump.alloc_layout(layout)
    }

    pub fn bumpalo(bump: &bumpalo::Bump, layout: Layout) -> NonNull<u8> {
        bump.alloc_layout(layout)
    }

    pub fn try_up(bump: &Bump<1, true>, layout: Layout) -> Option<NonNull<u8>> {
        bump.try_alloc_layout(layout).ok()
    }

    pub fn try_down(bump: &Bump<1, false>, layout: Layout) -> Option<NonNull<u8>> {
        bump.try_alloc_layout(layout).ok()
    }

    pub fn try_bumpalo(bump: &bumpalo::Bump, layout: Layout) -> Option<NonNull<u8>> {
        bump.try_alloc_layout(layout).ok()
    }
}

cases! {
    mod alloc_zst fn(bump, value: zst) -> &mut zst { bump.alloc(value) } in { bump.try_alloc(value) }
    mod alloc_u8 fn(bump, value: u8) -> &mut u8 { bump.alloc(value) } in { bump.try_alloc(value) }
    mod alloc_u32 fn(bump, value: u32) -> &mut u32 { bump.alloc(value) } in { bump.try_alloc(value) }
    mod alloc_vec3 fn(bump, value: vec3) -> &mut vec3 { bump.alloc(value) } in { bump.try_alloc(value) }
    mod alloc_big fn(bump, value: &big) -> &'a mut big { bump.alloc_with(|| *value) } in { bump.try_alloc_with(|| *value) }
    mod alloc_str fn(bump, value: &str) -> &'a mut str { bump.alloc_str(value) } in { bump.try_alloc_str(value) }
    mod alloc_u32_slice fn(bump, value: &[u32]) -> &'a mut [u32] { bump.alloc_slice_copy(value) } in { bump.try_alloc_slice_copy(value) }
    mod alloc_u32_slice_clone fn(bump, value: &[u32]) -> &'a mut [u32] { bump.alloc_slice_clone(value) } in { bump.try_alloc_slice_clone(value) }
}

pub mod alloc_overaligned_but_size_matches {
    use super::*;

    pub fn up(bump: &Bump<4, true>, value: [u8; 4]) -> &[u8; 4] {
        bump.alloc(value).into_ref()
    }

    pub fn down(bump: &Bump<4, false>, value: [u8; 4]) -> &[u8; 4] {
        bump.alloc(value).into_ref()
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct foo(u64);

#[cold]
#[inline(never)]
fn something() {
    println!("Hey");
}

impl Drop for foo {
    fn drop(&mut self) {
        something();
    }
}

#[cfg(any())]
pub mod alloc_with_drop {
    use super::*;

    type WithDrop<const MIN_ALIGN: usize, const UP: bool> =
        bump_scope::WithDrop<Global, MIN_ALIGN, UP, Bump<Global, MIN_ALIGN, UP>>;

    pub fn up(bump: &WithDrop<1, true>, value: foo) -> &mut foo {
        bump.alloc(value)
    }

    pub fn down(bump: &WithDrop<1, false>, value: foo) -> &mut foo {
        bump.alloc(value)
    }

    pub fn up_a(bump: &WithDrop<8, true>, value: foo) -> &mut foo {
        bump.alloc(value)
    }

    pub fn down_a(bump: &WithDrop<8, false>, value: foo) -> &mut foo {
        bump.alloc(value)
    }

    pub fn try_up(bump: &WithDrop<1, true>, value: foo) -> Option<&mut foo> {
        bump.try_alloc(value).ok()
    }

    pub fn try_down(bump: &WithDrop<1, false>, value: foo) -> Option<&mut foo> {
        bump.try_alloc(value).ok()
    }

    pub fn try_up_a(bump: &WithDrop<8, true>, value: foo) -> Option<&mut foo> {
        bump.try_alloc(value).ok()
    }

    pub fn try_down_a(bump: &WithDrop<8, false>, value: foo) -> Option<&mut foo> {
        bump.try_alloc(value).ok()
    }
}

pub mod allocate {
    use super::*;

    pub fn up(bump: &Bump<1, true>, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        bump.allocate(layout)
    }

    pub fn down(bump: &Bump<1, false>, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        bump.allocate(layout)
    }

    pub fn bumpalo(bump: &bumpalo::Bump, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        bump.allocate(layout)
    }
}

pub mod deallocate {
    use super::*;

    pub unsafe fn up(bump: &Bump<1, true>, ptr: NonNull<u8>, layout: Layout) {
        bump.deallocate(ptr, layout)
    }

    pub unsafe fn down(bump: &Bump<1, false>, ptr: NonNull<u8>, layout: Layout) {
        bump.deallocate(ptr, layout)
    }

    pub unsafe fn bumpalo(bump: &bumpalo::Bump, ptr: NonNull<u8>, layout: Layout) {
        bump.deallocate(ptr, layout)
    }
}

pub mod grow {
    use super::*;

    pub unsafe fn up(
        bump: &Bump<1, true>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        bump.grow(ptr, old_layout, new_layout)
    }

    pub unsafe fn down(
        bump: &Bump<1, false>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        bump.grow(ptr, old_layout, new_layout)
    }

    pub unsafe fn bumpalo(
        bump: &bumpalo::Bump,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        bump.grow(ptr, old_layout, new_layout)
    }
}

pub mod shrink {
    use super::*;

    pub unsafe fn up(
        bump: &Bump<1, true>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        bump.shrink(ptr, old_layout, new_layout)
    }

    pub unsafe fn down(
        bump: &Bump<1, false>,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        bump.shrink(ptr, old_layout, new_layout)
    }

    pub unsafe fn bumpalo(
        bump: &bumpalo::Bump,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        bump.shrink(ptr, old_layout, new_layout)
    }
}

macro_rules! cases_bump_vec {
    ($vec:ident, $ty:ident, $up:ident) => {
        use super::*;

        pub fn with_capacity(capacity: usize, bump: &mut Bump<1, {$up}>) -> $vec<$ty, 1, {$up}> {
            $vec::with_capacity_in(capacity, bump)
        }

        pub fn push(bump: &mut $vec<$ty, 1, {$up}>, value: $ty) {
            bump.push(value)
        }

        pub fn try_with_capacity(capacity: usize, bump: &mut Bump<1, {$up}>) -> Result<$vec<$ty, 1, {$up}>, AllocError> {
            $vec::try_with_capacity_in(capacity, bump)
        }

        pub fn try_push(bump: &mut $vec<$ty, 1, {$up}>, value: $ty) -> Result<(), AllocError> {
            bump.try_push(value)
        }
    };
    ($(mod $mod:ident for $vec:ident $ty:ident)*) => {
        $(
            pub mod $mod {
                use super::*;

                pub mod up {
                    cases_bump_vec!($vec, $ty, true);
                }

                pub mod down {
                    cases_bump_vec!($vec, $ty, false);
                }
            }
        )*
    };
}

cases_bump_vec! {
    mod bump_vec_u32 for MutBumpVec u32
    mod bump_vec_u32_rev for MutBumpVec u32
}

pub mod alloc_iter_u32 {
    use bump_scope::BumpBox;

    use super::*;

    pub fn up<'a>(bump: &'a Bump<1, true>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter(slice.iter().copied()).into_mut()
    }

    pub fn up_a<'a>(bump: &'a Bump<4, true>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter(slice.iter().copied()).into_mut()
    }

    pub fn down<'a>(bump: &'a Bump<1, false>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter(slice.iter().copied()).into_mut()
    }

    pub fn down_a<'a>(bump: &'a Bump<4, false>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter(slice.iter().copied()).into_mut()
    }

    pub fn exact_up<'a>(bump: &'a Bump<1, true>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_exact(slice.iter().copied()).into_mut()
    }

    pub fn exact_up_a<'a>(bump: &'a Bump<4, true>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_exact(slice.iter().copied()).into_mut()
    }

    pub fn exact_down<'a>(bump: &'a Bump<1, false>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_exact(slice.iter().copied()).into_mut()
    }

    pub fn exact_down_a<'a>(bump: &'a Bump<4, false>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_exact(slice.iter().copied()).into_mut()
    }

    pub fn mut_up<'a>(bump: &'a mut Bump<1, true>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_mut(slice.iter().copied()).into_mut()
    }

    pub fn mut_up_a<'a>(bump: &'a mut Bump<4, true>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_mut(slice.iter().copied()).into_mut()
    }

    pub fn mut_down<'a>(bump: &'a mut Bump<1, false>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_mut(slice.iter().copied()).into_mut()
    }

    pub fn mut_down_a<'a>(bump: &'a mut Bump<4, false>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_mut(slice.iter().copied()).into_mut()
    }

    pub fn mut_rev_up<'a>(bump: &'a mut Bump<1, true>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_mut_rev(slice.iter().copied()).into_mut()
    }

    pub fn mut_rev_up_a<'a>(bump: &'a mut Bump<4, true>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_mut_rev(slice.iter().copied()).into_mut()
    }

    pub fn mut_rev_down<'a>(bump: &'a mut Bump<1, false>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_mut_rev(slice.iter().copied()).into_mut()
    }

    pub fn mut_rev_down_a<'a>(bump: &'a mut Bump<4, false>, slice: &[u32]) -> &'a [u32] {
        bump.alloc_iter_mut_rev(slice.iter().copied()).into_mut()
    }

    pub fn try_up<'a>(bump: &'a Bump<1, true>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_up_a<'a>(bump: &'a Bump<4, true>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_down<'a>(bump: &'a Bump<1, false>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_down_a<'a>(bump: &'a Bump<4, false>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_exact_up<'a>(bump: &'a Bump<1, true>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_exact(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_exact_up_a<'a>(bump: &'a Bump<4, true>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_exact(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_exact_down<'a>(bump: &'a Bump<1, false>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_exact(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_exact_down_a<'a>(bump: &'a Bump<4, false>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_exact(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_mut_up<'a>(bump: &'a mut Bump<1, true>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_mut(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_mut_up_a<'a>(bump: &'a mut Bump<4, true>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_mut(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_mut_down<'a>(bump: &'a mut Bump<1, false>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_mut(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_mut_down_a<'a>(bump: &'a mut Bump<4, false>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_mut(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_mut_rev_up<'a>(bump: &'a mut Bump<1, true>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_mut_rev(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_mut_rev_up_a<'a>(bump: &'a mut Bump<4, true>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_mut_rev(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_mut_rev_down<'a>(bump: &'a mut Bump<1, false>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_mut_rev(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn try_mut_rev_down_a<'a>(bump: &'a mut Bump<4, false>, slice: &[u32]) -> Option<&'a [u32]> {
        bump.try_alloc_iter_mut_rev(slice.iter().copied()).ok().map(BumpBox::into_ref)
    }

    pub fn bumpalo<'a>(bump: &'a bumpalo::Bump, slice: &[u32]) -> &'a [u32] {
        bump.alloc_slice_fill_iter(slice.iter().copied())
    }
}

pub mod alloc_iter_u32_bump_vec {
    use super::*;

    pub fn up<'a>(bump: &'a mut Bump<1, true>, slice: &[u32]) -> &'a [u32] {
        let mut vec = MutBumpVec::new_in(bump);
        vec.extend(slice.iter().copied());
        vec.into_boxed_slice().into_ref()
    }

    pub fn up_a<'a>(bump: &'a mut Bump<4, true>, slice: &[u32]) -> &'a [u32] {
        let mut vec = MutBumpVec::new_in(bump);
        vec.extend(slice.iter().copied());
        vec.into_boxed_slice().into_ref()
    }

    pub fn down<'a>(bump: &'a mut Bump<1, false>, slice: &[u32]) -> &'a [u32] {
        let mut vec = MutBumpVec::new_in(bump);
        vec.extend(slice.iter().copied());
        vec.into_boxed_slice().into_ref()
    }

    pub fn down_a<'a>(bump: &'a mut Bump<4, false>, slice: &[u32]) -> &'a [u32] {
        let mut vec = MutBumpVec::new_in(bump);
        vec.extend(slice.iter().copied());
        vec.into_boxed_slice().into_ref()
    }

    pub fn rev_up<'a>(bump: &'a mut Bump<1, true>, slice: &[u32]) -> &'a [u32] {
        let mut vec = MutBumpVecRev::new_in(bump);
        vec.extend(slice.iter().copied());
        vec.into_boxed_slice().into_ref()
    }

    pub fn rev_up_a<'a>(bump: &'a mut Bump<4, true>, slice: &[u32]) -> &'a [u32] {
        let mut vec = MutBumpVecRev::new_in(bump);
        vec.extend(slice.iter().copied());
        vec.into_boxed_slice().into_ref()
    }

    pub fn rev_down<'a>(bump: &'a mut Bump<1, false>, slice: &[u32]) -> &'a [u32] {
        let mut vec = MutBumpVecRev::new_in(bump);
        vec.extend(slice.iter().copied());
        vec.into_boxed_slice().into_ref()
    }

    pub fn rev_down_a<'a>(bump: &'a mut Bump<4, false>, slice: &[u32]) -> &'a [u32] {
        let mut vec = MutBumpVecRev::new_in(bump);
        vec.extend(slice.iter().copied());
        vec.into_boxed_slice().into_ref()
    }
}

pub mod alloc_fmt {
    use bump_scope::BumpBox;

    use super::*;

    pub fn up<'a>(bump: &'a Bump<1, true>, display: &str) -> &'a str {
        bump.alloc_fmt(format_args!("begin{display}end")).into_ref()
    }

    pub fn up_a<'a>(bump: &'a Bump<4, true>, display: &str) -> &'a str {
        bump.alloc_fmt(format_args!("begin{display}end")).into_ref()
    }

    pub fn down<'a>(bump: &'a Bump<1, false>, display: &str) -> &'a str {
        bump.alloc_fmt(format_args!("begin{display}end")).into_ref()
    }

    pub fn down_a<'a>(bump: &'a Bump<4, false>, display: &str) -> &'a str {
        bump.alloc_fmt(format_args!("begin{display}end")).into_ref()
    }

    pub fn mut_up<'a>(bump: &'a mut Bump<1, true>, display: &str) -> &'a str {
        bump.alloc_fmt_mut(format_args!("begin{display}end")).into_ref()
    }

    pub fn mut_up_a<'a>(bump: &'a mut Bump<4, true>, display: &str) -> &'a str {
        bump.alloc_fmt_mut(format_args!("begin{display}end")).into_ref()
    }

    pub fn mut_down<'a>(bump: &'a mut Bump<1, false>, display: &str) -> &'a str {
        bump.alloc_fmt_mut(format_args!("begin{display}end")).into_ref()
    }

    pub fn mut_down_a<'a>(bump: &'a mut Bump<4, false>, display: &str) -> &'a str {
        bump.alloc_fmt_mut(format_args!("begin{display}end")).into_ref()
    }

    pub fn try_up<'a>(bump: &'a Bump<1, true>, display: &str) -> Option<&'a str> {
        bump.try_alloc_fmt(format_args!("begin{display}end"))
            .ok()
            .map(BumpBox::into_ref)
    }

    pub fn try_up_a<'a>(bump: &'a Bump<4, true>, display: &str) -> Option<&'a str> {
        bump.try_alloc_fmt(format_args!("begin{display}end"))
            .ok()
            .map(BumpBox::into_ref)
    }

    pub fn try_down<'a>(bump: &'a Bump<1, false>, display: &str) -> Option<&'a str> {
        bump.try_alloc_fmt(format_args!("begin{display}end"))
            .ok()
            .map(BumpBox::into_ref)
    }

    pub fn try_down_a<'a>(bump: &'a Bump<4, false>, display: &str) -> Option<&'a str> {
        bump.try_alloc_fmt(format_args!("begin{display}end"))
            .ok()
            .map(BumpBox::into_ref)
    }

    pub fn try_mut_up<'a>(bump: &'a mut Bump<1, true>, display: &str) -> Option<&'a str> {
        bump.try_alloc_fmt_mut(format_args!("begin{display}end"))
            .ok()
            .map(BumpBox::into_ref)
    }

    pub fn try_mut_up_a<'a>(bump: &'a mut Bump<4, true>, display: &str) -> Option<&'a str> {
        bump.try_alloc_fmt_mut(format_args!("begin{display}end"))
            .ok()
            .map(BumpBox::into_ref)
    }

    pub fn try_mut_down<'a>(bump: &'a mut Bump<1, false>, display: &str) -> Option<&'a str> {
        bump.try_alloc_fmt_mut(format_args!("begin{display}end"))
            .ok()
            .map(BumpBox::into_ref)
    }

    pub fn try_mut_down_a<'a>(bump: &'a mut Bump<4, false>, display: &str) -> Option<&'a str> {
        bump.try_alloc_fmt_mut(format_args!("begin{display}end"))
            .ok()
            .map(BumpBox::into_ref)
    }
}
