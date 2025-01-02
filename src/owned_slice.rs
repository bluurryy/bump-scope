mod drain;
mod extract_if;
mod into_iter;

pub use drain::Drain;
pub use extract_if::ExtractIf;
pub use into_iter::IntoIter;

use core::{array, mem, ptr::NonNull};

#[cfg(feature = "alloc")]
use core::mem::ManuallyDrop;

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};

use crate::{BumpAllocator, BumpBox, BumpVec, FixedBumpVec, MutBumpVec, MutBumpVecRev};

/// A type that owns a slice of elements.
///
/// The whole point of this trait is as a parameter of the `append` method available on this crate's vector types.
///
/// Any implementor of `TakeOwnedSlice` automatically implements this trait.
pub trait OwnedSlice {
    /// The type of an element of this owned slice.
    type Item;

    /// A type can convert into that implements `TakeOwnedSlice`.
    type Take: TakeOwnedSlice<Item = Self::Item>;

    /// Converts this type into one that implements `TakeOwnedSlice`.
    fn into_take_owned_slice(self) -> Self::Take;
}

// every `TakeOwnedSlice` automatically implements `OwnedSlice`
impl<T: TakeOwnedSlice> OwnedSlice for T {
    type Item = <T as TakeOwnedSlice>::Item;

    type Take = T;

    fn into_take_owned_slice(self) -> Self::Take {
        self
    }
}

impl<T, const N: usize> OwnedSlice for [T; N] {
    type Item = T;

    type Take = array::IntoIter<T, N>;

    fn into_take_owned_slice(self) -> Self::Take {
        self.into_iter()
    }
}

impl<'a, T, const N: usize> OwnedSlice for BumpBox<'a, [T; N]> {
    type Item = T;

    type Take = BumpBox<'a, [T]>;

    fn into_take_owned_slice(self) -> Self::Take {
        self.into_unsized()
    }
}

#[cfg(feature = "alloc")]
impl<T, const N: usize> OwnedSlice for Box<[T; N]> {
    type Item = T;

    type Take = Box<[T]>;

    fn into_take_owned_slice(self) -> Self::Take {
        self
    }
}

/// A type which owns a slice and can abandon its ownership over it.
///
/// This trait is used for the `append` method of this crate's vector types via the [`OwnedSlice`] trait.
///
/// This trait can only be implemented for types that can empty their slice.
/// Types like `[T; N]` or `Box<[T]>` can not implement it but can be converted into a type that does via the [`OwnedSlice`] trait.
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
/// # use bump_scope::owned_slice::TakeOwnedSlice;
/// fn append<T>(vec: &mut Vec<T>, mut to_append: impl TakeOwnedSlice<Item = T>) {
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
/// [`owned_slice_ptr`]: TakeOwnedSlice::owned_slice_ptr
/// [`take_owned_slice`]: TakeOwnedSlice::take_owned_slice
#[allow(clippy::len_without_is_empty)]
pub unsafe trait TakeOwnedSlice {
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

unsafe impl<T: TakeOwnedSlice + ?Sized> TakeOwnedSlice for &mut T {
    type Item = T::Item;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        T::owned_slice_ptr(self)
    }

    fn take_owned_slice(&mut self) {
        T::take_owned_slice(self);
    }
}

unsafe impl<T, const N: usize> TakeOwnedSlice for array::IntoIter<T, N> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        NonNull::from(self.as_slice())
    }

    fn take_owned_slice(&mut self) {
        self.for_each(mem::forget);
    }
}

unsafe impl<T> TakeOwnedSlice for BumpBox<'_, [T]> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

unsafe impl<T> TakeOwnedSlice for FixedBumpVec<'_, T> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

unsafe impl<T, A: BumpAllocator> TakeOwnedSlice for BumpVec<T, A> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

unsafe impl<T, A> TakeOwnedSlice for MutBumpVec<T, A> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

unsafe impl<T, A> TakeOwnedSlice for MutBumpVecRev<T, A> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        self.as_non_null_slice()
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T> TakeOwnedSlice for Box<[T]> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        NonNull::from(&**self)
    }

    fn take_owned_slice(&mut self) {
        // we must not drop the elements but we must deallocate the slice itself
        let ptr = Box::into_raw(mem::take(self));
        let forget_elements_box = unsafe { Box::<[ManuallyDrop<T>]>::from_raw(ptr as *mut [ManuallyDrop<T>]) };
        drop(forget_elements_box);
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T> TakeOwnedSlice for Vec<T> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        NonNull::from(&**self)
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

#[cfg(test)]
mod tests {
    use crate::Bump;

    use super::*;

    const _: () = {
        const fn is_dyn_compatible<T: TakeOwnedSlice + ?Sized>() {}
        is_dyn_compatible::<dyn TakeOwnedSlice<Item = i32>>();
        is_dyn_compatible::<&mut dyn TakeOwnedSlice<Item = i32>>();
    };

    macro_rules! assert_implements {
        ($($ty:ty)*) => {
            const _: () = {
                type T = i32;
                const fn implements<S: OwnedSlice + ?Sized>() {}
                $(implements::<$ty>();)*
            };
        };
    }

    assert_implements! {
        &mut dyn TakeOwnedSlice<Item = T>

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
        Box<[T]>
        Box<[T; 3]>

        Vec<T>
        &mut Vec<T>
    }
}
