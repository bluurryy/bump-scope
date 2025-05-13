use zerocopy::FromZeros;

use crate::{
    alloc::AllocError, polyfill::nonnull, BumpAllocator, BumpVec, ErrorBehavior, FixedBumpVec, MutBumpAllocator, MutBumpVec,
    MutBumpVecRev,
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// Extension trait for this crate's vector types.
pub trait VecExt {
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
    /// # use bump_scope::{ Bump, bump_vec, zerocopy::VecExt };
    /// # let bump: Bump = Bump::new();
    ///
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// ```
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        Self::T: FromZeros;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, bump_vec, zerocopy::VecExt };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        Self::T: FromZeros;

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
    /// # use bump_scope::{ Bump, bump_vec, zerocopy::VecExt };
    /// # let bump: Bump = Bump::new();
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
        Self::T: FromZeros;

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
    /// # use bump_scope::{ Bump, bump_vec, zerocopy::VecExt };
    /// # let bump: Bump = Bump::try_new()?;
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
        Self::T: FromZeros;
}

impl<T> VecExt for FixedBumpVec<'_, T> {
    type T = T;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Panics
    /// Panics if the vector is full.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, zerocopy::VecExt};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump.alloc_fixed_vec(5);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        Self::T: zerocopy::FromZeros,
    {
        panic_on_error(self.generic_extend_zeroed(additional));
    }

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Errors
    /// Errors if the vector is full.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, zerocopy::VecExt};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump.try_alloc_fixed_vec(5)?;
    /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        Self::T: zerocopy::FromZeros,
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
    /// Panics if the vector is full.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, zerocopy::VecExt};
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump.alloc_fixed_vec(5);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.resize_zeroed(5);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = bump.alloc_fixed_vec(5);
    /// vec.extend_from_slice_copy(&[1, 2, 3]);
    /// vec.resize_zeroed(2);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn resize_zeroed(&mut self, new_len: usize)
    where
        T: zerocopy::FromZeros,
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
    /// Errors if the vector is full.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{Bump, zerocopy::VecExt};
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump.try_alloc_fixed_vec(5)?;
    /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
    /// vec.try_resize_zeroed(5)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = bump.try_alloc_fixed_vec(5)?;
    /// vec.try_extend_from_slice_copy(&[1, 2, 3])?;
    /// vec.try_resize_zeroed(2)?;
    /// assert_eq!(vec, [1, 2]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_resize_zeroed(&mut self, new_len: usize) -> Result<(), AllocError>
    where
        Self::T: zerocopy::FromZeros,
    {
        self.generic_resize_zeroed(new_len)
    }
}

impl<T, A: BumpAllocator> VecExt for BumpVec<T, A> {
    type T = T;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, bump_vec, zerocopy::VecExt };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        Self::T: FromZeros,
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
    /// # use bump_scope::{ Bump, bump_vec, zerocopy::VecExt };
    /// # let bump: Bump = Bump::try_new()?;
    /// let mut vec = bump_vec![try in &bump; 1, 2, 3]?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        Self::T: FromZeros,
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
    /// # use bump_scope::{ Bump, bump_vec, zerocopy::VecExt };
    /// # let bump: Bump = Bump::new();
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.resize_zeroed(5);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = bump_vec![in &bump; 1, 2, 3];
    /// vec.resize_zeroed(2);
    /// assert_eq!(vec, [1, 2]);
    /// ```
    fn resize_zeroed(&mut self, new_len: usize)
    where
        Self::T: FromZeros,
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
    /// # use bump_scope::{ Bump, bump_vec, zerocopy::VecExt };
    /// # let bump: Bump = Bump::try_new()?;
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
        T: zerocopy::FromZeros,
    {
        self.generic_resize_zeroed(new_len)
    }
}

impl<T, A: MutBumpAllocator> VecExt for MutBumpVec<T, A> {
    type T = T;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec, zerocopy::VecExt };
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec![in &mut bump; 1, 2, 3];
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        Self::T: zerocopy::FromZeros,
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
    /// # use bump_scope::{ Bump, mut_bump_vec, zerocopy::VecExt };
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec![try in &mut bump; 1, 2, 3]?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        Self::T: zerocopy::FromZeros,
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
    /// # use bump_scope::{ Bump, mut_bump_vec, zerocopy::VecExt };
    /// # let mut bump: Bump = Bump::new();
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
        Self::T: zerocopy::FromZeros,
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
    /// # use bump_scope::{ Bump, mut_bump_vec, zerocopy::VecExt };
    /// # let mut bump: Bump = Bump::try_new()?;
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
        Self::T: zerocopy::FromZeros,
    {
        self.generic_resize_zeroed(new_len)
    }
}

impl<T, A: MutBumpAllocator> VecExt for MutBumpVecRev<T, A> {
    type T = T;

    /// Extends this vector by pushing `additional` new items onto the end.
    /// The new items are initialized with zeroes.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::{ Bump, mut_bump_vec_rev, zerocopy::VecExt };
    /// # let mut bump: Bump = Bump::new();
    /// let mut vec = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
    /// vec.extend_zeroed(2);
    /// assert_eq!(vec, [0, 0, 1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn extend_zeroed(&mut self, additional: usize)
    where
        T: zerocopy::FromZeros,
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
    /// # use bump_scope::{ Bump, mut_bump_vec_rev, zerocopy::VecExt };
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let mut vec = mut_bump_vec_rev![try in bump; 1, 2, 3]?;
    /// vec.try_extend_zeroed(2)?;
    /// assert_eq!(vec, [0, 0, 1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_extend_zeroed(&mut self, additional: usize) -> Result<(), AllocError>
    where
        T: zerocopy::FromZeros,
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
    /// # use bump_scope::{ Bump, mut_bump_vec_rev, zerocopy::VecExt };
    /// # let mut bump: Bump = Bump::new();
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
        T: zerocopy::FromZeros,
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
    /// # use bump_scope::{ Bump, mut_bump_vec_rev, zerocopy::VecExt };
    /// # let mut bump: Bump = Bump::try_new()?;
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
        T: zerocopy::FromZeros,
    {
        self.generic_resize_zeroed(new_len)
    }
}

impl<T> FixedBumpVec<'_, T> {
    #[inline]
    fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E>
    where
        T: FromZeros,
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
        T: FromZeros,
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

impl<T, A: BumpAllocator> BumpVec<T, A> {
    #[inline]
    fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E>
    where
        T: FromZeros,
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
    pub(crate) fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E>
    where
        T: FromZeros,
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

impl<T, A: MutBumpAllocator> MutBumpVec<T, A> {
    #[inline]
    fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E>
    where
        T: zerocopy::FromZeros,
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
        T: zerocopy::FromZeros,
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

impl<T, A: MutBumpAllocator> MutBumpVecRev<T, A> {
    #[inline]
    fn generic_extend_zeroed<E: ErrorBehavior>(&mut self, additional: usize) -> Result<(), E>
    where
        T: zerocopy::FromZeros,
    {
        self.generic_reserve(additional)?;

        unsafe {
            let new_len = self.len() + additional;
            nonnull::sub(self.end, new_len).as_ptr().write_bytes(0, additional);
            self.set_len(new_len);
        }

        Ok(())
    }

    #[inline]
    fn generic_resize_zeroed<E: ErrorBehavior>(&mut self, new_len: usize) -> Result<(), E>
    where
        T: zerocopy::FromZeros,
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
