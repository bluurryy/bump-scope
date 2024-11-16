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

/// An owned slice, like a `Vec<T>`. This allows for efficient generic `append` implementations.
///
/// # Safety
///
/// `take_slice` returns a pointer to a valid slice of initialized values, with a length corresponding to the most recent call to `len`.
///
/// The slice remains valid as long as the implementor is only accessed through methods provided by this trait.  
/// Interaction outside this trait's API may invalidate the slice.
#[allow(clippy::missing_safety_doc)]
pub unsafe trait OwnedSlice<T> {
    /// Returns the length of the slice.
    fn len(&self) -> usize;
    /// Returns a pointer to a valid slice of initialized values, with a length corresponding to the most recent call to `len`.
    /// The caller is now responsible for dropping the values of this slice.
    fn take_slice(&mut self) -> NonNull<T>;
}

unsafe impl<T, S: OwnedSlice<T>> OwnedSlice<T> for &mut S {
    fn len(&self) -> usize {
        S::len(self)
    }

    fn take_slice(&mut self) -> NonNull<T> {
        S::take_slice(self)
    }
}

unsafe impl<T> OwnedSlice<T> for BumpBox<'_, [T]> {
    fn len(&self) -> usize {
        BumpBox::<[T]>::len(self)
    }

    fn take_slice(&mut self) -> NonNull<T> {
        unsafe {
            let ptr = self.as_non_null_ptr();
            self.set_len(0);
            ptr
        }
    }
}

unsafe impl<T> OwnedSlice<T> for FixedBumpVec<'_, T> {
    fn len(&self) -> usize {
        FixedBumpVec::len(self)
    }

    fn take_slice(&mut self) -> NonNull<T> {
        unsafe {
            let ptr = self.as_non_null_ptr();
            self.set_len(0);
            ptr
        }
    }
}

unsafe impl<T, A: BumpAllocator> OwnedSlice<T> for BumpVec<T, A> {
    fn len(&self) -> usize {
        BumpVec::len(self)
    }

    fn take_slice(&mut self) -> NonNull<T> {
        unsafe {
            let ptr = self.as_non_null_ptr();
            self.set_len(0);
            ptr
        }
    }
}

unsafe impl<T, A> OwnedSlice<T> for MutBumpVec<T, A> {
    fn len(&self) -> usize {
        MutBumpVec::len(self)
    }

    fn take_slice(&mut self) -> NonNull<T> {
        unsafe {
            let ptr = self.as_non_null_ptr();
            self.set_len(0);
            ptr
        }
    }
}

unsafe impl<T, A> OwnedSlice<T> for MutBumpVecRev<T, A> {
    fn len(&self) -> usize {
        MutBumpVecRev::len(self)
    }

    fn take_slice(&mut self) -> NonNull<T> {
        unsafe {
            let slice = self.as_non_null_ptr();
            self.set_len(0);
            slice
        }
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T, A: Allocator> OwnedSlice<T> for Vec<T, A> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn take_slice(&mut self) -> NonNull<T> {
        unsafe {
            let ptr = NonNull::new_unchecked(self.as_mut_ptr());
            self.set_len(0);
            ptr
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Bump;
    use allocator_api2::alloc::Global;

    use super::*;

    const _: () = {
        fn assertions() {
            type T = i32;
            fn implements<S: OwnedSlice<T>>() {}
            implements::<BumpBox<[T]>>();
            implements::<&mut BumpBox<[T]>>();
        }
    };

    macro_rules! assert_implements {
        ($($ty:ty)*) => {
            const _: () = {
                fn assertions() {
                    type T = i32;
                    fn implements<S: OwnedSlice<T>>() {}
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
