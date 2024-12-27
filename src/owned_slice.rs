mod drain;
mod extract_if;
mod into_iter;

pub use drain::Drain;
pub use extract_if::ExtractIf;
pub use into_iter::IntoIter;

use core::{mem::ManuallyDrop, ptr::NonNull};

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "alloc")]
use core::ptr;

use crate::{
    polyfill::{nonnull, transmute_value},
    BumpAllocator, BumpBox, BumpVec, FixedBumpVec, MutBumpVec, MutBumpVecRev,
};

/// Conversion to an [`OwnedSlice`].
///
/// Any implementor of `OwnedSlice` automatically implements this trait.
// FIXME: this naming is just plain wrong; of course boxed slices or arrays are owned slices
//        and don't need to be converted to such; the `OwnedSlice` trait conveys something different
//        like `Appendable`(?); this trait could be named `OwnedSlice` instead
pub trait IntoOwnedSlice {
    /// The type of the elements of the owned slice.
    type Item;

    /// Which kind of owned slice are we turning this into?
    type OwnedSlice: OwnedSlice<Item = Self::Item>;

    /// Creates an owned slice from a value.
    fn into_owned_slice(self) -> Self::OwnedSlice;
}

impl<T: OwnedSlice> IntoOwnedSlice for T {
    type Item = <T as OwnedSlice>::Item;
    type OwnedSlice = T;

    fn into_owned_slice(self) -> Self::OwnedSlice {
        self
    }
}

impl<'a, T, const N: usize> IntoOwnedSlice for BumpBox<'a, [T; N]> {
    type Item = T;

    type OwnedSlice = BumpBox<'a, [T]>;

    fn into_owned_slice(self) -> Self::OwnedSlice {
        self.into_unsized()
    }
}

#[cfg(feature = "alloc")]
impl<T, const N: usize> IntoOwnedSlice for Box<[T; N]> {
    type Item = T;

    type OwnedSlice = Vec<T>;

    fn into_owned_slice(self) -> Self::OwnedSlice {
        let boxed_slice: Box<[T]> = self;
        boxed_slice.into_owned_slice()
    }
}

#[cfg(feature = "alloc")]
impl<T> IntoOwnedSlice for Box<[T]> {
    type Item = T;

    type OwnedSlice = Vec<T>;

    fn into_owned_slice(self) -> Self::OwnedSlice {
        self.into()
    }
}

/// An owned slice, like a `Vec<T>`. This allows for efficient generic `append` implementations.
///
/// This trait can only be implemented for types that can empty their slice.
/// Types like `[T;N]` or `Box<[T]>` can not implement it but do implement [`IntoOwnedSlice`].
///
/// # Safety
///
/// - [`owned_slice_ptr`] must return a pointer to a valid slice of initialized values
/// - [`take_owned_slice`] will make the implementor relinquish its ownership over the elements of this slice, the caller is now responsible for dropping those elements
///   The elements must no longer be accessible via the implementor. (like <code>Vec::[set_len]\(0)</code>).
///   After calling this, `owned_slice_ptr` must return a slice pointer with a length of `0`.
///
/// For example this function must be sound:
/// ```
/// # extern crate alloc;
/// # use alloc::vec::Vec;
/// # use bump_scope::owned_slice::OwnedSlice;
/// fn append<T>(vec: &mut Vec<T>, mut to_append: impl OwnedSlice<Item = T>) {
///     let slice = to_append.owned_slice_ptr();
///     vec.reserve(slice.len());
///     
///     unsafe {
///         let src = slice.cast::<T>().as_ptr();
///         let dst = vec.as_mut_ptr().add(vec.len());
///         src.copy_to_nonoverlapping(dst, slice.len());
///
///         to_append.take_owned_slice();
///         vec.set_len(vec.len() + slice.len());
///     }
/// }
///
/// # use alloc::string::ToString;
/// # let mut vec = (0..3).map(|i| i.to_string()).collect::<Vec<_>>();
/// # let mut to_append = (3..10).map(|i| i.to_string()).collect::<Vec<_>>();
/// # append(&mut vec, &mut to_append);
/// # assert_eq!(to_append.len(), 0);
/// # assert_eq!(vec.len(), 10);
/// # assert_eq!(vec, ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"])
/// ```
///
/// [set_len]: alloc::vec::Vec::set_len
/// [`owned_slice_ptr`]: OwnedSlice::owned_slice_ptr
/// [`take_owned_slice`]: OwnedSlice::take_owned_slice
#[allow(clippy::len_without_is_empty)]
pub unsafe trait OwnedSlice {
    /// The element type of the slice.
    type Item;

    /// Returns the raw slice pointer.
    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]>;

    /// This makes the container forget all of its elements.
    /// (like <code>Vec::[set_len]\(0)</code>)
    ///
    /// The caller is now responsible for dropping the elements.
    ///
    /// [set_len]: alloc::vec::Vec::set_len
    fn take_owned_slice(&mut self);
}

unsafe impl<T: OwnedSlice + ?Sized> OwnedSlice for &mut T {
    type Item = T::Item;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        T::owned_slice_ptr(self)
    }

    fn take_owned_slice(&mut self) {
        T::take_owned_slice(self);
    }
}

unsafe impl<T> OwnedSlice for BumpBox<'_, [T]> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

unsafe impl<T> OwnedSlice for FixedBumpVec<'_, T> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

unsafe impl<T, A: BumpAllocator> OwnedSlice for BumpVec<T, A> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

unsafe impl<T, A> OwnedSlice for MutBumpVec<T, A> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

unsafe impl<T, A> OwnedSlice for MutBumpVecRev<T, A> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T> OwnedSlice for Vec<T> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        let slice = ptr::slice_from_raw_parts(self.as_ptr(), self.len());
        unsafe { NonNull::new_unchecked(slice as *mut [T]) }
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

/// The type returned from <code><[T; N]>::[into_owned_slice]</code>.
///
/// [into_owned_slice]: IntoOwnedSlice::into_owned_slice
pub struct ArrayOwnedSlice<T, const N: usize> {
    array: [ManuallyDrop<T>; N],
    taken: bool,
}

unsafe impl<T, const N: usize> OwnedSlice for ArrayOwnedSlice<T, N> {
    type Item = T;

    #[inline(always)]
    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        if self.taken {
            return nonnull::slice_from_raw_parts(NonNull::dangling(), 0);
        }

        unsafe {
            let ptr = NonNull::new_unchecked(self.array.as_ptr() as *mut T);
            nonnull::slice_from_raw_parts(ptr, self.array.len())
        }
    }

    #[inline(always)]
    fn take_owned_slice(&mut self) {
        self.taken = true;
    }
}

impl<T, const N: usize> IntoOwnedSlice for [T; N] {
    type Item = T;

    type OwnedSlice = ArrayOwnedSlice<T, N>;

    fn into_owned_slice(self) -> Self::OwnedSlice {
        ArrayOwnedSlice {
            array: unsafe { transmute_value(self) },
            taken: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Bump;

    use super::*;

    const _: () = {
        const fn is_dyn_compatible<T: OwnedSlice + ?Sized>() {}
        is_dyn_compatible::<dyn OwnedSlice<Item = i32>>();
        is_dyn_compatible::<&mut dyn OwnedSlice<Item = i32>>();
    };

    macro_rules! assert_implements {
        ($($ty:ty)*) => {
            const _: () = {
                type T = i32;
                const fn implements<S: IntoOwnedSlice + ?Sized>() {}
                $(implements::<$ty>();)*
            };
        };
    }

    assert_implements! {
        &mut dyn OwnedSlice<Item = T>

        [T; 3]
        BumpBox<[T; 3]>

        BumpBox<[T]>
        &mut BumpBox<[T]>
        FixedBumpVec<T>
        &mut FixedBumpVec<T>
        BumpVec<T, &Bump>
        &mut BumpVec<T, &Bump>
        MutBumpVec<T, &mut Bump>
        &mut MutBumpVec<T, &mut Bump>
        MutBumpVecRev<T, &mut Bump>
        &mut MutBumpVecRev<T, &mut Bump>
        BumpVec<T, Bump>
        &mut BumpVec<T, Bump>
        MutBumpVec<T, Bump>
        &mut MutBumpVec<T, Bump>
        MutBumpVecRev<T, Bump>
        &mut MutBumpVecRev<T, Bump>
    }

    #[cfg(feature = "alloc")]
    assert_implements! {
        Box<[T; 3]>

        Box<[T]>
        Vec<T>
        &mut Vec<T>
    }
}
