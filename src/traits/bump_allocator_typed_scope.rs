use core::{ffi::CStr, fmt, mem::MaybeUninit, ptr::NonNull};

#[cfg(feature = "nightly-clone-to-uninit")]
use core::{alloc::Layout, clone::CloneToUninit, ptr};

use crate::{
    BumpBox, BumpString, BumpVec, SizedTypeProperties,
    alloc::AllocError,
    owned_slice::OwnedSlice,
    traits::{BumpAllocatorCoreScope, BumpAllocatorTyped, assert_implements},
};

#[cfg(feature = "panic-on-alloc")]
use crate::panic_on_error;

/// A bump allocator scope with convenient `alloc*` methods.
pub trait BumpAllocatorTypedScope<'a>: BumpAllocatorCoreScope<'a> + BumpAllocatorTyped {
    /// Allocate an object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc(123);
    /// assert_eq!(allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc<T>(&self, value: T) -> BumpBox<'a, T> {
        self.alloc_uninit().init(value)
    }

    /// Allocate an object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc(123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc<T>(&self, value: T) -> Result<BumpBox<'a, T>, AllocError> {
        Ok(self.try_alloc_uninit()?.init(value))
    }

    /// Allocates space for an object, then calls `f` to produce the
    /// value to be put in that place.
    ///
    /// In some cases this could be more performant than `alloc(f())` because it
    /// permits the compiler to directly place `T` in the allocated memory instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_with(|| 123);
    /// assert_eq!(allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> BumpBox<'a, T> {
        self.alloc_uninit().init(f())
    }

    /// Allocates space for an object, then calls `f` to produce the
    /// value to be put in that place.
    ///
    /// In some cases this could be more performant than `try_alloc(f())` because it
    /// permits the compiler to directly place `T` in the allocated memory instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_with(|| 123)?;
    /// assert_eq!(allocated, 123);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<BumpBox<'a, T>, AllocError> {
        Ok(self.try_alloc_uninit()?.init(f()))
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[alloc_with](crate::traits::BumpAllocatorTypedScope::alloc_with)(T::default)</code>.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_default::<i32>();
    /// assert_eq!(allocated, 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_default<T: Default>(&self) -> BumpBox<'a, T> {
        self.alloc_with(T::default)
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[try_alloc_with](crate::traits::BumpAllocatorTypedScope::try_alloc_with)(T::default)</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_default()?;
    /// assert_eq!(allocated, 0);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_default<T: Default>(&self) -> Result<BumpBox<'a, T>, AllocError> {
        self.try_alloc_with(T::default)
    }

    /// Allocate an object by cloning it.
    ///
    /// Unlike `alloc(value.clone())` this method also works for dynamically-sized types.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// Allocate a `slice`, `str`, `CStr`, `Path`:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    ///
    /// let cloned = bump.alloc_clone(&[1, 2, 3]);
    /// assert_eq!(cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.alloc_clone("foo");
    /// assert_eq!(cloned, "foo");
    ///
    /// let cloned = bump.alloc_clone(c"foo");
    /// assert_eq!(cloned, c"foo");
    ///
    /// let cloned = bump.alloc_clone(Path::new("foo"));
    /// assert_eq!(cloned, Path::new("foo"));
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// # use bump_scope::Bump;
    ///
    /// trait FnClone: Fn() -> String + CloneToUninit {}
    /// impl<T: ?Sized + Fn() -> String + CloneToUninit> FnClone for T {}
    ///
    /// // the closure references a local variable
    /// let reference = &String::from("Hello,");
    ///
    /// // and owns a string that it will have to clone
    /// let value = String::from("world!");
    ///
    /// let closure = move || format!("{reference} {value}");
    /// let object: &dyn FnClone = &closure;
    ///
    /// assert_eq!(object(), "Hello, world!");
    ///
    /// let bump: Bump = Bump::new();
    /// let object_clone = bump.alloc_clone(object);
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// ```
    #[cfg(feature = "nightly-clone-to-uninit")]
    fn alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> BumpBox<'a, T> {
        let data = self.allocate_layout(Layout::for_value(value));
        let metadata = ptr::metadata(value);

        unsafe {
            value.clone_to_uninit(data.as_ptr());
            let ptr = ptr::from_raw_parts_mut(data.as_ptr(), metadata);
            let ptr = NonNull::new_unchecked(ptr);
            BumpBox::from_raw(ptr)
        }
    }

    /// Allocate an object by cloning it.
    ///
    /// Unlike `alloc(value.clone())` this method also works for dynamically-sized types.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    ///
    /// Allocate a `slice`, `str`, `CStr`, `Path`:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    ///
    /// let cloned = bump.try_alloc_clone(&[1, 2, 3])?;
    /// assert_eq!(cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.try_alloc_clone("foo")?;
    /// assert_eq!(cloned, "foo");
    ///
    /// let cloned = bump.try_alloc_clone(c"foo")?;
    /// assert_eq!(cloned, c"foo");
    ///
    /// let cloned = bump.try_alloc_clone(Path::new("foo"))?;
    /// assert_eq!(cloned, Path::new("foo"));
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly-clone-to-uninit", doc = "```")]
    #[cfg_attr(not(feature = "nightly-clone-to-uninit"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// # use bump_scope::Bump;
    ///
    /// trait FnClone: Fn() -> String + CloneToUninit {}
    /// impl<T: ?Sized + Fn() -> String + CloneToUninit> FnClone for T {}
    ///
    /// // the closure references a local variable
    /// let reference = &String::from("Hello,");
    ///
    /// // and owns a string that it will have to clone
    /// let value = String::from("world!");
    ///
    /// let closure = move || format!("{reference} {value}");
    /// let object: &dyn FnClone = &closure;
    ///
    /// assert_eq!(object(), "Hello, world!");
    ///
    /// let bump: Bump = Bump::new();
    /// let object_clone = bump.try_alloc_clone(object)?;
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[cfg(feature = "nightly-clone-to-uninit")]
    fn try_alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> Result<BumpBox<'a, T>, AllocError> {
        let data = self.try_allocate_layout(Layout::for_value(value))?;
        let metadata = ptr::metadata(value);

        unsafe {
            value.clone_to_uninit(data.as_ptr());
            let ptr = ptr::from_raw_parts_mut(data.as_ptr(), metadata);
            let ptr = NonNull::new_unchecked(ptr);
            Ok(BumpBox::from_raw(ptr))
        }
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`init`](BumpBox::init) or unsafely with [`assume_init`](BumpBox::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit();
    ///
    /// let five = uninit.init(5);
    ///
    /// assert_eq!(*five, 5)
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.alloc_uninit();
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_uninit<T>(&self) -> BumpBox<'a, MaybeUninit<T>> {
        if T::IS_ZST {
            return BumpBox::zst_uninit();
        }

        let ptr = self.allocate_sized();
        unsafe { BumpBox::from_raw(ptr) }
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`init`](BumpBox::init) or unsafely with [`assume_init`](BumpBox::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.try_alloc_uninit()?;
    ///
    /// let five = uninit.init(5);
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.try_alloc_uninit()?;
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_uninit<T>(&self) -> Result<BumpBox<'a, MaybeUninit<T>>, AllocError> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_uninit());
        }

        let ptr = self.try_allocate_sized()?;
        unsafe { Ok(BumpBox::from_raw(ptr)) }
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// // by value
    /// let a = bump.alloc_slice_move([1, 2]);
    /// let b = bump.alloc_slice_move(vec![3, 4]);
    /// let c = bump.alloc_slice_move(bump.alloc_iter(5..=6));
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.alloc_slice_move(&mut other);
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(a, [1, 2]);
    /// assert_eq!(b, [3, 4]);
    /// assert_eq!(c, [5, 6]);
    /// assert_eq!(d, [7, 8]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> BumpBox<'a, [T]> {
        BumpVec::from_owned_slice_in(slice, self).into_boxed_slice()
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// // by value
    /// let a = bump.try_alloc_slice_move([1, 2])?;
    /// let b = bump.try_alloc_slice_move(vec![3, 4])?;
    /// let c = bump.try_alloc_slice_move(bump.alloc_iter(5..=6))?;
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.try_alloc_slice_move(&mut other)?;
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(a, [1, 2]);
    /// assert_eq!(b, [3, 4]);
    /// assert_eq!(c, [5, 6]);
    /// assert_eq!(d, [7, 8]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> Result<BumpBox<'a, [T]>, AllocError> {
        Ok(BumpVec::try_from_owned_slice_in(slice, self)?.into_boxed_slice())
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_copy(&[1, 2, 3]);
    /// assert_eq!(allocated, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> BumpBox<'a, [T]> {
        if T::IS_ZST {
            return BumpBox::zst_slice_clone(slice);
        }

        let len = slice.len();
        let src = slice.as_ptr();
        let dst = self.allocate_slice_for(slice);

        unsafe {
            core::ptr::copy_nonoverlapping(src, dst.as_ptr(), len);
            BumpBox::from_raw(NonNull::slice_from_raw_parts(dst, len))
        }
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_slice_copy(&[1, 2, 3])?;
    /// assert_eq!(allocated, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, AllocError> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice_clone(slice));
        }

        let len = slice.len();
        let src = slice.as_ptr();
        let dst = self.try_allocate_slice_for(slice)?;

        unsafe {
            core::ptr::copy_nonoverlapping(src, dst.as_ptr(), len);
            Ok(BumpBox::from_raw(NonNull::slice_from_raw_parts(dst, len)))
        }
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_clone(&[String::from("a"), String::from("b")]);
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> BumpBox<'a, [T]> {
        if T::IS_ZST {
            return BumpBox::zst_slice_clone(slice);
        }

        self.alloc_uninit_slice_for(slice).init_clone(slice)
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_slice_clone(&[String::from("a"), String::from("b")])?;
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> Result<BumpBox<'a, [T]>, AllocError> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice_clone(slice));
        }

        Ok(self.try_alloc_uninit_slice_for(slice)?.init_clone(slice))
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill(3, "ho");
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> BumpBox<'a, [T]> {
        if T::IS_ZST {
            return BumpBox::zst_slice_fill(len, value);
        }

        self.alloc_uninit_slice(len).init_fill(value)
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_slice_fill(3, "ho")?;
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> Result<BumpBox<'a, [T]>, AllocError> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice_fill(len, value));
        }

        Ok(self.try_alloc_uninit_slice(len)?.init_fill(value))
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`alloc_slice_fill`](crate::traits::BumpAllocatorTypedScope::alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill_with::<i32>(3, Default::default);
    /// assert_eq!(allocated, [0, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> BumpBox<'a, [T]> {
        if T::IS_ZST {
            return BumpBox::zst_slice_fill_with(len, f);
        }

        self.alloc_uninit_slice(len).init_fill_with(f)
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`try_alloc_slice_fill`](crate::traits::BumpAllocatorTypedScope::try_alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_slice_fill_with::<i32>(3, Default::default)?;
    /// assert_eq!(allocated, [0, 0, 0]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<'a, [T]>, AllocError> {
        if T::IS_ZST {
            return Ok(BumpBox::zst_slice_fill_with(len, f));
        }

        Ok(self.try_alloc_uninit_slice(len)?.init_fill_with(f))
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.alloc_uninit_slice(3);
    ///
    /// let values = uninit.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(values, [1, 2, 3])
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.alloc_uninit_slice(3);
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_uninit_slice<T>(&self, len: usize) -> BumpBox<'a, [MaybeUninit<T>]> {
        let ptr = self.allocate_slice::<MaybeUninit<T>>(len);

        unsafe {
            let slice_ptr = NonNull::slice_from_raw_parts(ptr, len);
            BumpBox::from_raw(slice_ptr)
        }
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let uninit = bump.try_alloc_uninit_slice(3)?;
    ///
    /// let values = uninit.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut uninit = bump.try_alloc_uninit_slice(3)?;
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_uninit_slice<T>(&self, len: usize) -> Result<BumpBox<'a, [MaybeUninit<T>]>, AllocError> {
        let ptr = self.try_allocate_slice::<MaybeUninit<T>>(len)?;

        unsafe {
            let slice_ptr = NonNull::slice_from_raw_parts(ptr, len);
            Ok(BumpBox::from_raw(slice_ptr))
        }
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// This is just like [`alloc_uninit_slice`](crate::traits::BumpAllocatorTypedScope::alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = &[1, 2, 3];
    /// let uninit_slice = bump.alloc_uninit_slice_for(slice);
    /// assert_eq!(uninit_slice.len(), 3);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_uninit_slice_for<T>(&self, slice: &[T]) -> BumpBox<'a, [MaybeUninit<T>]> {
        let ptr = self.allocate_slice_for(slice).cast::<MaybeUninit<T>>();

        unsafe {
            let slice_ptr = NonNull::slice_from_raw_parts(ptr, slice.len());
            BumpBox::from_raw(slice_ptr)
        }
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone),
    /// [`init_zeroed`](crate::zerocopy_08::InitZeroed::init_zeroed) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// This is just like [`try_alloc_uninit_slice`](crate::traits::BumpAllocatorTypedScope::try_alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = &[1, 2, 3];
    /// let uninit_slice = bump.try_alloc_uninit_slice_for(slice)?;
    /// assert_eq!(uninit_slice.len(), 3);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_uninit_slice_for<T>(&self, slice: &[T]) -> Result<BumpBox<'a, [MaybeUninit<T>]>, AllocError> {
        let ptr = self.try_allocate_slice_for(slice)?.cast::<MaybeUninit<T>>();

        unsafe {
            let slice_ptr = NonNull::slice_from_raw_parts(ptr, slice.len());
            Ok(BumpBox::from_raw(slice_ptr))
        }
    }

    /// Allocate a `str`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_str("Hello, world!");
    /// assert_eq!(allocated, "Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_str(&self, src: &str) -> BumpBox<'a, str> {
        let slice = self.alloc_slice_copy(src.as_bytes());

        // SAFETY: input is `str` so this is too
        unsafe { BumpBox::from_utf8_unchecked(slice) }
    }

    /// Allocate a `str`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_str("Hello, world!")?;
    /// assert_eq!(allocated, "Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_str(&self, src: &str) -> Result<BumpBox<'a, str>, AllocError> {
        let slice = self.try_alloc_slice_copy(src.as_bytes())?;

        // SAFETY: input is `str` so this is too
        Ok(unsafe { BumpBox::from_utf8_unchecked(slice) })
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`alloc_fmt_mut`](crate::traits::MutBumpAllocatorTypedScope::alloc_fmt_mut)
    /// instead for better performance.
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
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_fmt(&self, args: fmt::Arguments) -> BumpBox<'a, str> {
        if let Some(string) = args.as_str() {
            return self.alloc_str(string);
        }

        let mut string = BumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_boxed_str()
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_fmt_mut`](crate::traits::MutBumpAllocatorTypedScope::try_alloc_fmt_mut)
    /// instead for better performance.
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
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt(format_args!("{one} + {two} = {}", one + two))?;
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_fmt(&self, args: fmt::Arguments) -> Result<BumpBox<'a, str>, AllocError> {
        if let Some(string) = args.as_str() {
            return self.try_alloc_str(string);
        }

        let mut string = BumpString::new_in(self);
        string.generic_write_fmt::<AllocError>(args)?;
        Ok(string.into_boxed_str())
    }

    /// Allocate a `CStr`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr(c"Hello, world!");
    /// assert_eq!(allocated, c"Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr(&self, src: &CStr) -> &'a CStr {
        let slice = self.alloc_slice_copy(src.to_bytes_with_nul()).into_ref();

        // SAFETY: input is `CStr` so this is too
        unsafe { CStr::from_bytes_with_nul_unchecked(slice) }
    }

    /// Allocate a `CStr`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_cstr(c"Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_cstr(&self, src: &CStr) -> Result<&'a CStr, AllocError> {
        let slice = self.try_alloc_slice_copy(src.to_bytes_with_nul())?.into_ref();

        // SAFETY: input is `CStr` so this is too
        Ok(unsafe { CStr::from_bytes_with_nul_unchecked(slice) })
    }

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr_from_str("Hello, world!");
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.alloc_cstr_from_str("abc\0def");
    /// assert_eq!(allocated, c"abc");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr_from_str(&self, src: &str) -> &'a CStr {
        let src = src.as_bytes();

        if let Some(nul) = src.iter().position(|&c| c == b'\0') {
            let bytes_with_nul = unsafe { src.get_unchecked(..nul + 1) };
            let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) };
            self.alloc_cstr(cstr)
        } else {
            // `src` contains no null
            let dst = self.allocate_slice::<u8>(src.len() + 1);

            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
                dst.as_ptr().add(src.len()).write(0);

                let bytes = core::slice::from_raw_parts(dst.as_ptr(), src.len() + 1);
                CStr::from_bytes_with_nul_unchecked(bytes)
            }
        }
    }

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.try_alloc_cstr_from_str("Hello, world!")?;
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.try_alloc_cstr_from_str("abc\0def")?;
    /// assert_eq!(allocated, c"abc");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_cstr_from_str(&self, src: &str) -> Result<&'a CStr, AllocError> {
        let src = src.as_bytes();

        if let Some(nul) = src.iter().position(|&c| c == b'\0') {
            let bytes_with_nul = unsafe { src.get_unchecked(..nul + 1) };
            let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) };
            self.try_alloc_cstr(cstr)
        } else {
            // `src` contains no null
            let dst = self.try_allocate_slice::<u8>(src.len() + 1)?;

            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
                dst.as_ptr().add(src.len()).write(0);

                let bytes = core::slice::from_raw_parts(dst.as_ptr(), src.len() + 1);
                Ok(CStr::from_bytes_with_nul_unchecked(bytes))
            }
        }
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`alloc_cstr_fmt_mut`](crate::traits::MutBumpAllocatorTypedScope::alloc_cstr_fmt_mut)
    /// instead for better performance.
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
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr_fmt(&self, args: fmt::Arguments) -> &'a CStr {
        if let Some(string) = args.as_str() {
            return self.alloc_cstr_from_str(string);
        }

        let mut string = BumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_cstr()
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_cstr_fmt_mut`](crate::traits::MutBumpAllocatorTypedScope::try_alloc_cstr_fmt_mut)
    /// instead for better performance.
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
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two))?;
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt(format_args!("{one}\0{two}"))?;
    /// assert_eq!(one, c"1");
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_cstr_fmt(&self, args: fmt::Arguments) -> Result<&'a CStr, AllocError> {
        if let Some(string) = args.as_str() {
            return self.try_alloc_cstr_from_str(string);
        }

        let mut string = BumpString::new_in(self);
        string.generic_write_fmt::<AllocError>(args)?;
        string.generic_into_cstr()
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`alloc_iter_mut`].
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    ///
    /// [`alloc_iter_exact`]: crate::traits::BumpAllocatorTypedScope::alloc_iter_exact
    /// [`alloc_iter_mut`]: crate::traits::MutBumpAllocatorTypedScope::alloc_iter_mut
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = BumpVec::<T, &Self>::with_capacity_in(capacity, self);

        for value in iter {
            vec.push(value);
        }

        vec.into_boxed_slice()
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`try_alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`try_alloc_iter_mut`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.try_alloc_iter([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_iter_exact`]: crate::traits::BumpAllocatorTypedScope::try_alloc_iter_exact
    /// [`try_alloc_iter_mut`]: crate::traits::MutBumpAllocatorTypedScope::try_alloc_iter_mut
    #[inline(always)]
    fn try_alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<'a, [T]>, AllocError> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = BumpVec::<T, &Self>::try_with_capacity_in(capacity, self)?;

        for value in iter {
            vec.try_push(value)?;
        }

        Ok(vec.into_boxed_slice())
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_exact([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_iter_exact<T, I>(&self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> BumpBox<'a, [T]>
    where
        I: ExactSizeIterator<Item = T>,
    {
        let mut iter = iter.into_iter();
        let len = iter.len();

        let mut vec = BumpVec::<T, &Self>::with_capacity_in(len, self);

        while vec.len() != vec.capacity() {
            match iter.next() {
                // SAFETY: we checked above that `len != capacity`, so there is space
                Some(value) => unsafe { vec.push_unchecked(value) },
                None => break,
            }
        }

        vec.into_fixed_vec().into_boxed_slice()
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.try_alloc_iter_exact([1, 2, 3])?;
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), bump_scope::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_iter_exact<T, I>(
        &self,
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> Result<BumpBox<'a, [T]>, AllocError>
    where
        I: ExactSizeIterator<Item = T>,
    {
        let mut iter = iter.into_iter();
        let len = iter.len();

        let mut vec = BumpVec::<T, &Self>::try_with_capacity_in(len, self)?;

        while vec.len() != vec.capacity() {
            match iter.next() {
                // SAFETY: we checked above that `len != capacity`, so there is space
                Some(value) => unsafe { vec.push_unchecked(value) },
                None => break,
            }
        }

        Ok(vec.into_fixed_vec().into_boxed_slice())
    }
}

impl<'a, B> BumpAllocatorTypedScope<'a> for B where B: ?Sized + BumpAllocatorCoreScope<'a> + BumpAllocatorTyped {}

assert_implements! {
    [BumpAllocatorTypedScope<'a> + ?Sized]

    &Bump
    &BumpScope

    &mut Bump
    &mut BumpScope

    dyn BumpAllocatorCoreScope
    &dyn BumpAllocatorCoreScope
    &mut dyn BumpAllocatorCoreScope

    dyn MutBumpAllocatorCoreScope
    &dyn MutBumpAllocatorCoreScope
    &mut dyn MutBumpAllocatorCoreScope
}
