use crate::{BumpAllocator, BumpVec, FixedBumpVec, MutBumpAllocator, MutBumpVec, MutBumpVecRev};

macro_rules! impl_slice_eq {
    ([$($vars:tt)*] $lhs:ty, $rhs:ty $(where $ty:ty: $bound:ident)?) => {
        impl<T, U, $($vars)*> PartialEq<$rhs> for $lhs
        where
            T: PartialEq<U>,
            $($ty: $bound)?
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool { self[..] == other[..] }
            #[inline]
            fn ne(&self, other: &$rhs) -> bool { self[..] != other[..] }
        }
    }
}

impl_slice_eq! { [] FixedBumpVec<'_, T>, FixedBumpVec<'_, U> }
impl_slice_eq! { [] FixedBumpVec<'_, T>, [U] }
impl_slice_eq! { [] FixedBumpVec<'_, T>, &[U] }
impl_slice_eq! { [] FixedBumpVec<'_, T>, &mut [U] }
impl_slice_eq! { [] [T], FixedBumpVec<'_, U> }
impl_slice_eq! { [] &[T], FixedBumpVec<'_, U> }
impl_slice_eq! { [] &mut [T], FixedBumpVec<'_, U> }
impl_slice_eq! { [const N: usize] FixedBumpVec<'_, T>, [U; N] }
impl_slice_eq! { [const N: usize] FixedBumpVec<'_, T>, &[U; N] }
impl_slice_eq! { [const N: usize] FixedBumpVec<'_, T>, &mut [U; N] }

impl_slice_eq! { [A1: BumpAllocator, A2: BumpAllocator] BumpVec<T, A1>, BumpVec<U, A2> }
impl_slice_eq! { [A: BumpAllocator] BumpVec<T, A>, [U] }
impl_slice_eq! { [A: BumpAllocator] BumpVec<T, A>, &[U] }
impl_slice_eq! { [A: BumpAllocator] BumpVec<T, A>, &mut [U] }
impl_slice_eq! { [A: BumpAllocator] [T], BumpVec<U, A> }
impl_slice_eq! { [A: BumpAllocator] &[T], BumpVec<U, A> }
impl_slice_eq! { [A: BumpAllocator] &mut [T], BumpVec<U, A> }
impl_slice_eq! { [A: BumpAllocator, const N: usize] BumpVec<T, A>, [U; N] }
impl_slice_eq! { [A: BumpAllocator, const N: usize] BumpVec<T, A>, &[U; N] }
impl_slice_eq! { [A: BumpAllocator, const N: usize] BumpVec<T, A>, &mut [U; N] }

impl_slice_eq! { [A1: MutBumpAllocator, A2: MutBumpAllocator] MutBumpVec<T, A1>, MutBumpVec<U, A2> }
impl_slice_eq! { [A: MutBumpAllocator] MutBumpVec<T, A>, [U] }
impl_slice_eq! { [A: MutBumpAllocator] MutBumpVec<T, A>, &[U] }
impl_slice_eq! { [A: MutBumpAllocator] MutBumpVec<T, A>, &mut [U] }
impl_slice_eq! { [A: MutBumpAllocator] [T], MutBumpVec<U, A> }
impl_slice_eq! { [A: MutBumpAllocator] &[T], MutBumpVec<U, A> }
impl_slice_eq! { [A: MutBumpAllocator] &mut [T], MutBumpVec<U, A> }
impl_slice_eq! { [A: MutBumpAllocator, const N: usize] MutBumpVec<T, A>, [U; N] }
impl_slice_eq! { [A: MutBumpAllocator, const N: usize] MutBumpVec<T, A>, &[U; N] }
impl_slice_eq! { [A: MutBumpAllocator, const N: usize] MutBumpVec<T, A>, &mut [U; N] }

impl_slice_eq! { [A1: MutBumpAllocator, A2: MutBumpAllocator] MutBumpVecRev<T, A1>, MutBumpVecRev<U, A2> }
impl_slice_eq! { [A: MutBumpAllocator] MutBumpVecRev<T, A>, [U] }
impl_slice_eq! { [A: MutBumpAllocator] MutBumpVecRev<T, A>, &[U] }
impl_slice_eq! { [A: MutBumpAllocator] MutBumpVecRev<T, A>, &mut [U] }
impl_slice_eq! { [A: MutBumpAllocator] [T], MutBumpVecRev<U, A> }
impl_slice_eq! { [A: MutBumpAllocator] &[T], MutBumpVecRev<U, A> }
impl_slice_eq! { [A: MutBumpAllocator] &mut [T], MutBumpVecRev<U, A> }
impl_slice_eq! { [A: MutBumpAllocator, const N: usize] MutBumpVecRev<T, A>, [U; N] }
impl_slice_eq! { [A: MutBumpAllocator, const N: usize] MutBumpVecRev<T, A>, &[U; N] }
impl_slice_eq! { [A: MutBumpAllocator, const N: usize] MutBumpVecRev<T, A>, &mut [U; N] }
