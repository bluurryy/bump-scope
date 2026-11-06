//! Auxiliary functions.
//!
//! This module contains functions to:
//!
//! - Replace the bump allocator of vectors or boxes:
//!   - [`box_replace_allocator`]
//!   - [`vec_replace_allocator`]
//! - Split a vector or boxed slice in two without allocation:
//!   - [`vec_split_off`]
//!   - [`boxed_slice_split_off`]
//!   - [`boxed_str_split_off`]
//! - Initialize boxes:
//!   - [`box_init`]
//!   - [`boxed_slice_init_move`]
//!   - [`boxed_slice_init_copy`]
//!   - [`boxed_slice_init_clone`]
//!   - [`boxed_slice_init_fill`]
//!   - [`boxed_slice_init_fill_with`]
//! - Turn boxes into references:
//!   - [`box_into_ref`]
//!   - [`box_into_mut`]
//!   - [`box_leak`]
//! - Map a vector or boxed slice, potentially reusing the allocation:
//!   - [`vec_map`]
//!   - [`vec_try_map`]
//!   - [`boxed_slice_map`]
//!   - [`boxed_slice_try_map`]
//! - Append owned slices to vectors:
//!   - [`vec_append`]
//!   - [`vec_try_append`]
mod map;
mod replace_allocator;
mod split;

use core::{
    alloc::{AllocError, Allocator},
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    NoDrop, SizedTypeProperties, Unmanaged, VecU,
    containers::BoxedSliceInitializer,
    error_mode::ErrorMode,
    traits::{BumpAllocator, BumpAllocatorPrivate, BumpAllocatorScope},
};

pub use crate::owned_slice::{OwnedSlice, TakeOwnedSlice};
pub use map::{boxed_slice_try_map, vec_try_map};
pub use replace_allocator::{box_replace_allocator, vec_replace_allocator};
pub use split::{boxed_slice_split_off, boxed_str_split_off, vec_split_off};

#[cfg(feature = "panic-on-alloc")]
pub use map::{boxed_slice_map, vec_map};

#[doc(inline)]
pub use crate::__vec as vec;

fn dangling_mut_slice<T>(len: usize) -> *mut [T] {
    ptr::slice_from_raw_parts_mut(ptr::dangling_mut(), len)
}

pub(crate) fn box_uninit_zst_in<T, A>(alloc: A) -> Box<MaybeUninit<T>, A>
where
    A: Allocator,
{
    assert!(T::IS_ZST);
    unsafe { Box::from_raw_in(ptr::dangling_mut(), alloc) }
}

pub(crate) fn box_uninit_zst_slice_in<T, A>(len: usize, alloc: A) -> Box<[MaybeUninit<T>], A>
where
    A: Allocator,
{
    assert!(T::IS_ZST);
    unsafe { Box::from_raw_in(dangling_mut_slice(len), alloc) }
}

pub(crate) unsafe fn box_from_utf8_unchecked<A: Allocator>(boxed: Box<[u8], A>) -> Box<str, A> {
    let (ptr, alloc) = Box::into_raw_with_allocator(boxed);
    unsafe { Box::from_raw_in(ptr as *mut str, alloc) }
}

pub(crate) fn ptr_from_utf8(ptr: *mut [u8]) -> *mut str {
    ptr as *mut str
}

/// Consumes and leaks the `Box`, returning a mutable reference, `&'a mut T`.
///
/// Unlike [`Box::leak`] this function returns a mutable reference with the lifetime of the bump allocator scope.
pub fn box_leak<'a, T: ?Sized, A: BumpAllocatorScope<'a>>(boxed: Box<T, A>) -> &'a mut T {
    // We can't use `Box::leak` here, since that requires a 'static Allocator.
    // SAFETY: The `BumpAllocatorScope<'a>` trait guarantees that allocations live for `'a`.
    unsafe { &mut *Box::into_raw_with_allocator(boxed).0 }
}

/// Initializes a `Box<MaybeUninit<T>>`, returning `Box<T>`.
pub fn box_init<T, A: Allocator>(boxed: Box<MaybeUninit<T>, A>, value: T) -> Box<T, A> {
    Box::<MaybeUninit<T>, A>::write(boxed, value)
}

/// Initializes a boxed slice by moving elements from `slice`.
///
/// The length of of `boxed` and `slice` must be the same.
///
/// # Panics
/// This function will panic if the two slices have different lengths.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::boxed_slice_init_move};
/// let bump: Bump = Bump::new();
///
/// let uninit = bump.try_alloc_uninit_slice(3)?.into_box();
///
/// let vec = vec![
///     String::from("foo"),
///     String::from("bar"),
///     String::from("baz"),
/// ];
///
/// let init = boxed_slice_init_move(uninit, vec);
///
/// assert_eq!(&*init, &["foo", "bar", "baz"]);
/// # Ok::<(), core::alloc::AllocError>(())
/// ```
#[must_use]
pub fn boxed_slice_init_move<T, A>(
    mut boxed: Box<[MaybeUninit<T>], A>,
    owned_slice: impl OwnedSlice<Item = T>,
) -> Box<[T], A>
where
    T: Clone,
    A: Allocator,
{
    let mut owned_slice = owned_slice.into_take_owned_slice();
    let slice = NonNull::from(owned_slice.owned_slice_ref());

    assert_eq!(boxed.len(), slice.len());

    // SAFETY: we asserted that the lengths are the same
    unsafe {
        let src = slice.cast::<T>().as_ptr();
        let dst = boxed.as_mut_ptr().cast::<T>();
        ptr::copy_nonoverlapping(src, dst, slice.len());

        owned_slice.take_owned_slice();
        Box::<[MaybeUninit<T>], A>::assume_init(boxed)
    }
}

/// Initializes a boxed slice by cloning elements from `slice`.
///
/// The length of `boxed` and `slice` must be the same.
///
/// # Panics
/// This function will panic if the two slices have different lengths.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::boxed_slice_init_clone};;
/// let bump: Bump = Bump::new();
///
/// let uninit = bump.try_alloc_uninit_slice(3)?.into_box();
///
/// let slice = &[
///     String::from("foo"),
///     String::from("bar"),
///     String::from("baz"),
/// ];
///
/// let init = boxed_slice_init_clone(uninit, slice);
///
/// assert_eq!(&*init, &["foo", "bar", "baz"]);
/// # Ok::<(), core::alloc::AllocError>(())
/// ```
#[must_use]
pub fn boxed_slice_init_clone<T, A>(boxed: Box<[MaybeUninit<T>], A>, slice: &[T]) -> Box<[T], A>
where
    T: Clone,
    A: Allocator,
{
    assert_eq!(boxed.len(), slice.len());

    let mut initializer = BoxedSliceInitializer::new(boxed);

    // SAFETY: we asserted that the lengths are the same
    unsafe {
        for value in slice {
            initializer.push_unchecked(value.clone());
        }

        initializer.into_init_unchecked()
    }
}
/// Initializes a boxed slice by copying elements from `slice`.
///
/// The length of `boxed` and `slice` must be the same.
///
/// # Panics
///
/// This function will panic if the two slices have different lengths.
///
/// # Examples
///
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::boxed_slice_init_copy};
/// let bump: Bump = Bump::new();
///
/// let uninit = bump.try_alloc_uninit_slice(3)?.into_box();
///
/// let slice = &[
///     "foo",
///     "bar",
///     "baz",
/// ];
///
/// let init = boxed_slice_init_copy(uninit, slice);
///
/// assert_eq!(&*init, &["foo", "bar", "baz"]);
/// # Ok::<(), core::alloc::AllocError>(())
/// ```
#[must_use]
pub fn boxed_slice_init_copy<T, A>(mut boxed: Box<[MaybeUninit<T>], A>, slice: &[T]) -> Box<[T], A>
where
    T: Copy,
    A: Allocator,
{
    boxed.copy_from_slice(as_uninit_slice(slice));
    unsafe { boxed.assume_init() }
}

/// Initializes a boxed slice by filling it with clones of `value`.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::boxed_slice_init_fill};
/// let bump: Bump = Bump::new();
///
/// let uninit = bump.try_alloc_uninit_slice::<i32>(10)?.into_box();
/// let init = boxed_slice_init_fill(uninit, 1);
/// assert_eq!(&*init, &[1; 10]);
/// # Ok::<(), core::alloc::AllocError>(())
/// ```
#[must_use]
pub fn boxed_slice_init_fill<T, A>(boxed: Box<[MaybeUninit<T>], A>, value: T) -> Box<[T], A>
where
    T: Clone,
    A: Allocator,
{
    let len = boxed.len();

    if len == 0 {
        return unsafe { Box::<[MaybeUninit<T>], A>::assume_init(boxed) };
    }

    unsafe {
        let mut initializer = BoxedSliceInitializer::new(boxed);

        for _ in 0..(len - 1) {
            initializer.push_unchecked(value.clone());
        }

        initializer.push_unchecked(value);
        initializer.into_init_unchecked()
    }
}

/// Initializes a boxed slice by filling it with elements returned from a closure.
///
/// This method uses a closure to create new values. If you'd rather
/// [`Clone`] a given value, use [`boxed_slice_init_fill`]. If you want to use the [`Default`]
/// trait to generate values, you can pass [`Default::default`] as the
/// argument.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::boxed_slice_init_fill_with};
/// let bump: Bump = Bump::new();
///
/// let uninit = bump.try_alloc_uninit_slice::<i32>(10)?.into_box();
/// let init = boxed_slice_init_fill_with(uninit, Default::default);
/// assert_eq!(&*init, &[0i32; 10]);
/// # Ok::<(), core::alloc::AllocError>(())
/// ```
#[must_use]
pub fn boxed_slice_init_fill_with<T, A>(boxed: Box<[MaybeUninit<T>], A>, mut f: impl FnMut() -> T) -> Box<[T], A>
where
    A: Allocator,
{
    let mut initializer = BoxedSliceInitializer::new(boxed);

    while !initializer.is_full() {
        initializer.push_with(&mut f);
    }

    initializer.into_init()
}

/// Initializes a boxed slice by filling it with elements from an iterator.
///
/// If the iterator runs out of items before the slice is filled then the resulting `Box<[T], _>` will
/// be shorter than the `boxed` input.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::boxed_slice_init_fill_iter};
/// let bump: Bump = Bump::new();
///
/// let uninit = bump.try_alloc_uninit_slice::<char>(5)?.into_box();
/// let init = boxed_slice_init_fill_iter(uninit, ['a', 'b'].iter().copied().cycle());
/// assert_eq!(&*init, &['a', 'b', 'a', 'b', 'a']);
/// # Ok::<(), core::alloc::AllocError>(())
/// ```
#[must_use]
pub fn boxed_slice_init_fill_iter<T, A>(boxed: Box<[MaybeUninit<T>], A>, iter: impl IntoIterator<Item = T>) -> Box<[T], A>
where
    A: BumpAllocator,
{
    let len = boxed.len();
    let mut vec = vec_from_uninit_boxed_slice(boxed);
    vec.extend(iter.into_iter().take(len));
    vec_into_boxed_slice_without_shrink(vec)
}

#[inline(always)]
fn as_uninit_slice<T>(slice: &[T]) -> &[MaybeUninit<T>] {
    unsafe { &*(ptr::from_ref(slice) as *const [MaybeUninit<T>]) }
}

#[inline]
fn box_empty_slice_in<T, A>(alloc: A) -> Box<[T], A>
where
    A: Allocator,
{
    let ptr = ptr::slice_from_raw_parts_mut(ptr::dangling_mut::<T>(), 0);
    unsafe { Box::from_raw_in(ptr, alloc) }
}

#[inline]
fn box_empty_str_in<A>(alloc: A) -> Box<str, A>
where
    A: Allocator,
{
    let ptr = ptr_from_utf8(ptr::slice_from_raw_parts_mut(ptr::dangling_mut(), 0));
    unsafe { Box::from_raw_in(ptr, alloc) }
}

#[inline(always)]
unsafe fn vec_replace_buffer<T, A>(vec: &mut Vec<T, A>, ptr: *mut T, len: usize, cap: usize)
where
    A: Allocator,
{
    unsafe {
        let (_, _, _, alloc) = ptr::read(vec).into_raw_parts_with_allocator();
        let replacement = Vec::from_raw_parts_in(ptr, len, cap, alloc);
        mem::forget(mem::replace(vec, replacement));
    }
}

#[inline(always)]
unsafe fn box_set_ptr<T, A>(boxed: &mut Box<T, A>, ptr: *mut T)
where
    T: ?Sized,
    A: Allocator,
{
    unsafe {
        let (_, alloc) = Box::into_raw_with_allocator(ptr::read(boxed));
        let replacement = Box::from_raw_in(ptr, alloc);
        mem::forget(mem::replace(boxed, replacement));
    }
}

#[inline(always)]
unsafe fn box_set_len<T, A>(boxed: &mut Box<[T], A>, len: usize)
where
    A: Allocator,
{
    unsafe {
        let (old, alloc) = Box::into_raw_with_allocator(ptr::read(boxed));
        let new = ptr::slice_from_raw_parts_mut(old.cast::<T>(), len);
        let replacement = Box::from_raw_in(new, alloc);
        mem::forget(mem::replace(boxed, replacement));
    }
}

#[track_caller]
pub(crate) fn assert_char_boundary(str: &str, index: usize) {
    #[cold]
    #[track_caller]
    #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
    fn assert_failed() {
        panic!("index is not on a char boundary")
    }

    if !str.is_char_boundary(index) {
        assert_failed();
    }
}

pub(crate) unsafe fn vec_replace_allocator_unchecked<T, Old, New>(boxed: Vec<T, Old>, new_alloc: New) -> (Vec<T, New>, Old)
where
    Old: Allocator,
    New: Allocator,
{
    let (ptr, len, cap, old) = Vec::into_raw_parts_with_allocator(boxed);
    (unsafe { Vec::from_raw_parts_in(ptr, len, cap, new_alloc) }, old)
}

pub(crate) unsafe fn vec_remove_allocator_unchecked<'a, T, A>(vec: Vec<T, A>) -> (VecU<'a, T>, A)
where
    A: Allocator,
{
    unsafe { vec_replace_allocator_unchecked(vec, Unmanaged::VALUE) }
}

/// Returns this box as a `&T`.
/// This is only available for [`NoDrop`] types so you don't omit dropping a value for which it matters.
///
/// `!NoDrop` types can still be turned into references with [`box_leak`].
pub fn box_into_ref<'a, T, A>(boxed: Box<T, A>) -> &'a T
where
    T: NoDrop + ?Sized,
    A: BumpAllocatorScope<'a>,
{
    box_leak(boxed)
}

/// Returns this box as a `&mut T`.
/// This is only available for [`NoDrop`] types so you don't omit dropping a value for which it matters.
///
/// `!NoDrop` types can still be turned into references with [`box_leak`].
pub fn box_into_mut<'a, T, A>(boxed: Box<T, A>) -> &'a mut T
where
    T: NoDrop + ?Sized,
    A: BumpAllocatorScope<'a>,
{
    box_leak(boxed)
}

/// Moves all the elements of `other` into `self`, leaving `other` empty.
///
/// # Panics
/// Panics if the allocation fails.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::vec_append};
///
/// let bump: Bump = Bump::new();
///
/// let mut vec = Vec::<i32, _>::new_in(&bump);
///
/// // append by value
/// vec_append(&mut vec, [1, 2]);
/// vec_append(&mut vec, vec![3, 4]);
/// vec_append(&mut vec, bump.alloc_iter(5..=6).into_boxu());
///
/// // append by mutable reference
/// let mut other = vec![7, 8];
/// vec_append(&mut vec, &mut other);
///
/// assert_eq!(other, []);
/// assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8]);
/// ```
#[inline]
#[cfg(feature = "panic-on-alloc")]
pub fn vec_append<T, A>(vec: &mut Vec<T, A>, slice: impl OwnedSlice)
where
    A: Allocator,
{
    unsafe {
        let mut owned_slice = slice.into_take_owned_slice();

        let slice = NonNull::from(owned_slice.owned_slice_ref());
        vec.reserve(slice.len());

        let src = slice.cast::<T>().as_ptr();
        let dst = vec.as_mut_ptr().add(vec.len());
        ptr::copy_nonoverlapping(src, dst, slice.len());

        owned_slice.take_owned_slice();
        vec.set_len(vec.len() + slice.len());
    }
}

/// Moves all the elements of `other` into `self`, leaving `other` empty.
///
/// # Errors
/// Errors if the allocation fails.
///
/// # Examples
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra::vec_try_append};
///
/// let bump: Bump = Bump::new();
///
/// let mut vec = Vec::<i32, _>::new_in(&bump);
///
/// // append by value
/// vec_try_append(&mut vec, [1, 2]);
/// vec_try_append(&mut vec, vec![3, 4]);
/// vec_try_append(&mut vec, bump.try_alloc_iter(5..=6)?.into_boxu());
///
/// // append by mutable reference
/// let mut other = vec![7, 8];
/// vec_try_append(&mut vec, &mut other);
///
/// assert_eq!(other, []);
/// assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8]);
/// # Ok::<(), core::alloc::AllocError>(())
/// ```
#[inline]
pub fn vec_try_append<T, A>(vec: &mut Vec<T, A>, slice: impl OwnedSlice) -> Result<(), AllocError>
where
    A: Allocator,
{
    unsafe {
        let mut owned_slice = slice.into_take_owned_slice();

        let slice = NonNull::from(owned_slice.owned_slice_ref());
        vec.try_reserve(slice.len()).map_err(|_| AllocError)?;

        let src = slice.cast::<T>().as_ptr();
        let dst = vec.as_mut_ptr().add(vec.len());
        ptr::copy_nonoverlapping(src, dst, slice.len());

        owned_slice.take_owned_slice();
        vec.set_len(vec.len() + slice.len());
        Ok(())
    }
}

/// Turns a vector into a boxed slice without attempting to shrink the allocation.
pub fn vec_into_boxed_slice_without_shrink<T, A>(vec: Vec<T, A>) -> Box<[T], A>
where
    A: BumpAllocator,
{
    imp_vec_into_boxed_slice_without_shrink(vec)
}

pub(crate) fn imp_vec_into_boxed_slice_without_shrink<T, A>(vec: Vec<T, A>) -> Box<[T], A>
where
    A: BumpAllocatorPrivate,
{
    let (ptr, len, _, alloc) = Vec::into_raw_parts_with_allocator(vec);
    let slice = ptr::slice_from_raw_parts_mut(ptr, len);

    // SAFETY: `BumpAllocator` allows you shrink a memory block
    unsafe { Box::from_raw_in(slice, alloc) }
}

/// Turns an uninitialized boxed slice into an empty vector.
pub fn vec_from_uninit_boxed_slice<T, A>(boxed: Box<[MaybeUninit<T>], A>) -> Vec<T, A>
where
    A: Allocator,
{
    let (slice, alloc) = Box::into_raw_with_allocator(boxed);
    let len = slice.len();
    let ptr = slice.cast::<T>();
    unsafe { Vec::from_raw_parts_in(ptr, 0, len, alloc) }
}

/// Extend the vector by `n` clones of value.
pub(crate) fn imp_vec_extend<E, T, A, I>(vec: &mut Vec<T, A>, iterator: I) -> Result<(), E>
where
    E: ErrorMode,
    A: Allocator,
    I: IntoIterator<Item = T>,
{
    // If T is a ZST then this won't allocate, so it's fine to use this in the fallible api too.
    if !E::FALLIBLE || T::IS_ZST {
        vec.extend(iterator);
        return Ok(());
    }

    let mut iterator = iterator.into_iter();

    // This is the case for a general iterator.
    //
    // This function should be the moral equivalent of:
    //
    //      for item in iterator {
    //          self.push(item);
    //      }
    while let Some(element) = iterator.next() {
        let len = vec.len();
        if len == vec.capacity() {
            let (lower, _) = iterator.size_hint();
            E::vec_reserve(vec, lower.saturating_add(1))?;
        }
        unsafe {
            ptr::write(vec.as_mut_ptr().add(len), element);
            // Since next() executes user code which can panic we have to bump the length
            // after each step.
            // NB can't overflow since we would have had to alloc the address space
            vec.set_len(len + 1);
        }
    }

    Ok(())
}
