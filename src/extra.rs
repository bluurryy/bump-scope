use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::{ManuallyDrop, MaybeUninit},
    ptr::{self, NonNull},
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    BoxU, NoDrop, SizedTypeProperties, Unmanaged, VecU,
    boxed_slice_initializer::BoxedSliceInitializer,
    destructure::destructure,
    error_behavior::ErrorBehavior,
    owned_slice::{OwnedSlice, TakeOwnedSlice as _},
    panic_on_error,
    traits::{BumpAllocator, BumpAllocatorScope},
};

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
/// let a = extra::vec_from_iter_in([0, 1, 2], &bump);
/// assert_eq!(bump.stats().allocated(), 12);
///
/// let b = a.map(NonZero::new);
/// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
/// assert_eq!(bump.stats().allocated(), 12); // allocation was reused!
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
/// let vec_b: Vec<u32, _> = extra::vec_map(vec_a; |i| i as u32);
/// assert_eq!(vec_b.capacity(), 4);
/// assert_eq!(bump.stats().allocated(), 4 * 2 + 4 * 4);
/// ```
#[inline]
pub fn vec_map<'a, T, U, A>(vec: Vec<T, A>, mut f: impl FnMut(T) -> U) -> Vec<U, A>
where
    A: BumpAllocator,
{
    if !T::IS_ZST && !U::IS_ZST && T::ALIGN >= U::ALIGN && T::SIZE >= U::SIZE {
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
    } else {
        // fallback
        let (old, alloc) = unsafe { vec_remove_allocator_unchecked(vec) };
        let mut new = Vec::new_in(alloc);
        new.extend(old.into_iter().map(f));
        new
    }
}

/// Maps the elements of a boxed slice, reusing the allocation if possible.
///
/// # Examples
/// Mapping to a type with an equal alignment and size:
/// ```
/// use core::num::NonZero;
/// use bump_scope::{Bump, extra};
///
/// let bump: Bump = Bump::new();
///
/// let a = bump.alloc_slice_copy(&[0, 1, 2]).into_boxu();
/// assert_eq!(bump.stats().allocated(), 12);
///
/// let b = extra::box_slice_map(a, NonZero::new);
/// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
/// assert_eq!(bump.stats().allocated(), 12); // allocation was reused!
/// ```
///
/// Mapping to a type with a smaller alignment and size:
/// ```
/// use bump_scope::{Bump, BoxU, extra};
///
/// let bump: Bump = Bump::new();
///
/// let a: BoxU<[u32]> = bump.alloc_slice_copy(&[0, 1, 2]).into_boxu();
/// assert_eq!(bump.stats().allocated(), 12);   
///
/// let b: BoxU<[u16]> = extra::box_slice_map(a, |i| i as u16);
/// assert_eq!(b, [0, 1, 2]);
/// assert_eq!(bump.stats().allocated(), 6); // allocation was reused and shrunk!
/// ```
///
/// Mapping to a type with a greater alignment will allocate:
/// ```
/// use bump_scope::{Bump, BoxU, extra};
///
/// let bump: Bump = Bump::new();
///
/// let a: BoxU<[[u8; 4]]> = bump.alloc_slice_copy(&[[0, 1, 2, 3]]);
/// assert_eq!(bump.stats().allocated(), 4);
///
/// let b: BoxU<[u32]> = extra::box_slice_map(a, u32::from_le_bytes);
/// assert_eq!(b, 123);
/// assert_eq!(bump.stats().allocated(), 8); // new allocation
/// ```
///
/// Mapping to a type with a greater size will allocate:
/// ```
/// use bump_scope::{Bump, BoxU};
///
/// let bump: Bump = Bump::new();
///
/// let a: BoxU<[u32]> = bump.alloc_slice_copy(&[42]);
/// assert_eq!(bump.stats().allocated(), 4);
///
/// let b: BoxU<[[u32; 2]]> = a.map_in_place(|i| [i; 2]);
/// assert_eq!(b, [[42, 42]]);
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

pub(crate) unsafe fn boxu_from_raw<'a, T: ?Sized>(ptr: *mut T) -> BoxU<'a, T> {
    unsafe { BoxU::from_raw_in(ptr, Unmanaged::NEW) }
}

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

pub(crate) fn boxu_uninit_zst_slice<'a, T>(len: usize) -> BoxU<'a, [MaybeUninit<T>]> {
    assert!(T::IS_ZST);
    unsafe { boxu_from_raw(dangling_mut_slice(len)) }
}

pub(crate) unsafe fn box_from_utf8_unchecked<A: Allocator>(boxed: Box<[u8], A>) -> Box<str, A> {
    let (ptr, alloc) = Box::into_raw_with_allocator(boxed);
    unsafe { Box::from_raw_in(ptr as *mut str, alloc) }
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

/// Divides a boxed slice into two at an index.
///
/// The first will contain all indices from `[0, mid)` (excluding
/// the index `mid` itself) and the second will contain all
/// indices from `[mid, len)` (excluding the index `len` itself).
///
/// # Panics
///
/// Panics if `mid > len`.
///
/// # Examples
///
/// ```
/// # use bump_scope::Bump;
/// # let bump: Bump = Bump::new();
/// #
/// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
///
/// {
///    let (left, right) = v.split_at(0);
///    assert_eq!(left, []);
///    assert_eq!(right, [1, 2, 3, 4, 5, 6]);
/// }
///
/// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
///
/// {
///     let (left, right) = v.split_at(2);
///     assert_eq!(left, [1, 2]);
///     assert_eq!(right, [3, 4, 5, 6]);
/// }
///
/// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
///
/// {
///     let (left, right) = v.split_at(6);
///     assert_eq!(left, [1, 2, 3, 4, 5, 6]);
///     assert_eq!(right, []);
/// }
/// ```
#[inline]
#[must_use]
pub fn box_split_at<T, A>(boxed: Box<[T], A>, at: usize) -> (Box<[T], A>, Box<[T], A>)
where
    A: Allocator + Clone,
{
    #[cold]
    #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
    #[track_caller]
    fn assert_failed(at: usize, len: usize) -> ! {
        panic!("`at` split index (is {at}) should be <= len (is {len})");
    }

    let len = boxed.len();

    if at > len {
        assert_failed(at, len);
    }

    // SAFETY: `[ptr; mid]` and `[mid; len]` are inside `self`, which
    // fulfills the requirements of `split_at_unchecked`.
    unsafe { box_split_at_unchecked(boxed, at) }
}

/// Divides one slice into two at an index, without doing bounds checking.
///
/// The first will contain all indices from `[0, mid)` (excluding
/// the index `mid` itself) and the second will contain all
/// indices from `[mid, len)` (excluding the index `len` itself).
///
/// For a safe alternative see [`split_at`].
///
/// # Safety
///
/// Calling this method with an out-of-bounds index is *[undefined behavior]*
/// even if the resulting reference is not used. The caller has to ensure that
/// `0 <= mid <= self.len()`.
///
/// [`split_at`]: Self::split_at
/// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
///
/// # Examples
///
/// ```
/// # use bump_scope::Bump;
/// # let bump: Bump = Bump::new();
/// #
/// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
///
/// unsafe {
///    let (left, right) = v.split_at_unchecked(0);
///    assert_eq!(left, []);
///    assert_eq!(right, [1, 2, 3, 4, 5, 6]);
/// }
///
/// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
///
/// unsafe {
///     let (left, right) = v.split_at_unchecked(2);
///     assert_eq!(left, [1, 2]);
///     assert_eq!(right, [3, 4, 5, 6]);
/// }
///
/// let v = bump.alloc_slice_copy(&[1, 2, 3, 4, 5, 6]);
///
/// unsafe {
///     let (left, right) = v.split_at_unchecked(6);
///     assert_eq!(left, [1, 2, 3, 4, 5, 6]);
///     assert_eq!(right, []);
/// }
/// ```
#[inline]
#[must_use]
pub unsafe fn box_split_at_unchecked<T, A>(boxed: Box<[T], A>, mid: usize) -> (Box<[T], A>, Box<[T], A>)
where
    A: Allocator + Clone,
{
    unsafe {
        let alloc_clone = Box::allocator(&boxed).clone();
        let (ptr, alloc) = Box::into_raw_with_allocator(boxed);

        let len = ptr.len();
        let ptr = ptr.cast::<T>();

        debug_assert!(
            mid <= len,
            "slice::split_at_unchecked requires the index to be within the slice"
        );

        (
            Box::from_raw_in(ptr::slice_from_raw_parts_mut(ptr, mid), alloc),
            Box::from_raw_in(ptr::slice_from_raw_parts_mut(ptr.add(mid), len - mid), alloc_clone),
        )
    }
}

/// TODO
pub fn box_replace_allocator<'a, T, Old, New>(boxed: Box<T, Old>, new: New) -> (Box<T, New>, Old)
where
    T: ?Sized,
    Old: BumpAllocatorScope<'a>,
    New: BumpAllocatorScope<'a>,
{
    let (ptr, old) = Box::into_raw_with_allocator(boxed);
    (unsafe { Box::from_raw_in(ptr, new) }, old)
}

/// TODO
pub(crate) unsafe fn box_replace_allocator_unchecked<'a, T, Old, New>(boxed: Box<T, Old>, new: New) -> (Box<T, New>, Old)
where
    T: ?Sized,
    Old: BumpAllocator,
    New: BumpAllocator,
{
    let (ptr, old) = Box::into_raw_with_allocator(boxed);
    (unsafe { Box::from_raw_in(ptr, new) }, old)
}

/// TODO
pub fn box_remove_allocator<'a, T, A>(boxed: Box<T, A>) -> (BoxU<'a, T>, A)
where
    T: ?Sized,
    A: BumpAllocatorScope<'a>,
{
    box_replace_allocator(boxed, Unmanaged::NEW)
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
pub fn vec_replace_allocator<'a, T, Old, New>(boxed: Vec<T, Old>, new_alloc: New) -> (Vec<T, New>, Old)
where
    Old: BumpAllocatorScope<'a>,
    New: BumpAllocatorScope<'a>,
{
    unsafe { vec_replace_allocator_unchecked(boxed, new_alloc) }
}

/// TODO
pub unsafe fn vec_replace_allocator_unchecked<'a, T, Old, New>(boxed: Vec<T, Old>, new_alloc: New) -> (Vec<T, New>, Old)
where
    Old: BumpAllocator,
    New: BumpAllocator,
{
    let (ptr, len, cap, old) = Vec::into_raw_parts_with_allocator(boxed);
    (unsafe { Vec::from_raw_parts_in(ptr, len, cap, new_alloc) }, old)
}

/// TODO
pub fn vec_remove_allocator<'a, T, A>(vec: Vec<T, A>) -> (VecU<'a, T>, A)
where
    A: BumpAllocatorScope<'a>,
{
    vec_replace_allocator(vec, Unmanaged::NEW)
}

/// TODO
pub unsafe fn vec_remove_allocator_unchecked<'a, T, A>(vec: Vec<T, A>) -> (VecU<'a, T>, A)
where
    A: BumpAllocator,
{
    unsafe { vec_replace_allocator_unchecked(vec, Unmanaged::NEW) }
}

/// TODO
pub fn box_from_owned_slice_in<T, A>(slice: impl OwnedSlice<Item = T>, alloc: A) -> Box<[T], A>
where
    A: BumpAllocator,
{
    panic_on_error(gen_box_from_owned_slice_in(slice, alloc))
}

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
pub fn vec_from_owned_slice_in<T, A>(slice: impl OwnedSlice<Item = T>, alloc: A) -> Vec<T, A>
where
    A: BumpAllocator,
{
    box_from_owned_slice_in(slice, alloc).into()
}

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

/// TODO
#[cfg(feature = "bytemuck")]
pub mod bytemuck {
    use core::{alloc::Allocator, mem::MaybeUninit, ptr};

    use alloc::boxed::Box;

    /// TODO
    pub fn box_init_zeroed<T, A: Allocator>(boxed: Box<MaybeUninit<T>, A>) -> Box<T, A> {
        let (ptr, alloc) = Box::into_raw_with_allocator(boxed);
        let ptr = ptr.cast::<T>();

        unsafe {
            ptr.write_bytes(0, 1);
            Box::from_raw_in(ptr, alloc)
        }
    }

    /// TODO
    pub fn box_init_slice_zeroed<T, A: Allocator>(boxed: Box<[MaybeUninit<T>], A>) -> Box<[T], A> {
        let len = boxed.len();
        let (ptr, alloc) = Box::into_raw_with_allocator(boxed);
        let ptr = ptr.cast::<T>();

        unsafe {
            ptr.write_bytes(0, 1);
            Box::from_raw_in(ptr::from_raw_parts_mut(ptr, len), alloc)
        }
    }
}
