use core::{
    alloc::{AllocError, Allocator, Layout},
    marker::PhantomData,
    mem::{MaybeUninit, forget},
    ptr::{self, NonNull},
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    BoxU, SizedTypeProperties, Unmanaged,
    boxed_slice_initializer::BoxedSliceInitializer,
    error_behavior::ErrorBehavior,
    owned_slice::{OwnedSlice, TakeOwnedSlice as _},
    panic_on_error,
    traits::{BumpAllocator, BumpAllocatorScope},
};

macro_rules! assert_in_place_mappable {
    ($src:ty, $dst:ty) => {
        let _assert = AssertInPlaceMappable::<$src, $dst>::ASSERT;
    };
}

struct AssertInPlaceMappable<Src, Dst>(PhantomData<(Src, Dst)>);

impl<Src, Dst> AssertInPlaceMappable<Src, Dst> {
    const ASSERT: () = assert!(
        Dst::IS_ZST || (Src::ALIGN >= Dst::ALIGN && Src::SIZE >= Dst::SIZE),
        "`map_in_place` only compiles when `U`s size and alignment is less or equal to `T`'s or if `U` has a size of 0"
    );
}

/// Returns a boxed slice of the same length as `self`, with function `f` applied to each element in order.
///
/// This function only compiles when `U`s size and alignment is less or equal to `T`'s or if `U` has a size of 0.
///
/// # Examples
/// Mapping to a type with an equal alignment and size:
/// ```
/// # use bump_scope::{Bump};
/// # use core::num::NonZero;
/// # let bump: Bump = Bump::new();
/// let a = bump.alloc_slice_copy(&[0, 1, 2]).into_boxu();
/// let b = a.map_in_place(NonZero::new);
/// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2)]");
/// ```
///
/// Mapping to a type with a smaller alignment and size:
/// ```
/// # use bump_scope::{Bump, BoxU};
/// # let bump: Bump = Bump::new();
/// let a: BoxU<[u32]> = bump.alloc_slice_copy(&[0, 1, 2]).into_boxu();
/// let b: BoxU<[u16]> = a.map_in_place(|i| i as u16);
/// assert_eq!(b, [0, 1, 2]);
/// ```
///
/// Mapping to a type with a greater alignment won't compile:
/// ```compile_fail,E0080
/// # use bump_scope::{Bump, BoxU};
/// # let bump: Bump = Bump::new();
/// let a: BoxU<[[u8; 4]]> = bump.alloc_slice_copy(&[[0, 1, 2, 3]]);
/// let b: BoxU<[u32]> = a.map_in_place(u32::from_le_bytes);
/// # _ = b;
/// ```
///
/// Mapping to a type with a greater size won't compile:
/// ```compile_fail,E0080
/// # use bump_scope::{Bump, BoxU};
/// # let bump: Bump = Bump::new();
/// let a: BoxU<[u32]> = bump.alloc_slice_copy(&[42]);
/// let b: BoxU<[[u32; 2]]> = a.map_in_place(|i| [i; 2]);
/// # _ = b;
/// ```
pub fn box_map_in_place<T, U, A>(boxed: Box<[T], A>, mut f: impl FnMut(T) -> U) -> Box<[U], A>
where
    A: Allocator,
{
    assert_in_place_mappable!(T, U);

    if U::IS_ZST {
        return unsafe {
            let len = boxed.len();
            let (_, alloc) = Box::into_raw_with_allocator(boxed);
            let src: Box<[T], &Unmanaged> = Box::from_raw_in(dangling_mut_slice(len), &Unmanaged);
            let dst: Box<[MaybeUninit<U>], A> = Box::from_raw_in(dangling_mut_slice(len), alloc);
            box_init_fill_iter(dst, src.into_iter().map(f))
        };
    }

    struct DropGuard<T, U> {
        ptr: *mut T,
        end: *mut T,
        src: *mut T,
        dst: *mut U,
    }

    impl<T, U> Drop for DropGuard<T, U> {
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
            }
        }
    }

    let (slice, alloc) = Box::into_raw_with_allocator(boxed);
    let ptr = slice.cast::<T>();
    let len = slice.len();

    unsafe {
        let mut guard = DropGuard::<T, U> {
            ptr,
            end: ptr.add(len),
            src: ptr,
            dst: ptr.cast::<U>(),
        };

        while guard.src < guard.end {
            let src_value = guard.src.read();
            let dst_value = f(src_value);
            guard.dst.write(dst_value);
            guard.src = guard.src.add(1);
            guard.dst = guard.dst.add(1);
        }

        forget(guard);

        Box::from_raw_in(ptr::slice_from_raw_parts_mut(ptr.cast(), len), alloc)
    }
}

fn dangling_mut_slice<T>(len: usize) -> *mut [T] {
    ptr::slice_from_raw_parts_mut(ptr::dangling_mut(), len)
}

pub(crate) fn boxu<'a, T, A>(boxed: Box<T, A>) -> BoxU<'a, T>
where
    T: ?Sized,
    A: BumpAllocatorScope<'a>,
{
    box_replace_allocator(boxed, &Unmanaged).0
}

pub(crate) unsafe fn boxu_from_raw<'a, T: ?Sized>(ptr: *mut T) -> BoxU<'a, T> {
    unsafe { BoxU::from_raw_in(ptr, &Unmanaged) }
}

pub(crate) fn boxu_uninit_zst<'a, T>() -> BoxU<'a, MaybeUninit<T>> {
    assert!(T::IS_ZST);
    unsafe { boxu_from_raw(ptr::dangling_mut()) }
}

pub(crate) fn boxu_uninit_zst_slice<'a, T>(len: usize) -> BoxU<'a, [MaybeUninit<T>]> {
    assert!(T::IS_ZST);
    unsafe { boxu_from_raw(dangling_mut_slice(len)) }
}

pub(crate) unsafe fn box_from_utf8_unchecked<A: Allocator>(boxed: Box<[u8], A>) -> Box<str, A> {
    let (ptr, alloc) = Box::into_raw_with_allocator(boxed);
    unsafe { Box::from_raw_in(ptr as *mut str, alloc) }
}

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
pub fn vec_replace_allocator<'a, T, A, NewA>(boxed: Vec<T, A>, new_alloc: NewA) -> Vec<T, NewA>
where
    A: BumpAllocatorScope<'a>,
    NewA: BumpAllocatorScope<'a>,
{
    let (ptr, len, cap, _) = Vec::into_raw_parts_with_allocator(boxed);
    unsafe { Vec::from_raw_parts_in(ptr, len, cap, new_alloc) }
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
pub fn vec_append_owned_slice<T, A>(vec: &mut Vec<T, A>, slice: impl OwnedSlice)
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
