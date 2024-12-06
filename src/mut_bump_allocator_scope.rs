use core::{ffi::CStr, fmt};

use crate::{panic_on_error, BumpAllocatorScope, BumpBox, MutBumpAllocator, MutBumpString, MutBumpVec, MutBumpVecRev};

/// Shorthand for <code>[MutBumpAllocator] + [BumpAllocatorScope]<'a></code>
///
/// TODO: not just a shorthand ...
pub trait MutBumpAllocatorScope<'a>: MutBumpAllocator + BumpAllocatorScope<'a> {
    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`alloc_fmt`](Self::alloc_fmt). By taking `self` as `&mut`, it can use the entire remaining chunk space
    /// as the capacity for its string buffer. As a result, the string buffer rarely needs to grow.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// Panics if a formatting trait implementation returned an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    fn alloc_fmt_mut(&mut self, args: fmt::Arguments) -> BumpBox<'a, str>
    where
        Self: Sized,
    {
        if let Some(string) = args.as_str() {
            return self.alloc_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_boxed_str()
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop there.
    ///
    /// This function is designed as a performance improvement over [`alloc_fmt`](Self::alloc_fmt). By taking `self` as `&mut`, it can use the entire remaining chunk space
    /// as the capacity for its string buffer. As a result, the string buffer rarely needs to grow.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// Panics if a formatting trait implementation returned an error.
    ///
    /// # Examples
    ///
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
    fn alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> &'a CStr
    where
        Self: Sized,
    {
        if let Some(string) = args.as_str() {
            return self.alloc_cstr_from_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_cstr()
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`alloc_iter`](Self::alloc_iter). By taking `self` as `&mut`, it can use the entire remaining chunk space
    /// as the capacity for its string buffer. As a result, the string buffer rarely needs to grow.
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`](Bump::alloc_iter_mut_rev) instead.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    fn alloc_iter_mut<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]>
    where
        Self: Sized,
    {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVec::<T, &mut Self>::with_capacity_in(capacity, self);

        for value in iter {
            vec.push(value);
        }

        vec.into_boxed_slice()
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
    /// [`alloc_iter_mut`]: Self::alloc_iter_mut
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut_rev([1, 2, 3]);
    /// assert_eq!(slice, [3, 2, 1]);
    /// ```
    fn alloc_iter_mut_rev<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]>
    where
        Self: Sized,
    {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVecRev::<T, &mut Self>::with_capacity_in(capacity, self);

        for value in iter {
            vec.push(value);
        }

        vec.into_boxed_slice()
    }
}

impl<'a, T: MutBumpAllocator + BumpAllocatorScope<'a>> MutBumpAllocatorScope<'a> for T {}
