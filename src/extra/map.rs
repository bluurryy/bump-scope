use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::ManuallyDrop,
    ptr::{self, NonNull},
};

#[cfg(feature = "panic-on-alloc")]
use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::{
    BumpAllocator, SizedTypeProperties,
    destructure::destructure,
    error_behavior::ErrorBehavior,
    extra::{imp_vec_extend, vec_remove_allocator_unchecked},
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// Maps the elements of a vector, reusing the allocation if possible.
///
/// # Panics
/// Panics if the allocation fails. An allocation only occurs when the alignment or size of `U` is greater than that of `T`.
///
/// # Examples
/// Mapping to a type with an equal alignment and size (no allocation):
/// ```
/// use core::num::NonZero;
/// use bump_scope::{Bump, BumpAllocator, extra};
///
/// let bump: Bump = Bump::new();
///
/// let a = bump.alloc_iter_exact([0, 1, 2, 3]).into_box().into_vec();
/// assert_eq!(bump.stats().allocated(), 4 * 4);
///
/// let b = extra::vec_map(a, NonZero::new);
/// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2), Some(3)]");
/// assert_eq!(bump.stats().allocated(), 4 * 4); // allocation was reused!
/// ```
///
/// Mapping to a type with a smaller alignment and size (no allocation, capacity may grow):
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra};
///
/// let bump: Bump = Bump::new();
///
/// let vec_a: Vec<u32, _> = bump.alloc_iter_exact([1, 2, 3, 4]).into_box().into_vec();
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
/// use bump_scope::{Bump, BumpAllocator, extra};
///
/// let bump: Bump = Bump::new();
///
/// let vec_a: Vec<u16, _> = bump.alloc_iter_exact([1, 2, 3, 4]).into_box().into_vec();
/// assert_eq!(vec_a.capacity(), 4);
/// assert_eq!(bump.stats().allocated(), 4 * 2);
///
/// let vec_b: Vec<u32, _> = extra::vec_map(vec_a, |i| i as u32);
/// assert_eq!(vec_b.capacity(), 4);
/// assert_eq!(bump.stats().allocated(), 4 * 2 + 4 * 4);
/// ```
#[inline]
#[cfg(feature = "panic-on-alloc")]
pub fn vec_map<T, U, A>(vec: Vec<T, A>, f: impl FnMut(T) -> U) -> Vec<U, A>
where
    A: BumpAllocator,
{
    panic_on_error(imp_vec_map(vec, f))
}

/// Maps the elements of a vector, reusing the allocation if possible.
///
/// # Errors
/// Errors if the allocation fails. An allocation only occurs when the alignment or size of `U` is greater than that of `T`.
///
/// # Examples
/// Mapping to a type with an equal alignment and size (no allocation):
/// ```
/// use core::num::NonZero;
/// use bump_scope::{Bump, BumpAllocator, extra};
///
/// let bump: Bump = Bump::new();
///
/// let a = bump.try_alloc_iter_exact([0, 1, 2, 3])?.into_box().into_vec();
/// assert_eq!(bump.stats().allocated(), 4 * 4);
///
/// let b = extra::vec_try_map(a, NonZero::new)?;
/// assert_eq!(format!("{b:?}"), "[None, Some(1), Some(2), Some(3)]");
/// assert_eq!(bump.stats().allocated(), 4 * 4); // allocation was reused!
/// # Ok::<(), ::core::alloc::AllocError>(())
/// ```
///
/// Mapping to a type with a smaller alignment and size (no allocation, capacity may grow):
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra};
///
/// let bump: Bump = Bump::new();
///
/// let vec_a: Vec<u32, _> = bump.try_alloc_iter_exact([1, 2, 3, 4])?.into_box().into_vec();
/// assert_eq!(vec_a.capacity(), 4);
/// assert_eq!(bump.stats().allocated(), 4 * 4);
///
/// let vec_b: Vec<u16, _> = extra::vec_try_map(vec_a, |i| i as u16)?;
/// assert_eq!(vec_b.capacity(), 8);
/// assert_eq!(bump.stats().allocated(), 4 * 4);
/// # Ok::<(), ::core::alloc::AllocError>(())
/// ```
///
/// Mapping to a type with a higher alignment or size will allocate:
/// ```
/// use bump_scope::{Bump, BumpAllocator, extra};
///
/// let bump: Bump = Bump::new();
///
/// let vec_a: Vec<u16, _> = bump.try_alloc_iter_exact([1, 2, 3, 4])?.into_box().into_vec();
/// assert_eq!(vec_a.capacity(), 4);
/// assert_eq!(bump.stats().allocated(), 4 * 2);
///
/// let vec_b: Vec<u32, _> = extra::vec_try_map(vec_a, |i| i as u32)?;
/// assert_eq!(vec_b.capacity(), 4);
/// assert_eq!(bump.stats().allocated(), 4 * 2 + 4 * 4);
/// # Ok::<(), ::core::alloc::AllocError>(())
/// ```
#[inline]
pub fn vec_try_map<T, U, A>(vec: Vec<T, A>, f: impl FnMut(T) -> U) -> Result<Vec<U, A>, AllocError>
where
    A: BumpAllocator,
{
    imp_vec_map(vec, f)
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
#[inline]
#[cfg(feature = "panic-on-alloc")]
pub fn box_slice_map<T, U, A>(boxed: Box<[T], A>, f: impl FnMut(T) -> U) -> Box<[U], A>
where
    A: BumpAllocator,
{
    vec_map(boxed.into_vec(), f).into_boxed_slice()
}

#[inline(always)]
fn imp_vec_map<E, T, U, A>(vec: Vec<T, A>, f: impl FnMut(T) -> U) -> Result<Vec<U, A>, E>
where
    E: ErrorBehavior,
    A: BumpAllocator,
{
    // fallback
    // FIXME: if align or size increases we could do a grow instead
    if T::IS_ZST || U::IS_ZST || T::ALIGN < U::ALIGN || T::SIZE < U::SIZE {
        return vec_map_fallback(vec, f);
    }

    Ok(vec_map_no_grow(vec, f))
}

fn vec_map_no_grow<T, U, A>(vec: Vec<T, A>, mut f: impl FnMut(T) -> U) -> Vec<U, A>
where
    A: BumpAllocator,
{
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

fn vec_map_fallback<E, T, U, A>(vec: Vec<T, A>, f: impl FnMut(T) -> U) -> Result<Vec<U, A>, E>
where
    E: ErrorBehavior,
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

    imp_vec_extend::<E, U, A, _>(&mut drop_guard.new, old.into_iter().map(f))?;
    Ok(drop_guard.finish())
}
