use ::bytemuck::Zeroable;

use crate::{
    BumpAllocatorExt, BumpVec, ErrorBehavior, FixedBumpVec, MutBumpAllocatorExt, MutBumpVec, MutBumpVecRev,
    alloc::AllocError,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

mod private {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub trait Sealed {}

    impl<T> Sealed for FixedBumpVec<'_, T> {}
    impl<T, A: BumpAllocatorExt> Sealed for BumpVec<T, A> {}
    impl<T, A: MutBumpAllocatorExt> Sealed for MutBumpVec<T, A> {}
    impl<T, A: MutBumpAllocatorExt> Sealed for MutBumpVecRev<T, A> {}
}

/// Extension trait for this crate's vector types.
pub trait VecExt: private::Sealed {
    /// The element type of this vector.
    type T;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bump_vec, bytemuck::VecExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        Self::T: Zeroable;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bump_vec, bytemuck::VecExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        Self::T: Zeroable;

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bump_vec, bytemuck::VecExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.resize_zeroed(5);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.resize_zeroed(2);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    fn resize_zeroed(&mut self, new_len: usize)
    where
        Self::T: Zeroable;

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bump_vec, bytemuck::VecExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
    /// vec.try_resize_zeroed(5)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
    /// vec.try_resize_zeroed(2)?;
    /// assert_eq!(vec, [1, 2]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
    where
        Self::T: Zeroable;
}

impl<T> VecExt for FixedBumpVec<'_, T> {
    type T = T;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, FixedBumpVec, bytemuck::VecExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        Self::T: Zeroable,
    {
        panic_on_error(self.generic_extend_zeroed(additional));
    }

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, FixedBumpVec, bytemuck::VecExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let mut vec = FixedBumpVec::try_with_capacity_in(5, &bump)?;
    /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        Self::T: Zeroable,
    {
        self.generic_extend_zeroed(additional)
    }

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Panics
    /// Panics if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, FixedBumpVec, bytemuck::VecExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.resize_zeroed(5);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = FixedBumpVec::with_capacity_in(5, &bump);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.resize_zeroed(2);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn resize_zeroed(&mut self, new_len: usize)
    where
        T: Zeroable,
    {
        panic_on_error(self.generic_resize_zeroed(new_len));
    }

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Errors
    /// Errors if the vector does not have enough capacity.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, FixedBumpVec, bytemuck::VecExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let mut vec = FixedBumpVec::try_with_capacity_in(5, &bump)?;
    /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
    /// vec.try_resize_zeroed(5)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = FixedBumpVec::try_with_capacity_in(5, &bump)?;
    /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
    /// vec.try_resize_zeroed(2)?;
    /// assert_eq!(vec, [1, 2]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
    where
        Self::T: Zeroable,
    {
        self.generic_resize_zeroed(new_len)
    }
}

impl<T, A: BumpAllocatorExt> VecExt for BumpVec<T, A> {
    type T = T;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bump_vec, bytemuck::VecExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        Self::T: Zeroable,
    {
        panic_on_error(self.generic_extend_zeroed(additional));
    }

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bump_vec, bytemuck::VecExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        Self::T: Zeroable,
    {
        self.generic_extend_zeroed(additional)
    }

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bump_vec, bytemuck::VecExt};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.resize_zeroed(5);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.resize_zeroed(2);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn resize_zeroed(&mut self, new_len: usize)
    where
        Self::T: Zeroable,
    {
        panic_on_error(self.generic_resize_zeroed(new_len));
    }

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, bump_vec, bytemuck::VecExt};
    /// let bump: Bump = Bump::try_new()?;
    ///
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
    /// vec.try_resize_zeroed(5)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
    /// vec.try_resize_zeroed(2)?;
    /// assert_eq!(vec, [1, 2]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
    where
        T: Zeroable,
    {
        self.generic_resize_zeroed(new_len)
    }
}

impl<T, A: MutBumpAllocatorExt> VecExt for MutBumpVec<T, A> {
    type T = T;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, mut_bump_vec, bytemuck::VecExt};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        Self::T: Zeroable,
    {
        panic_on_error(self.generic_extend_zeroed(additional));
    }

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, mut_bump_vec, bytemuck::VecExt};
    /// let mut bump: Bump = Bump::try_new()?;
    ///
    /// let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        Self::T: Zeroable,
    {
        self.generic_extend_zeroed(additional)
    }

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, mut_bump_vec, bytemuck::VecExt};
    /// let mut bump: Bump = Bump::new();
    ///
    /// {
    ///     let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    ///     vec.resize_zeroed(5);
    ///     assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// }
    ///
    /// {
    ///    let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    ///    vec.resize_zeroed(2);
    ///    assert_eq!(vec, [1, 2]);
    /// }
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn resize_zeroed(&mut self, new_len: usize)
    where
        Self::T: Zeroable,
    {
        panic_on_error(self.generic_resize_zeroed(new_len));
    }

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, mut_bump_vec, bytemuck::VecExt};
    /// let mut bump: Bump = Bump::try_new()?;
    ///
    /// {
    ///     let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
    ///     vec.try_resize_zeroed(5)?;
    ///     assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// }
    ///
    /// {
    ///    let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
    ///    vec.try_resize_zeroed(2)?;
    ///    assert_eq!(vec, [1, 2]);
    /// }
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
    where
        Self::T: Zeroable,
    {
        self.generic_resize_zeroed(new_len)
    }
}

impl<T, A: MutBumpAllocatorExt> VecExt for MutBumpVecRev<T, A> {
    type T = T;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, mut_bump_vec_rev, bytemuck::VecExt};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [0, 0, 1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        T: Zeroable,
    {
        panic_on_error(self.generic_extend_zeroed(additional));
    }

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, mut_bump_vec_rev, bytemuck::VecExt};
    /// let mut bump: Bump = Bump::try_new()?;
    ///
    /// let mut vec = mut_bump_vec_rev![try in &mut bump; 1, 2, 3]?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [0, 0, 1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        T: Zeroable,
    {
        self.generic_extend_zeroed(additional)
    }

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, mut_bump_vec_rev, bytemuck::VecExt};
    /// let mut bump: Bump = Bump::new();
    ///
    /// {
    ///     let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    ///     vec.resize_zeroed(5);
    ///     assert_eq!(vec, [0, 0, 1, 2, 3]);
    /// }
    ///
    /// {
    ///     let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    ///     vec.resize_zeroed(2);
    ///     assert_eq!(vec, [2, 3]);
    /// }
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn resize_zeroed(&mut self, new_len: usize)
    where
        T: Zeroable,
    {
        panic_on_error(self.generic_resize_zeroed(new_len));
    }

    /// Resizes this vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, mut_bump_vec_rev, bytemuck::VecExt};
    /// let mut bump: Bump = Bump::try_new()?;
    ///
    /// {
    ///     let mut vec = mut_bump_vec_rev![try in &mut bump; 1, 2, 3]?;
    ///     vec.try_resize_zeroed(5)?;
    ///     assert_eq!(vec, [0, 0, 1, 2, 3]);
    /// }
    ///
    /// {
    ///     let mut vec = mut_bump_vec_rev![try in &mut bump; 1, 2, 3]?;
    ///     vec.try_resize_zeroed(2)?;
    ///     assert_eq!(vec, [2, 3]);
    /// }
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
    where
        T: Zeroable,
    {
        self.generic_resize_zeroed(new_len)
    }
}

trait PrivateVecExt {
    fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E>;
    fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E>;
}

impl<T: Zeroable> PrivateVecExt for FixedBumpVec<'_, T> {
    #[inline]
    fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        self.generic_reserve(additional)?;

        unsafe {
            let ptr = self.as_mut_ptr();
            let len = self.len();

            ptr.add(len).write_bytes(0, additional);
            self.set_len(len + additional);
        }

        Ok(())
    }

    #[inline]
    fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E> {
        let len = self.len();

        if new_len > len {
            self.generic_extend_zeroed(new_len - len)
        } else {
            self.truncate(new_len);
            Ok(())
        }
    }
}

impl<T: Zeroable, A: BumpAllocatorExt> PrivateVecExt for BumpVec<T, A> {
    #[inline]
    fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        self.generic_reserve(additional)?;

        unsafe {
            let ptr = self.as_mut_ptr();
            let len = self.len();

            ptr.add(len).write_bytes(0, additional);
            self.set_len(len + additional);
        }

        Ok(())
    }

    #[inline]
    fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E> {
        let len = self.len();

        if new_len > len {
            self.generic_extend_zeroed(new_len - len)
        } else {
            self.truncate(new_len);
            Ok(())
        }
    }
}

impl<T: Zeroable, A: MutBumpAllocatorExt> PrivateVecExt for MutBumpVec<T, A> {
    #[inline]
    fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E>
    where
        T: Zeroable,
    {
        self.generic_reserve(additional)?;

        unsafe {
            let ptr = self.as_mut_ptr();
            let len = self.len();

            ptr.add(len).write_bytes(0, additional);
            self.set_len(len + additional);
        }

        Ok(())
    }

    #[inline]
    fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E>
    where
        T: Zeroable,
    {
        let len = self.len();

        if new_len > len {
            self.generic_extend_zeroed(new_len - len)
        } else {
            self.truncate(new_len);
            Ok(())
        }
    }
}

impl<T: Zeroable, A: MutBumpAllocatorExt> PrivateVecExt for MutBumpVecRev<T, A> {
    #[inline]
    fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E> {
        self.generic_reserve(additional)?;

        unsafe {
            let new_len = self.len() + additional;
            self.end.sub(new_len).as_ptr().write_bytes(0, additional);
            self.set_len(new_len);
        }

        Ok(())
    }

    #[inline]
    fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E> {
        let len = self.len();

        if new_len > len {
            self.generic_extend_zeroed(new_len - len)
        } else {
            self.truncate(new_len);
            Ok(())
        }
    }
}
