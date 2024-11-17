mod drain;
mod extract_if;
mod into_iter;

use core::ptr::NonNull;

pub use drain::Drain;
pub use extract_if::ExtractIf;
pub use into_iter::IntoIter;

use crate::{BumpAllocator, BumpBox, BumpVec, FixedBumpVec, MutBumpVec, MutBumpVecRev};

#[cfg(feature = "alloc")]
use allocator_api2::{alloc::Allocator, vec::Vec};

#[cfg(feature = "alloc")]
use core::ptr;

/// An owned slice, like a `Vec<T>`. This allows for efficient generic `append` implementations.
///
/// # Safety
///
/// `owned_slice` returns a pointer to a valid slice of initialized values.
///
/// The slice remains valid as long as the implementor is only accessed through methods provided by this trait.
/// Interaction outside this trait's API may invalidate the slice.
#[allow(clippy::len_without_is_empty)]
pub unsafe trait OwnedSlice {
    /// The element type of the slice.
    type Item;

    /// Returns the raw slice pointer.
    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]>;

    /// This makes the container forget all of its elements.
    /// It is equivalent to <code>[set_len]\(0)</code>.
    ///
    /// The caller is now responsible for dropping the elements.
    ///
    /// [set_len]: alloc::vec::Vec::set_len
    fn take_owned_slice(&mut self);
}

unsafe impl<T: OwnedSlice> OwnedSlice for &mut T {
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
unsafe impl<T, A: Allocator> OwnedSlice for Vec<T, A> {
    type Item = T;

    fn owned_slice_ptr(&self) -> NonNull<[Self::Item]> {
        let slice = ptr::slice_from_raw_parts(self.as_ptr(), self.len());
        unsafe { NonNull::new_unchecked(slice as *mut [T]) }
    }

    fn take_owned_slice(&mut self) {
        unsafe { self.set_len(0) }
    }
}

#[cfg(test)]
mod tests {
    use crate::Bump;
    use allocator_api2::alloc::Global;

    use super::*;

    macro_rules! assert_implements {
        ($($ty:ty)*) => {
            const _: () = {
                fn assertions() {
                    type T = i32;
                    fn implements<S: OwnedSlice>() {}
                    $(implements::<$ty>();)*
                }
            };
        };
    }

    assert_implements! {
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
        Vec<T, Global>
        &mut Vec<T, Global>
    }
}
