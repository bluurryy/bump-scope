use core::{ffi::CStr, fmt};

use crate::{
    BumpBox, MutBumpString, MutBumpVec, MutBumpVecRev,
    alloc::AllocError,
    traits::{BumpAllocatorTypedScope, MutBumpAllocatorCoreScope, MutBumpAllocatorTyped},
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// A mutable bump allocator scope with convenient `alloc*` methods.
pub trait MutBumpAllocatorTypedScope<'a>: MutBumpAllocatorCoreScope<'a> + MutBumpAllocatorTyped {
    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`alloc_iter`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`] instead.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    ///
    /// [`alloc_iter`]: crate::traits::BumpAllocatorTypedScope::alloc_iter
    /// [`alloc_iter_mut_rev`]: crate::traits::MutBumpAllocatorTypedScope::alloc_iter_mut_rev
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_iter_mut<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVec::<T, &mut Self>::with_capacity_in(capacity, self);

        for value in iter {
            vec.push(value);
        }

        vec.into_boxed_slice()
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_iter`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// When bumping downwards, prefer [`try_alloc_iter_mut_rev`] instead.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_mut([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_iter`]: crate::traits::BumpAllocatorTypedScope::alloc_iter
    /// [`try_alloc_iter_mut_rev`]: crate::traits::MutBumpAllocatorTypedScope::alloc_iter_mut_rev
    #[inline(always)]
    fn try_alloc_iter_mut<T>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<'a, [T]>, AllocError> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVec::<T, &mut Self>::try_with_capacity_in(capacity, self)?;

        for value in iter {
            vec.try_push(value)?;
        }

        Ok(vec.into_boxed_slice())
    }

    /// Allocate elements of an iterator into a slice in reverse order.
    ///
    /// Compared to [`alloc_iter_mut`] this function is more performant
    /// for downwards bumping allocators as the allocation for the vector can be shrunk in place
    /// without any `ptr::copy`.
    ///
    /// The reverse is true when upwards allocating. In that case it's better to use [`alloc_iter_mut`] to prevent
    /// the `ptr::copy`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut_rev([1, 2, 3]);
    /// assert_eq!(slice, [3, 2, 1]);
    /// ```
    ///
    /// [`alloc_iter_mut`]: crate::traits::MutBumpAllocatorTypedScope::alloc_iter_mut
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_iter_mut_rev<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVecRev::<T, &mut Self>::with_capacity_in(capacity, self);

        for value in iter {
            vec.push(value);
        }

        vec.into_boxed_slice()
    }

    /// Allocate elements of an iterator into a slice in reverse order.
    ///
    /// Compared to [`try_alloc_iter_mut`] this function is more performant
    /// for downwards bumping allocators as the allocation for the vector can be shrunk in place
    /// without any `ptr::copy`.
    ///
    /// The reverse is true when upwards allocating. In that case it's better to use [`try_alloc_iter_mut`] to prevent
    /// the `ptr::copy`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let slice = bump.try_alloc_iter_mut_rev([1, 2, 3])?;
    /// assert_eq!(slice, [3, 2, 1]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_iter_mut`]: crate::traits::MutBumpAllocatorTypedScope::try_alloc_iter_mut
    #[inline(always)]
    fn try_alloc_iter_mut_rev<T>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<'a, [T]>, AllocError> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVecRev::<T, &mut Self>::try_with_capacity_in(capacity, self)?;

        for value in iter {
            vec.try_push(value)?;
        }

        Ok(vec.into_boxed_slice())
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`alloc_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    ///
    /// [`alloc_fmt`]: crate::traits::BumpAllocatorTypedScope::alloc_fmt
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_fmt_mut(&mut self, args: fmt::Arguments) -> BumpBox<'a, str> {
        if let Some(string) = args.as_str() {
            return self.alloc_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_boxed_str()
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_fmt`]: crate::traits::BumpAllocatorTypedScope::try_alloc_fmt
    #[inline(always)]
    fn try_alloc_fmt_mut(&mut self, args: fmt::Arguments) -> Result<BumpBox<'a, str>, AllocError> {
        if let Some(string) = args.as_str() {
            return self.try_alloc_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        string.generic_write_fmt::<AllocError>(args)?;
        Ok(string.into_boxed_str())
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`alloc_cstr_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt_mut(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    ///
    /// [`alloc_cstr_fmt`]: crate::traits::BumpAllocatorTypedScope::alloc_cstr_fmt
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> &'a CStr {
        if let Some(string) = args.as_str() {
            return self.alloc_cstr_from_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_cstr()
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_cstr_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::try_new()?;
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt_mut(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_cstr_fmt`]: crate::traits::BumpAllocatorTypedScope::try_alloc_cstr_fmt
    #[inline(always)]
    fn try_alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> Result<&'a CStr, AllocError> {
        if let Some(string) = args.as_str() {
            return self.try_alloc_cstr_from_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        string.generic_write_fmt::<AllocError>(args)?;
        string.generic_into_cstr()
    }
}

impl<'a, A: MutBumpAllocatorCoreScope<'a> + MutBumpAllocatorTyped + ?Sized> MutBumpAllocatorTypedScope<'a> for A {}
