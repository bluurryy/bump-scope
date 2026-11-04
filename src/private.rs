use core::{
    alloc::{AllocError, Allocator, Layout},
    ptr,
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    BumpAllocator,
    owned_slice::{OwnedSlice, TakeOwnedSlice as _},
};

pub extern crate alloc;
pub extern crate core;

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

#[cold]
#[inline(never)]
#[cfg(feature = "panic-on-alloc")]
pub const fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}

#[cold]
#[inline(never)]
#[cfg(feature = "panic-on-alloc")]
pub const fn format_trait_error() -> ! {
    panic!("formatting trait implementation returned an error");
}

pub fn box_try_new_in<T, A>(value: T, alloc: A) -> Result<Box<T, A>, AllocError>
where
    A: Allocator,
{
    let layout = Layout::new::<T>();
    let slice = alloc.allocate(layout)?;
    let ptr = slice.as_ptr().cast::<T>();

    unsafe {
        ptr.write(value);
        Ok(Box::from_raw_in(ptr, alloc))
    }
}

#[cfg(feature = "panic-on-alloc")]
#[cfg(feature = "nightly")]
pub use alloc::vec::from_elem_in as vec_from_elem_in;

#[cfg(feature = "panic-on-alloc")]
#[cfg(not(feature = "nightly"))]
pub fn vec_from_elem_in<T, A>(value: T, count: usize, alloc: A) -> Vec<T, A>
where
    T: Clone,
    A: Allocator,
{
    let Ok(layout) = Layout::array::<T>(count) else {
        capacity_overflow()
    };
    let Ok(slice) = alloc.allocate(layout) else {
        alloc::alloc::handle_alloc_error(layout)
    };
    let ptr = slice.cast::<T>().as_ptr();
    let mut vec = unsafe { Vec::from_raw_parts_in(ptr, 0, count, alloc) };
    vec.extend(core::iter::repeat_n(value, count));
    vec
}

pub fn vec_try_from_elem_in<T, A>(value: T, count: usize, alloc: A) -> Result<Vec<T, A>, AllocError>
where
    T: Clone,
    A: Allocator,
{
    let layout = Layout::array::<T>(count).map_err(|_| AllocError)?;
    let slice = alloc.allocate(layout)?;
    let ptr = slice.cast::<T>().as_ptr();
    let mut vec = unsafe { Vec::from_raw_parts_in(ptr, 0, count, alloc) };
    vec.extend(core::iter::repeat_n(value, count));
    Ok(vec)
}

/// See [`BumpAllocator::alloc_slice_move`]
#[cfg(feature = "panic-on-alloc")]
pub fn vec_from_owned_slice_in<T, A>(slice: impl OwnedSlice<Item = T>, alloc: A) -> Vec<T, A>
where
    A: BumpAllocator,
{
    unsafe {
        let mut owned_slice = slice.into_take_owned_slice();
        let owned_slice_ref = owned_slice.owned_slice_ref();

        let len = owned_slice_ref.len();
        let src = owned_slice_ref.as_ptr();
        let dst = panic_on_error(alloc.spec_alloc(Layout::for_value(owned_slice_ref)))
            .as_ptr()
            .cast();
        ptr::copy_nonoverlapping(src, dst, len);

        owned_slice.take_owned_slice();
        Vec::from_raw_parts_in(dst, len, len, alloc)
    }
}

/// See [`BumpAllocator::try_alloc_slice_move`]
pub fn vec_try_from_owned_slice_in<T, A>(slice: impl OwnedSlice<Item = T>, alloc: A) -> Result<Vec<T, A>, AllocError>
where
    A: BumpAllocator,
{
    unsafe {
        let mut owned_slice = slice.into_take_owned_slice();
        let owned_slice_ref = owned_slice.owned_slice_ref();

        let len = owned_slice_ref.len();
        let src = owned_slice_ref.as_ptr();
        let dst = alloc.spec_alloc(Layout::for_value(owned_slice_ref))?.as_ptr().cast();
        ptr::copy_nonoverlapping(src, dst, len);

        owned_slice.take_owned_slice();
        Ok(Vec::from_raw_parts_in(dst, len, len, alloc))
    }
}
