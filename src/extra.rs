mod replace_allocator;
mod split_off;

use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::{self, ManuallyDrop, MaybeUninit},
    ptr::{self, NonNull},
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    BoxU, NoDrop, SizedTypeProperties, Unmanaged, VecU,
    boxed_slice_initializer::BoxedSliceInitializer,
    destructure::destructure,
    error_behavior::ErrorBehavior,
    owned_slice::{OwnedSlice, TakeOwnedSlice as _},
    traits::{BumpAllocator, BumpAllocatorScope},
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

pub use replace_allocator::{detach_allocator, replace_allocator};
pub use split_off::{split_at, split_off};

/// Maps the elements of a vector, reusing the allocation if possible.
///
/// # Panics
/// Panics if the allocation fails. An allocation only occurs when the alignment or size of `U` is greater than that of `T`.
///
/// # Examples
/// Mapping to a type with an equal alignment and size (no allocation):
/// ```
/// use core::num::NonZero;
/// use bump_scope::{Bump, extra};
///
/// let bump: Bump = Bump::new();
///
/// let a = extra::vec_from_iter_in([0, 1, 2, 3], &bump);
/// assert_eq!(bump.stats().allocated(), 4 * 4);
///
/// let b = extra::vec_map(a, NonZero::new);
/// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2), Some(3)]");
/// assert_eq!(bump.stats().allocated(), 4 * 4); // allocation was reused!
/// ```
///
/// Mapping to a type with a smaller alignment and size (no allocation, capacity may grow):
/// ```
/// use bump_scope::{Bump, extra};
///
/// let bump: Bump = Bump::new();
///
/// let vec_a: Vec<u32, _> = extra::vec_from_iter_in([1, 2, 3, 4], &bump);
/// assert_eq!(vec_a.capacity(), 4);
/// assert_eq!(bump.stats().allocated(), 4 * 4);
///
/// let vec_b: Vec<u16, _> = extra::vec_map(vec_a, |i| i as u16);
/// assert_eq!(vec_b.capacity(), 8);
/// assert_eq!(bump.stats().allocated(), 4 * 4);
/// ```
///
/// Mapping to a type with a higher alignment or size will allocate:
/// ```
/// use bump_scope::{Bump, extra};
///
/// let bump: Bump = Bump::new();
///
/// let vec_a: Vec<u16, _> = extra::vec_from_iter_in([1, 2, 3, 4], &bump);
/// assert_eq!(vec_a.capacity(), 4);
/// assert_eq!(bump.stats().allocated(), 4 * 2);
///
/// let vec_b: Vec<u32, _> = extra::vec_map(vec_a, |i| i as u32);
/// assert_eq!(vec_b.capacity(), 4);
/// assert_eq!(bump.stats().allocated(), 4 * 2 + 4 * 4);
/// ```
#[inline]
pub fn vec_map<T, U, A>(vec: Vec<T, A>, mut f: impl FnMut(T) -> U) -> Vec<U, A>
where
    A: BumpAllocator,
{
    // fallback
    // FIXME: if align or size increases we could do a grow instead
    if T::IS_ZST || U::IS_ZST || T::ALIGN < U::ALIGN || T::SIZE < U::SIZE {
        return vec_map_fallback(vec, f);
    }

    struct DropGuard<T, U, A: Allocator> {
        ptr: *mut T,
        cap: usize,
        end: *mut T,
        src: *mut T,
        dst: *mut U,
        alloc: A,
    }

    impl<T, U, A: Allocator> DropGuard<T, U, A> {
        fn into_allocator(self) -> A {
            destructure!(let Self { alloc } = self);
            alloc
        }
    }

    impl<T, U, A: Allocator> Drop for DropGuard<T, U, A> {
        fn drop(&mut self) {
            unsafe {
                // drop `T`s
                let drop_ptr = self.src.add(1);
                let drop_len = self.end.offset_from_unsigned(drop_ptr);
                ptr::slice_from_raw_parts_mut(drop_ptr, drop_len).drop_in_place();

                // drop `U`s
                let drop_ptr = self.ptr.cast::<U>();
                let drop_len = self.dst.offset_from_unsigned(drop_ptr);
                ptr::slice_from_raw_parts_mut(drop_ptr, drop_len).drop_in_place();

                if T::IS_ZST || self.cap == 0 {
                    return;
                }

                let layout = Layout::from_size_align_unchecked(self.cap * T::SIZE, T::ALIGN);
                self.alloc.deallocate(NonNull::new_unchecked(self.ptr.cast()), layout);
            }
        }
    }

    let (ptr, len, cap, alloc) = Vec::into_raw_parts_with_allocator(vec);

    unsafe {
        let mut guard = DropGuard::<T, U, A> {
            ptr,
            cap,
            end: ptr.add(len),
            src: ptr,
            dst: ptr.cast(),
            alloc,
        };

        while guard.src < guard.end {
            let src_value = guard.src.read();
            let dst_value = f(src_value);
            guard.dst.write(dst_value);
            guard.src = guard.src.add(1);
            guard.dst = guard.dst.add(1);
        }

        let alloc = guard.into_allocator();
        let new_cap = (cap * T::SIZE) / U::SIZE;

        Vec::from_raw_parts_in(ptr.cast(), len, new_cap, alloc)
    }
}

fn vec_map_fallback<T, U, A>(vec: Vec<T, A>, f: impl FnMut(T) -> U) -> Vec<U, A>
where
    A: Allocator,
{
    struct DropGuard<T, U, A: Allocator> {
        new: ManuallyDrop<Vec<U, A>>,
        old_ptr: *mut T,
        old_cap: usize,
    }

    impl<T, U, A: Allocator> DropGuard<T, U, A> {
        #[inline(always)]
        fn finish(self) -> Vec<U, A> {
            unsafe { self.deallocate_old() };
            destructure!(let Self { new } = self);
            ManuallyDrop::into_inner(new)
        }

        #[inline(always)]
        unsafe fn deallocate_new(&mut self) {
            unsafe { ManuallyDrop::drop(&mut self.new) };
        }

        #[inline(always)]
        unsafe fn deallocate_old(&self) {
            unsafe {
                let old_ptr = NonNull::new_unchecked(self.old_ptr).cast();
                let old_layout = Layout::from_size_align_unchecked(T::SIZE * self.old_cap, T::ALIGN);
                self.new.allocator().deallocate(old_ptr, old_layout);
            }
        }
    }

    impl<T, U, A: Allocator> Drop for DropGuard<T, U, A> {
        fn drop(&mut self) {
            // When a panic occurs we clean up as much as we can.
            unsafe { self.deallocate_new() };
            unsafe { self.deallocate_old() };
        }
    }

    // We avoid a `Clone` bound by taking the allocator from the old vec.
    // That means that it won't get automatically deallocated though, so we do it ourselves.
    //
    // NB: Deallocating the old vector only frees memory if the length is 0 or U is a ZST,
    //     otherwise the old vector is not the last allocation. So usually it doesn't do anything.
    let (mut old, alloc) = unsafe { vec_remove_allocator_unchecked(vec) };

    let mut drop_guard = DropGuard {
        new: ManuallyDrop::new(Vec::new_in(alloc)),
        old_ptr: old.as_mut_ptr(),
        old_cap: old.capacity(),
    };

    drop_guard.new.extend(old.into_iter().map(f));
    drop_guard.finish()
}

/// Maps the elements of a boxed slice, reusing the allocation if possible.
///
/// # Examples
/// Mapping to a type with an equal alignment and size:
/// ```
/// use core::num::NonZero;
/// use bump_scope::{Bump, BumpAllocator, extra};
/// let bump: Bump = Bump::new();
///
/// let a = bump.alloc_slice_copy(&[0, 1, 2]).into_box();
/// assert_eq!(bump.stats().allocated(), 12);
///
/// let b = extra::box_slice_map(a, NonZero::new);
/// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
/// assert_eq!(bump.stats().allocated(), 12); // allocation was reused!
/// ```
///
/// Mapping to a type with a smaller alignment and size:
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra};
/// let bump: Bump = Bump::new();
///
/// let a: Box<[u32], _> = bump.alloc_slice_copy(&[0, 1, 2]).into_box();
/// assert_eq!(bump.stats().allocated(), 12);   
///
/// let b: Box<[u16], _> = extra::box_slice_map(a, |i| i as u16);
/// assert_eq!(&*b, [0, 1, 2]);
/// assert_eq!(bump.stats().allocated(), 6); // allocation was reused and shrunk!
/// ```
///
/// Mapping to a type with a greater alignment will allocate:
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra};
/// let bump: Bump = Bump::new();
///
/// let a: Box<[[u8; 4]], _> = bump.alloc_slice_copy(&[[0, 1, 2, 3]]).into_box();
/// assert_eq!(bump.stats().allocated(), 4);
///
/// let b: Box<[u32], _> = extra::box_slice_map(a, u32::from_le_bytes);
/// assert_eq!(b[0], 50462976);
/// assert_eq!(bump.stats().allocated(), 8); // new allocation
/// ```
///
/// Mapping to a type with a greater size will allocate:
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra};
/// let bump: Bump = Bump::new();
///
/// let a: Box<[u32], _> = bump.alloc_slice_copy(&[42]).into_box();
/// assert_eq!(bump.stats().allocated(), 4);
///
/// let b: Box<[[u32; 2]], _> = extra::box_slice_map(a, |i| [i; 2]);
/// assert_eq!(&*b, [[42, 42]]);
/// assert_eq!(bump.stats().allocated(), 12); // new allocation
/// ```
pub fn box_slice_map<T, U, A>(boxed: Box<[T], A>, f: impl FnMut(T) -> U) -> Box<[U], A>
where
    A: BumpAllocator,
{
    vec_map(boxed.into_vec(), f).into_boxed_slice()
}

fn dangling_mut_slice<T>(len: usize) -> *mut [T] {
    ptr::slice_from_raw_parts_mut(ptr::dangling_mut(), len)
}

#[expect(unused)]
pub(crate) unsafe fn boxu_from_raw<'a, T: ?Sized>(ptr: *mut T) -> BoxU<'a, T> {
    unsafe { BoxU::from_raw_in(ptr, Unmanaged::VALUE) }
}

#[expect(unused)]
pub(crate) fn boxu_uninit_zst<'a, T>() -> BoxU<'a, MaybeUninit<T>> {
    assert!(T::IS_ZST);
    unsafe { boxu_from_raw(ptr::dangling_mut()) }
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

#[expect(unused)]
pub(crate) fn boxu_uninit_zst_slice<'a, T>(len: usize) -> BoxU<'a, [MaybeUninit<T>]> {
    assert!(T::IS_ZST);
    unsafe { boxu_from_raw(dangling_mut_slice(len)) }
}

pub(crate) unsafe fn box_from_utf8_unchecked<A: Allocator>(boxed: Box<[u8], A>) -> Box<str, A> {
    let (ptr, alloc) = Box::into_raw_with_allocator(boxed);
    unsafe { Box::from_raw_in(ptr as *mut str, alloc) }
}

pub(crate) fn ptr_from_utf8(ptr: *mut [u8]) -> *mut str {
    ptr as *mut str
}

/// TODO
pub fn box_leak<'a, T: ?Sized, A: BumpAllocatorScope<'a>>(boxed: Box<T, A>) -> &'a mut T {
    // We can't use `Box::leak` here, since that requires a 'static Allocator.
    // SAFETY: The `BumpAllocatorScope<'a>` trait guarantees that allocations live for `'a`.
    unsafe { &mut *Box::into_raw_with_allocator(boxed).0 }
}

/// TODO
pub fn box_init<T, A: Allocator>(boxed: Box<MaybeUninit<T>, A>, value: T) -> Box<T, A> {
    Box::<MaybeUninit<T>, A>::write(boxed, value)
}

/// TODO
///
/// # Panics
/// TODO
pub fn box_init_move<T, A>(mut boxed: Box<[MaybeUninit<T>], A>, owned_slice: impl OwnedSlice<Item = T>) -> Box<[T], A>
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

/// TODO
///
/// # Panics
/// TODO
pub fn box_init_clone<T, A>(boxed: Box<[MaybeUninit<T>], A>, slice: &[T]) -> Box<[T], A>
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

/// TODO
///
/// # Panics
/// TODO
pub fn box_init_copy<T, A>(mut boxed: Box<[MaybeUninit<T>], A>, slice: &[T]) -> Box<[T], A>
where
    T: Copy,
    A: Allocator,
{
    boxed.copy_from_slice(as_uninit_slice(slice));
    unsafe { boxed.assume_init() }
}

#[inline(always)]
fn as_uninit_slice<T>(slice: &[T]) -> &[MaybeUninit<T>] {
    unsafe { &*(ptr::from_ref(slice) as *const [MaybeUninit<T>]) }
}

/// TODO
pub fn box_init_fill<T, A>(boxed: Box<[MaybeUninit<T>], A>, value: T) -> Box<[T], A>
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

/// TODO
pub fn box_init_fill_with<T, A>(boxed: Box<[MaybeUninit<T>], A>, mut f: impl FnMut() -> T) -> Box<[T], A>
where
    A: Allocator,
{
    let mut initializer = BoxedSliceInitializer::new(boxed);

    while !initializer.is_full() {
        initializer.push_with(&mut f);
    }

    initializer.into_init()
}

/// TODO
pub fn box_init_fill_iter<T, A>(boxed: Box<[MaybeUninit<T>], A>, iter: impl IntoIterator<Item = T>) -> Box<[T], A>
where
    A: Allocator,
{
    #[cold]
    #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
    #[track_caller]
    fn iter_ran_out() -> ! {
        panic!("iterator ran out of items to fill the slice with");
    }

    let mut iter = iter.into_iter();
    let mut initializer = BoxedSliceInitializer::new(boxed);

    while !initializer.is_full() {
        match iter.next() {
            Some(item) => {
                initializer.push(item);
            }
            None => iter_ran_out(),
        }
    }

    initializer.into_init()
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

/// TODO
pub fn vec_from_iter_in<T, A>(iter: impl IntoIterator<Item = T>, alloc: A) -> Vec<T, A>
where
    A: Allocator,
{
    let mut vec = Vec::new_in(alloc);
    vec.extend(iter);
    vec
}

/// TODO
pub fn vec_from_iter_exact_in<T, A>(iter: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>, alloc: A) -> Vec<T, A>
where
    A: Allocator,
{
    panic_on_error(imp_vec_from_iter_exact_in(iter, alloc))
}

/// TODO
pub(crate) fn imp_vec_from_iter_exact_in<E, T, A>(
    iter: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    alloc: A,
) -> Result<Vec<T, A>, E>
where
    E: ErrorBehavior,
    A: Allocator,
{
    let mut iter = iter.into_iter();
    let len = iter.len();

    let mut vec = E::vec_with_capacity_in::<T, A>(len, alloc)?;

    while vec.len() < vec.capacity() {
        match iter.next() {
            // SAFETY: we checked above that `len < capacity`, so there is space
            Some(val) => unsafe { vec_push_unchecked(&mut vec, val) },
            None => break,
        }
    }

    Ok(vec)
}

unsafe fn vec_push_unchecked<T, A>(vec: &mut Vec<T, A>, val: T)
where
    A: Allocator,
{
    let len = vec.len();

    unsafe {
        vec.as_mut_ptr().add(len).write(val);
        vec.set_len(len + 1);
    }
}

/// TODO
///
/// # Safety
/// TODO
pub(crate) unsafe fn vec_replace_allocator_unchecked<T, Old, New>(boxed: Vec<T, Old>, new_alloc: New) -> (Vec<T, New>, Old)
where
    Old: Allocator,
    New: Allocator,
{
    let (ptr, len, cap, old) = Vec::into_raw_parts_with_allocator(boxed);
    (unsafe { Vec::from_raw_parts_in(ptr, len, cap, new_alloc) }, old)
}
/// TODO
///
/// # Safety
/// TODO
pub unsafe fn vec_remove_allocator_unchecked<'a, T, A>(vec: Vec<T, A>) -> (VecU<'a, T>, A)
where
    A: Allocator,
{
    unsafe { vec_replace_allocator_unchecked(vec, Unmanaged::VALUE) }
}

/// TODO
///
/// # Safety
/// TODO
pub(crate) unsafe fn box_replace_allocator_unchecked<T, Old, New>(boxed: Box<T, Old>, new_alloc: New) -> (Box<T, New>, Old)
where
    T: ?Sized,
    Old: Allocator,
    New: Allocator,
{
    let (ptr, old) = Box::into_raw_with_allocator(boxed);
    (unsafe { Box::from_raw_in(ptr, new_alloc) }, old)
}
/// TODO
///
/// # Safety
/// TODO
pub unsafe fn box_remove_allocator_unchecked<'a, T, A>(vec: Box<T, A>) -> (BoxU<'a, T>, A)
where
    T: ?Sized,
    A: Allocator,
{
    unsafe { box_replace_allocator_unchecked(vec, Unmanaged::VALUE) }
}

/// TODO
///
/// # Panics
/// TODO
#[cfg(feature = "panic-on-alloc")]
pub fn box_from_owned_slice_in<T, A>(slice: impl OwnedSlice<Item = T>, alloc: A) -> Box<[T], A>
where
    A: BumpAllocator,
{
    panic_on_error(gen_box_from_owned_slice_in(slice, alloc))
}

/// TODO
///
/// # Errors
/// TODO
pub fn try_box_from_owned_slice_in<T, A>(slice: impl OwnedSlice<Item = T>, alloc: A) -> Result<Box<[T], A>, AllocError>
where
    A: BumpAllocator,
{
    gen_box_from_owned_slice_in(slice, alloc)
}

/// TODO
pub(crate) fn gen_box_from_owned_slice_in<E, T, A>(slice: impl OwnedSlice<Item = T>, alloc: A) -> Result<Box<[T], A>, E>
where
    E: ErrorBehavior,
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
        Ok(Box::from_raw_in(ptr::slice_from_raw_parts_mut(dst, len), alloc))
    }
}

/// TODO
///
/// # Panics
/// TODO
#[cfg(feature = "panic-on-alloc")]
pub fn vec_from_owned_slice_in<T, A>(slice: impl OwnedSlice<Item = T>, alloc: A) -> Vec<T, A>
where
    A: BumpAllocator,
{
    box_from_owned_slice_in(slice, alloc).into()
}

/// TODO
///
/// # Errors
/// TODO
pub fn vec_try_from_owned_slice_in<T, A>(slice: impl OwnedSlice<Item = T>, alloc: A) -> Result<Vec<T, A>, AllocError>
where
    A: BumpAllocator,
{
    Ok(try_box_from_owned_slice_in(slice, alloc)?.into())
}

/// TODO
pub fn box_into_mut<'a, T, A>(boxed: Box<T, A>) -> &'a mut T
where
    T: NoDrop + ?Sized,
    A: BumpAllocatorScope<'a>,
{
    box_leak(boxed)
}

/// TODO
pub fn vec_into_slice<'a, T, A>(vec: Vec<T, A>) -> &'a mut [T]
where
    T: NoDrop,
    A: BumpAllocatorScope<'a>,
{
    box_into_mut(vec.into_boxed_slice())
}

/// TODO
pub fn vec_leak<'a, T, A>(vec: Vec<T, A>) -> &'a mut [T]
where
    A: BumpAllocatorScope<'a>,
{
    box_leak(vec.into_boxed_slice())
}

/// TODO
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

/// TODO
pub fn vec_to_box<T, A>(vec: Vec<T, A>) -> Box<[T], A>
where
    A: BumpAllocator,
{
    let (ptr, len, _, alloc) = Vec::into_raw_parts_with_allocator(vec);
    let slice = ptr::slice_from_raw_parts_mut(ptr, len);

    // SAFETY: `BumpAllocator` allows you shrink the memory block
    unsafe { Box::from_raw_in(slice, alloc) }
}

/// Extend the vector by `n` clones of value.
pub(crate) fn vec_extend<E, T, A, I>(vec: &mut Vec<T, A>, iterator: I) -> Result<(), E>
where
    E: ErrorBehavior,
    A: Allocator,
    I: IntoIterator<Item = T>,
{
    if !E::FALLIBLE {
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
            vec.try_reserve(lower.saturating_add(1)).map_err(|_| E::vec_reserve())?;
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
