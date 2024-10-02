use core::ptr::NonNull;

use crate::{bump_allocator::LifetimeMarker, polyfill::nonnull, BumpAllocator, BumpBox, FixedBumpVec};
use allocator_api2::vec::Vec;

/// Trait that provides a [`into_fixed_vec`](Self::into_fixed_vec) for all [`BumpAllocator`]s.
pub trait VecExt<'a, T> {
    /// Turns this `Vec<T>` into a `FixedBumpVec<T>`.
    fn into_fixed_vec(self) -> FixedBumpVec<'a, T>;
}

impl<'a, T, A> VecExt<'a, T> for Vec<T, A>
where
    A: BumpAllocator<Lifetime = LifetimeMarker<'a>>,
{
    fn into_fixed_vec(self) -> FixedBumpVec<'a, T> {
        let (ptr, len, cap) = self.into_raw_parts();

        unsafe {
            FixedBumpVec {
                initialized: BumpBox::from_raw(nonnull::slice_from_raw_parts(NonNull::new_unchecked(ptr), len)),
                capacity: cap,
            }
        }
    }
}

#[cfg(test)]
#[cfg(feature = "nightly-allocator-api")]
mod tests {
    use crate::*;

    #[test]
    #[allow(unused_mut)]
    fn simple() {
        let mut bump: Bump = Bump::new();

        let slice: &[i32] = {
            let mut vec = Vec::new_in(&bump);

            vec.push(1);
            vec.push(2);
            vec.push(3);

            vec.into_fixed_vec().into_slice()
        };

        assert_eq!(slice, [1, 2, 3]);
        dbg!(slice);

        // bump.reset();
        // dbg!(slice);
    }
}
