#[cfg(feature = "nightly")]
use core::clone::CloneToUninit;
use core::{
    alloc::{AllocError, Allocator, Layout},
    ffi::CStr,
    fmt,
    mem::MaybeUninit,
    ptr,
};

#[cfg(feature = "nightly")]
use alloc::alloc::AllocatorNightly;
use alloc::{alloc::Global, boxed::Box, vec::Vec};

use crate::{
    Bump, BumpAllocation, BumpScope, Checkpoint, Unmanaged,
    bump_allocation::IntoRef,
    bump_imp::{AsBumpImp, BumpImp, BumpImpTrait},
    config::{BumpConfig, DEFAULT_BUMP_CONFIG, config},
    containers::{BumpString, BumpVec, MutBumpString, MutBumpVec},
    error_mode::ErrorMode,
    extra::{
        self, box_from_utf8_unchecked, box_init, box_uninit_zst_in, box_uninit_zst_slice_in, boxed_slice_init_clone,
        boxed_slice_init_fill, boxed_slice_init_fill_with,
    },
    owned_slice::{OwnedSlice, TakeOwnedSlice as _},
    util::{self, SizedTypeProperties, down_align_usize, transmute_mut, up_align_usize_unchecked},
};

#[cfg(feature = "panic-on-alloc")]
use crate::error_mode::panic_on_error;

#[cfg(feature = "nightly")]
impl<A: Allocator, const C: BumpConfig> AllocatorNightly for Bump<C, A> {}
#[cfg(feature = "nightly")]
impl<A: Allocator, const C: BumpConfig> AllocatorNightly for BumpScope<'_, C, A> {}
#[cfg(feature = "nightly")]
impl AllocatorNightly for Unmanaged<'_> {}

pub(crate) trait BumpAllocatorPrivate: AsBumpImp + Allocator + Sized {
    const CONFIG: BumpConfig = <<Self as AsBumpImp>::Type<'_> as BumpImpTrait>::CONFIG;

    #[inline(always)]
    fn imp_alloc<E, T>(&self, value: T) -> Result<Box<T, &Self>, E>
    where
        E: ErrorMode,
    {
        Ok(box_init(self.imp_alloc_uninit()?, value))
    }

    #[inline(always)]
    fn imp_alloc_with<E, T>(&self, f: impl FnOnce() -> T) -> Result<Box<T, &Self>, E>
    where
        E: ErrorMode,
    {
        Ok(box_init(self.imp_alloc_uninit()?, f()))
    }

    #[inline(always)]
    fn imp_alloc_default<E, T>(&self) -> Result<Box<T, &Self>, E>
    where
        E: ErrorMode,
        T: Default,
    {
        self.imp_alloc_with(T::default)
    }

    #[inline(always)]
    #[cfg(feature = "nightly")]
    fn imp_alloc_clone<E, T>(&self, value: &T) -> Result<Box<T, &Self>, E>
    where
        E: ErrorMode,
        T: CloneToUninit + ?Sized,
    {
        let data = self.imp().alloc(Layout::for_value(value))?;
        let metadata = ptr::metadata(value);

        unsafe {
            value.clone_to_uninit(data.as_ptr());
            let ptr = ptr::from_raw_parts_mut(data.as_ptr(), metadata);
            Ok(Box::from_raw_in(ptr, self))
        }
    }

    #[inline(always)]
    fn imp_alloc_uninit<E, T>(&self) -> Result<Box<MaybeUninit<T>, &Self>, E>
    where
        E: ErrorMode,
    {
        if T::IS_ZST {
            return unsafe { Ok(Box::from_raw_in(ptr::dangling_mut(), self)) };
        }

        let ptr = self.imp().alloc_sized()?.as_ptr();
        unsafe { Ok(Box::from_raw_in(ptr, self)) }
    }

    #[inline(always)]
    fn imp_alloc_slice_move<E, T>(&self, slice: impl OwnedSlice<Item = T>) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorMode,
    {
        unsafe {
            let mut owned_slice = slice.into_take_owned_slice();
            let owned_slice_ref = owned_slice.owned_slice_ref();

            let len = owned_slice_ref.len();
            let src = owned_slice_ref.as_ptr();
            let dst = self.imp().alloc(Layout::for_value(owned_slice_ref))?.as_ptr().cast();
            ptr::copy_nonoverlapping(src, dst, len);

            owned_slice.take_owned_slice();
            Ok(Box::from_raw_in(ptr::slice_from_raw_parts_mut(dst, len), self))
        }
    }

    #[inline(always)]
    fn imp_alloc_slice_copy<E, T>(&self, slice: &[T]) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorMode,
        T: Copy,
    {
        if T::IS_ZST {
            return Ok(boxed_slice_init_clone(box_uninit_zst_slice_in(slice.len(), self), slice));
        }

        let len = slice.len();
        let src = slice.as_ptr();
        let dst = self.imp().alloc_slice_for(slice)?.as_ptr();

        unsafe {
            core::ptr::copy_nonoverlapping(src, dst, len);
            Ok(Box::from_raw_in(ptr::slice_from_raw_parts_mut(dst, len), self))
        }
    }

    #[inline(always)]
    fn imp_alloc_slice_clone<E, T>(&self, slice: &[T]) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorMode,
        T: Clone,
    {
        Ok(boxed_slice_init_clone(self.imp_alloc_uninit_slice_for(slice)?, slice))
    }

    #[inline(always)]
    fn imp_alloc_slice_fill<E, T>(&self, len: usize, value: T) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorMode,
        T: Clone,
    {
        Ok(boxed_slice_init_fill(self.imp_alloc_uninit_slice(len)?, value))
    }

    #[inline(always)]
    fn imp_alloc_slice_fill_with<E, T>(&self, len: usize, f: impl FnMut() -> T) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorMode,
    {
        Ok(boxed_slice_init_fill_with(self.imp_alloc_uninit_slice(len)?, f))
    }

    #[inline(always)]
    fn imp_alloc_uninit_slice<E, T>(&self, len: usize) -> Result<Box<[MaybeUninit<T>], &Self>, E>
    where
        E: ErrorMode,
    {
        if T::IS_ZST {
            return Ok(box_uninit_zst_slice_in(len, self));
        }

        let ptr = self.imp().alloc_slice::<E, MaybeUninit<T>>(len)?.as_ptr();

        unsafe {
            let slice_ptr = ptr::slice_from_raw_parts_mut(ptr, len);
            Ok(Box::from_raw_in(slice_ptr, self))
        }
    }

    #[inline(always)]
    fn imp_alloc_uninit_slice_for<E, T, U>(&self, slice: &[U]) -> Result<Box<[MaybeUninit<T>], &Self>, E>
    where
        E: ErrorMode,
    {
        if U::LAYOUT != T::LAYOUT {
            return self.imp_alloc_uninit_slice(slice.len());
        }

        let ptr = self.imp().alloc_slice_for(slice)?.cast::<MaybeUninit<T>>().as_ptr();

        unsafe {
            let slice_ptr = ptr::slice_from_raw_parts_mut(ptr, slice.len());
            Ok(Box::from_raw_in(slice_ptr, self))
        }
    }

    #[inline(always)]
    fn imp_alloc_str<E>(&self, src: &str) -> Result<Box<str, &Self>, E>
    where
        E: ErrorMode,
    {
        let bytes = self.imp_alloc_slice_copy(src.as_bytes())?;

        // SAFETY: src is utf8 so this is too
        Ok(unsafe { box_from_utf8_unchecked(bytes) })
    }

    #[inline(always)]
    fn imp_alloc_fmt<E>(&self, args: fmt::Arguments) -> Result<Box<str, &Self>, E>
    where
        E: ErrorMode,
    {
        if let Some(string) = args.as_str() {
            return self.imp_alloc_str(string);
        }

        let mut string = BumpString::<_, E>::from_vec(Vec::new_in(self));
        string.write_fmt(args)?;
        Ok(string.into_str())
    }

    #[inline(always)]
    fn imp_alloc_cstr<E>(&self, src: &CStr) -> Result<IntoRef<CStr, &Self>, E>
    where
        E: ErrorMode,
    {
        let (slice, _) = Box::into_raw_with_allocator(self.imp_alloc_slice_copy(src.to_bytes_with_nul())?);

        // SAFETY: input is `CStr` so this is too
        let ptr = unsafe { CStr::from_bytes_with_nul_unchecked(&*slice) };

        Ok(unsafe { IntoRef::new(ptr, self) })
    }

    #[inline(always)]
    fn imp_alloc_cstr_from_str<E>(&self, src: &str) -> Result<IntoRef<CStr, &Self>, E>
    where
        E: ErrorMode,
    {
        let src = src.as_bytes();

        if let Some(nul) = src.iter().position(|&c| c == b'\0') {
            let bytes_with_nul = unsafe { src.get_unchecked(..nul + 1) };
            let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) };
            self.imp_alloc_cstr(cstr)
        } else {
            // `src` contains no null
            let dst = self.imp().alloc_slice::<E, u8>(src.len() + 1)?;

            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
                dst.as_ptr().add(src.len()).write(0);

                let bytes = core::slice::from_raw_parts(dst.as_ptr(), src.len() + 1);
                let ptr = CStr::from_bytes_with_nul_unchecked(bytes);

                Ok(IntoRef::new(ptr, self))
            }
        }
    }

    #[inline(always)]
    fn imp_alloc_cstr_fmt<E>(&self, args: fmt::Arguments) -> Result<IntoRef<CStr, &Self>, E>
    where
        E: ErrorMode,
    {
        if let Some(string) = args.as_str() {
            return self.imp_alloc_cstr_from_str(string);
        }

        let mut string = BumpString::<_, E>::from_vec(Vec::new_in(self));
        string.write_fmt(args)?;
        string.into_cstr()
    }

    #[inline(always)]
    fn imp_alloc_iter<E, T>(&self, iter: impl IntoIterator<Item = T>) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorMode,
    {
        let mut vec = BumpVec::<T, _, E>::from_vec(Vec::new_in(self));
        vec.extend(iter)?;
        Ok(vec.into_boxed_slice())
    }

    #[inline(always)]
    fn imp_alloc_iter_exact<E, T, I>(&self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorMode,
        I: ExactSizeIterator<Item = T>,
    {
        let iter = iter.into_iter();
        let (len, _) = iter.size_hint();
        let uninit = self.imp_alloc_uninit_slice(len)?;
        let mut vec = extra::vec_from_uninit_boxed_slice(uninit);
        // This can't fail due to allocation since `.take(len)` ensures the vec doesn't grow.
        vec.extend(iter.into_iter().take(len));
        let init = extra::imp_vec_into_boxed_slice_without_shrink(vec);
        Ok(init)
    }

    #[inline(always)]
    fn imp_alloc_try_with<B, T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<Result<Box<T, &Self>, E>, B>
    where
        B: ErrorMode,
    {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(box_init(box_uninit_zst_in(self), value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let checkpoint_before_alloc = self.imp().checkpoint();
        let uninit = self.imp_alloc_uninit::<B, Result<T, E>>()?;
        let ptr = Box::into_raw_with_allocator(uninit).0.cast::<Result<T, E>>();

        // When bumping downwards the chunk's position is the same as `ptr`.
        // Using `ptr` is faster so we use that.
        let pos = if config!(Self::CONFIG, upwards) {
            self.imp().pos()
        } else {
            ptr.cast::<u8>()
        };

        Ok(unsafe {
            ptr.write(f());

            match util::ptr_result(ptr) {
                Ok(value) => Ok(Box::from_raw_in(value, self)),
                Err(error) => Err({
                    let error = error.read();

                    // If `f` made allocations on this bump allocator we can't shrink the allocation.
                    let can_shrink = pos == self.imp().pos();

                    if can_shrink {
                        self.imp().reset_to(checkpoint_before_alloc);
                    }

                    error
                }),
            }
        })
    }

    /// Allocates a vector that uses the entire remaining chunk space for its capacity and
    /// only needs to update the bump pointer when calling [`into_boxed_slice`](MutBumpVec::into_boxed_slice).
    ///
    /// # Safety
    /// Nothing must be done that changes the bump pointer while this vector is live.
    fn imp_mut_alloc_vec<E, T>(&mut self, min_cap: usize) -> Result<MutBumpVec<'_, T, Self>, E>
    where
        E: ErrorMode,
    {
        MutBumpVec::new(min_cap, self)
    }

    #[inline(always)]
    fn imp_mut_alloc_iter<E, T>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorMode,
    {
        if T::IS_ZST {
            let mut vec = Vec::new_in(&*self);
            // This won't allocate, so it's fine to use this in the fallible api too.
            vec.extend(iter);
            return Ok(vec.into_boxed_slice());
        }

        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;
        let mut vec = self.imp_mut_alloc_vec::<E, T>(capacity)?;
        vec.extend::<E>(iter)?;
        Ok(vec.into_boxed_slice())
    }

    #[inline(always)]
    fn imp_mut_alloc_fmt<E>(&mut self, args: fmt::Arguments) -> Result<Box<str, &Self>, E>
    where
        E: ErrorMode,
    {
        if let Some(string) = args.as_str() {
            return (*self).imp_alloc_str(string);
        }

        let vec = self.imp_mut_alloc_vec::<E, u8>(0)?;
        let mut string = MutBumpString::<_, E>::from_vec(vec);
        string.write_fmt(args)?;
        Ok(string.into_str())
    }

    #[inline(always)]
    fn imp_mut_alloc_cstr_fmt<E>(&mut self, args: fmt::Arguments) -> Result<IntoRef<CStr, &Self>, E>
    where
        E: ErrorMode,
    {
        if let Some(string) = args.as_str() {
            return (*self).imp_alloc_cstr_from_str(string);
        }

        let vec = self.imp_mut_alloc_vec(0)?;
        let mut string = MutBumpString::<_, E>::from_vec(vec);
        string.write_fmt(args)?;
        string.into_cstr()
    }

    #[inline(always)]
    fn imp_mut_alloc_try_with<B, T, E>(&mut self, f: impl FnOnce() -> Result<T, E>) -> Result<Result<Box<T, &Self>, E>, B>
    where
        B: ErrorMode,
    {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(box_init(box_uninit_zst_in(self), value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let ptr = (*self).imp().prepare_sized_allocation::<B, Result<T, E>>()?.as_ptr();

        Ok(unsafe {
            ptr.write(f());

            // There is no need for `can_shrink` checks, because we have a mutable reference
            // so there's no way anyone else has allocated in `f`.
            match util::ptr_result(ptr) {
                Ok(value) => Ok({
                    let new_pos = if config!(Self::CONFIG, upwards) {
                        let pos = value.add(1).addr();
                        up_align_usize_unchecked(pos, config!(Self::CONFIG, min_align))
                    } else {
                        let pos = value.addr();
                        down_align_usize(pos, config!(Self::CONFIG, min_align))
                    };

                    // SAFETY: The allocation was successful, so our chunk must be allocated.
                    self.imp().set_pos(new_pos);

                    Box::from_raw_in(value, self)
                }),
                Err(error) => Err(error.read()),
            }
        })
    }
}
impl<A: BumpAllocatorPrivate> BumpAllocatorPrivate for &A {}
impl<A: BumpAllocatorPrivate> BumpAllocatorPrivate for &mut A {}
impl<const C: BumpConfig, A: Allocator, const IS_SCOPE: bool> BumpAllocatorPrivate for BumpImp<'_, C, A, IS_SCOPE> {}
impl<A: Allocator, const C: BumpConfig> BumpAllocatorPrivate for Bump<C, A> {}
impl<A: Allocator, const C: BumpConfig> BumpAllocatorPrivate for BumpScope<'_, C, A> {}
impl BumpAllocatorPrivate for Unmanaged<'_> {}

/// A bump allocator.
///
/// Provides methods for [`Bump`] and [`BumpScope`].
///
/// # Safety
/// This trait provides additional guarantees on top of [`Allocator`] and laxer safety conditions for its methods:
/// - You can call `grow*`, `shrink` and `deallocate` with pointers from a non-[equivalent] `BumpAllocator`.
///   - used for [`extra::box_replace_allocator`] and [`extra::vec_replace_allocator`]
/// - Memory blocks can be split.
///   - used for [`extra::vec_split_off`], [`extra::boxed_slice_split_off`], [`extra::boxed_str_split_off`]
/// - `shrink` never errors unless the new alignment is greater
///
/// [equivalent]: https://doc.rust-lang.org/nightly/std/alloc/trait.Allocator.html#equivalent-allocators
#[expect(private_bounds)]
pub unsafe trait BumpAllocator: Allocator + BumpAllocatorPrivate {
    /// Allocate an object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc(123).into_boxu();
    /// assert_eq!(*allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc<T>(&self, value: T) -> BumpAllocation<T, &Self> {
        let boxed = panic_on_error(self.imp_alloc(value));
        BumpAllocation(boxed)
    }

    /// Allocate an object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc(123)?.into_boxu();
    /// assert_eq!(*allocated, 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc<T>(&self, value: T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxed = self.imp_alloc(value)?;
        Ok(BumpAllocation(boxed))
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
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_with(|| 123).into_boxu();
    /// assert_eq!(*allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> BumpAllocation<T, &Self> {
        let boxed = panic_on_error(self.imp_alloc_with(f));
        BumpAllocation(boxed)
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
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_with(|| 123)?.into_boxu();
    /// assert_eq!(*allocated, 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxed = self.imp_alloc_with(f)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[alloc_with](BumpAllocator::alloc_with)(T::default)</code>.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_default::<i32>().into_boxu();
    /// assert_eq!(*allocated, 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_default<T: Default>(&self) -> BumpAllocation<T, &Self> {
        let boxed = panic_on_error(self.imp_alloc_default());
        BumpAllocation(boxed)
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[try_alloc_with](BumpAllocator::try_alloc_with)(T::default)</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_default::<i32>()?.into_boxu();
    /// assert_eq!(*allocated, 0);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_default<T: Default>(&self) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxed = self.imp_alloc_default()?;
        Ok(BumpAllocation(boxed))
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
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let cloned = bump.alloc_clone(&[1, 2, 3]).into_boxu();
    /// assert_eq!(&*cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.alloc_clone("foo").into_boxu();
    /// assert_eq!(&*cloned, "foo");
    ///
    /// let cloned = bump.alloc_clone(c"foo").into_boxu();
    /// assert_eq!(&*cloned, c"foo");
    ///
    /// let cloned = bump.alloc_clone(Path::new("foo")).into_boxu();
    /// assert_eq!(&*cloned, Path::new("foo"));
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// use bump_scope::{Bump, BumpAllocator};
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
    /// let object_clone = bump.alloc_clone(object).into_boxu();
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// ```
    #[inline(never)]
    #[cfg(feature = "nightly")]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> BumpAllocation<T, &Self> {
        let boxed = panic_on_error(self.imp_alloc_clone(value));
        BumpAllocation(boxed)
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
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let cloned = bump.try_alloc_clone(&[1, 2, 3])?.into_boxu();
    /// assert_eq!(&*cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.try_alloc_clone("foo")?.into_boxu();
    /// assert_eq!(&*cloned, "foo");
    ///
    /// let cloned = bump.try_alloc_clone(c"foo")?.into_boxu();
    /// assert_eq!(&*cloned, c"foo");
    ///
    /// let cloned = bump.try_alloc_clone(Path::new("foo"))?.into_boxu();
    /// assert_eq!(&*cloned, Path::new("foo"));
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// use bump_scope::{Bump, BumpAllocator};
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
    /// let object_clone = bump.try_alloc_clone(object)?.into_boxu();
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(never)]
    #[cfg(feature = "nightly")]
    fn try_alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxed = self.imp_alloc_clone(value)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`box_init`],
    /// `box_zero_init` <sup>[\[1\]](crate::zerocopy_08::box_zero_init) [\[2\]](crate::bytemuck::box_zero_init)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, extra};
    /// let bump: Bump = Bump::new();
    ///
    /// let uninit = bump.alloc_uninit().into_boxu();
    /// let five = extra::box_init(uninit, 5);
    ///
    /// assert_eq!(*five, 5)
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut uninit = bump.alloc_uninit().into_boxu();
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
    fn alloc_uninit<T>(&self) -> BumpAllocation<MaybeUninit<T>, &Self> {
        let boxed = panic_on_error(self.imp_alloc_uninit());
        BumpAllocation(boxed)
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`box_init`],
    /// `box_zero_init` <sup>[\[1\]](crate::bytemuck::box_zero_init) [\[2\]](crate::zerocopy_08::box_zero_init)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, extra};
    /// let bump: Bump = Bump::new();
    ///
    /// let uninit = bump.try_alloc_uninit()?.into_boxu();
    /// let five = extra::box_init(uninit, 5);
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut uninit = bump.try_alloc_uninit()?.into_boxu();
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_uninit<T>(&self) -> Result<BumpAllocation<MaybeUninit<T>, &Self>, AllocError> {
        let boxed = self.imp_alloc_uninit()?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// // by value
    /// let a = bump.alloc_slice_move([1, 2]).into_boxu();
    /// let b = bump.alloc_slice_move(vec![3, 4]).into_boxu();
    /// let c = bump.alloc_slice_move(bump.alloc_iter(5..=6).into_boxu()).into_boxu();
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.alloc_slice_move(&mut other).into_boxu();
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(&*a, [1, 2]);
    /// assert_eq!(&*b, [3, 4]);
    /// assert_eq!(&*c, [5, 6]);
    /// assert_eq!(&*d, [7, 8]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_move(slice));
        BumpAllocation(boxed)
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// // by value
    /// let a = bump.try_alloc_slice_move([1, 2])?.into_boxu();
    /// let b = bump.try_alloc_slice_move(vec![3, 4])?.into_boxu();
    /// let c = bump.try_alloc_slice_move(bump.alloc_iter(5..=6).into_boxu())?.into_boxu();
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.try_alloc_slice_move(&mut other)?.into_boxu();
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(&*a, [1, 2]);
    /// assert_eq!(&*b, [3, 4]);
    /// assert_eq!(&*c, [5, 6]);
    /// assert_eq!(&*d, [7, 8]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_move(slice)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_slice_copy(&[1, 2, 3]).into_boxu();
    /// assert_eq!(&*allocated, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_copy(slice));
        BumpAllocation(boxed)
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_slice_copy(&[1, 2, 3])?.into_boxu();
    /// assert_eq!(&*allocated, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_copy(slice)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_slice_clone(&[String::from("a"), String::from("b")]).into_boxu();
    /// assert_eq!(&*allocated, [String::from("a"), String::from("b")]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_clone(slice));
        BumpAllocation(boxed)
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_slice_clone(&[String::from("a"), String::from("b")])?.into_boxu();
    /// assert_eq!(&*allocated, [String::from("a"), String::from("b")]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_clone(slice)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_slice_fill(3, "ho").into_boxu();
    /// assert_eq!(&*allocated, ["ho", "ho", "ho"]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_fill(len, value));
        BumpAllocation(boxed)
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_slice_fill(3, "ho")?.into_boxu();
    /// assert_eq!(&*allocated, ["ho", "ho", "ho"]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_fill(len, value)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`alloc_slice_fill`](BumpAllocator::alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_slice_fill_with::<i32>(3, Default::default).into_boxu();
    /// assert_eq!(&*allocated, [0, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_fill_with(len, f));
        BumpAllocation(boxed)
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`try_alloc_slice_fill`](BumpAllocator::try_alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_slice_fill_with::<i32>(3, Default::default)?.into_boxu();
    /// assert_eq!(&*allocated, [0, 0, 0]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_fill_with<T>(
        &self,
        len: usize,
        f: impl FnMut() -> T,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_fill_with(len, f)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`boxed_slice_init_fill`],
    /// [`boxed_slice_init_fill_with`],
    /// [`boxed_slice_init_clone`],
    /// `box_zero_init_slice` <sup>[\[1\]](crate::zerocopy_08::box_zero_init_slice) [\[2\]](crate::bytemuck::box_zero_init_slice)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, extra::boxed_slice_init_copy};
    /// let bump: Bump = Bump::new();
    ///
    /// let uninit = bump.alloc_uninit_slice(3).into_boxu();
    /// let values = boxed_slice_init_copy(uninit, &[1, 2, 3]);
    ///
    /// assert_eq!(&*values, [1, 2, 3])
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut uninit = bump.alloc_uninit_slice(3).into_boxu();
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*values, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_uninit_slice<T>(&self, len: usize) -> BumpAllocation<[MaybeUninit<T>], &Self> {
        let boxed = panic_on_error(self.imp_alloc_uninit_slice(len));
        BumpAllocation(boxed)
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`boxed_slice_init_fill`],
    /// [`boxed_slice_init_fill_with`],
    /// [`boxed_slice_init_clone`],
    /// `box_zero_init_slice` <sup>[\[1\]](crate::zerocopy_08::box_zero_init_slice) [\[2\]](crate::bytemuck::box_zero_init_slice)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, extra::boxed_slice_init_copy};
    /// let bump: Bump = Bump::new();
    ///
    /// let uninit = bump.try_alloc_uninit_slice(3)?.into_boxu();
    /// let values = boxed_slice_init_copy(uninit, &[1, 2, 3]);
    ///
    /// assert_eq!(&*values, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut uninit = bump.try_alloc_uninit_slice(3)?.into_boxu();
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*values, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_uninit_slice<T>(&self, len: usize) -> Result<BumpAllocation<[MaybeUninit<T>], &Self>, AllocError> {
        let boxed = self.imp_alloc_uninit_slice(len)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`boxed_slice_init_fill`],
    /// [`boxed_slice_init_fill_with`],
    /// [`boxed_slice_init_clone`],
    /// `box_zero_init_slice` <sup>[\[1\]](crate::zerocopy_08::box_zero_init_slice) [\[2\]](crate::bytemuck::box_zero_init_slice)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// When the layouts of `T` and `U` are the same, this avoids checking for a valid `Layout` since the given slice can provide a valid one.
    /// Otherwise it falls back to [`alloc_uninit_slice`](BumpAllocator::alloc_uninit_slice).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice: &[u32] = &[1, 2, 3];
    /// let uninit_slice = bump.alloc_uninit_slice_for::<char, _>(slice).into_boxu();
    /// assert_eq!(uninit_slice.len(), 3);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_uninit_slice_for<T, U>(&self, slice: &[U]) -> BumpAllocation<[MaybeUninit<T>], &Self> {
        let boxed = panic_on_error(self.imp_alloc_uninit_slice_for(slice));
        BumpAllocation(boxed)
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`boxed_slice_init_fill`],
    /// [`boxed_slice_init_fill_with`],
    /// [`boxed_slice_init_clone`],
    /// `box_zero_init_slice` <sup>[\[1\]](crate::zerocopy_08::box_zero_init_slice) [\[2\]](crate::bytemuck::box_zero_init_slice)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// When the layouts of `T` and `U` are the same, this avoids checking for a valid `Layout` since the given slice can provide a valid one.
    /// Otherwise it falls back to [`try_alloc_uninit_slice`](BumpAllocator::try_alloc_uninit_slice).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice: &[u32] = &[1, 2, 3];
    /// let uninit_slice = bump.try_alloc_uninit_slice_for::<char, _>(slice)?.into_boxu();
    /// assert_eq!(uninit_slice.len(), 3);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_uninit_slice_for<T, U>(&self, slice: &[U]) -> Result<BumpAllocation<[MaybeUninit<T>], &Self>, AllocError> {
        let boxed = self.imp_alloc_uninit_slice_for(slice)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a `str`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_str("Hello, world!").into_ref();
    /// assert_eq!(allocated, "Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_str(&self, src: &str) -> BumpAllocation<str, &Self> {
        let boxed = panic_on_error(self.imp_alloc_str(src));
        BumpAllocation(boxed)
    }

    /// Allocate a `str`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_str("Hello, world!")?.into_ref();
    /// assert_eq!(allocated, "Hello, world!");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_str(&self, src: &str) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxed = self.imp_alloc_str(src)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`mut_alloc_fmt`](MutBumpAllocator::mut_alloc_fmt)
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
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two)).into_ref();
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_fmt(&self, args: fmt::Arguments) -> BumpAllocation<str, &Self> {
        let boxed = panic_on_error(self.imp_alloc_fmt(args));
        BumpAllocation(boxed)
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`try_mut_alloc_fmt`](MutBumpAllocator::try_mut_alloc_fmt)
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
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt(format_args!("{one} + {two} = {}", one + two))?.into_ref();
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_fmt(&self, args: fmt::Arguments) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxed = self.imp_alloc_fmt(args)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a `CStr`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_cstr(c"Hello, world!").into_ref();
    /// assert_eq!(allocated, c"Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr(&self, src: &CStr) -> IntoRef<CStr, &Self> {
        panic_on_error(self.imp_alloc_cstr(src))
    }

    /// Allocate a `CStr`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_cstr(c"Hello, world!")?.into_ref();
    /// assert_eq!(allocated, c"Hello, world!");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_cstr(&self, src: &CStr) -> Result<IntoRef<CStr, &Self>, AllocError> {
        self.imp_alloc_cstr(src)
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
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_cstr_from_str("Hello, world!").into_ref();
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.alloc_cstr_from_str("abc\0def").into_ref();
    /// assert_eq!(allocated, c"abc");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr_from_str(&self, src: &str) -> IntoRef<CStr, &Self> {
        panic_on_error(self.imp_alloc_cstr_from_str(src))
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
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_cstr_from_str("Hello, world!")?.into_ref();
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.try_alloc_cstr_from_str("abc\0def")?.into_ref();
    /// assert_eq!(allocated, c"abc");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_cstr_from_str(&self, src: &str) -> Result<IntoRef<CStr, &Self>, AllocError> {
        self.imp_alloc_cstr_from_str(src)
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`mut_alloc_cstr_fmt`](MutBumpAllocator::mut_alloc_cstr_fmt)
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
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two)).into_ref();
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt(format_args!("{one}\0{two}")).into_ref();
    /// assert_eq!(one, c"1");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr_fmt(&self, args: fmt::Arguments) -> IntoRef<CStr, &Self> {
        panic_on_error(self.imp_alloc_cstr_fmt(args))
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`try_mut_alloc_cstr_fmt`](MutBumpAllocator::try_mut_alloc_cstr_fmt)
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
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two))?.into_ref();
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt(format_args!("{one}\0{two}"))?.into_ref();
    /// assert_eq!(one, c"1");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_cstr_fmt(&self, args: fmt::Arguments) -> Result<IntoRef<CStr, &Self>, AllocError> {
        self.imp_alloc_cstr_fmt(args)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have a `&mut self` you can get somewhat better performance by using [`mut_alloc_iter`].
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = bump.alloc_iter([1, 2, 3]).into_boxu();
    /// assert_eq!(&*slice, [1, 2, 3]);
    /// ```
    ///
    /// [`mut_alloc_iter`]: MutBumpAllocator::mut_alloc_iter
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_iter(iter));
        BumpAllocation(boxed)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have a `&mut self` you can get somewhat better performance by using [`try_mut_alloc_iter`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = bump.try_alloc_iter([1, 2, 3])?.into_boxu();
    /// assert_eq!(&*slice, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_mut_alloc_iter`]: MutBumpAllocator::try_mut_alloc_iter
    #[inline(always)]
    fn try_alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_iter(iter)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = bump.alloc_iter_exact([1, 2, 3]).into_boxu();
    /// assert_eq!(&*slice, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_iter_exact<T, I>(&self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> BumpAllocation<[T], &Self>
    where
        I: ExactSizeIterator<Item = T>,
    {
        let boxed = panic_on_error(self.imp_alloc_iter_exact(iter));
        BumpAllocation(boxed)
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = bump.try_alloc_iter_exact([1, 2, 3])?.into_boxu();
    /// assert_eq!(&*slice, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_iter_exact<T, I>(
        &self,
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError>
    where
        I: ExactSizeIterator<Item = T>,
    {
        let boxed = self.imp_alloc_iter_exact(iter)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// There is also [`mut_alloc_try_with`], optimized for a mutable reference.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(bump.allocated(), size_of::<Result<i32, i32>>());
    /// assert_eq!(*result.unwrap().into_boxu(), 123);
    /// ```
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(bump.allocated(), 0);
    /// assert_eq!(result.unwrap_err(), 123);
    /// ```
    ///
    /// [`mut_alloc_try_with`]: MutBumpAllocator::mut_alloc_try_with
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpAllocation<T, &Self>, E> {
        match panic_on_error(self.imp_alloc_try_with(f)) {
            Ok(boxed) => Ok(BumpAllocation(boxed)),
            Err(err) => Err(err),
        }
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// There is also [`try_mut_alloc_try_with`], optimized for a mutable reference.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(bump.allocated(), size_of::<Result<i32, i32>>());
    /// assert_eq!(*result.unwrap().into_boxu(), 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(bump.allocated(), 0);
    /// assert_eq!(result.unwrap_err(), 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_mut_alloc_try_with`]: MutBumpAllocator::try_mut_alloc_try_with
    #[inline(always)]
    fn try_alloc_try_with<T, E>(
        &self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpAllocation<T, &Self>, E>, AllocError> {
        Ok(match self.imp_alloc_try_with(f)? {
            Ok(boxed) => Ok(BumpAllocation(boxed)),
            Err(err) => Err(err),
        })
    }

    /// Creates a checkpoint of the current bump position.
    ///
    /// The bump position can be reset to this checkpoint with [`reset_to`].
    ///
    /// [`reset_to`]: BumpAllocator::reset_to
    #[must_use]
    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        self.imp().checkpoint()
    }

    /// Resets the bump position to a previously created checkpoint.
    ///
    /// The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    ///
    /// - This method invalidates all memory blocks allocated since creation of this checkpoint.
    /// - The provided checkpoint must have been created by this bump allocator.
    /// - The bump allocator must not have been [`reset`] since creation of this checkpoint.
    /// - The checkpoint must not have been created by a bump allocator with no chunks when self is **not** [`const_constructible`].
    /// - The checkpoint must not have been created by a [claimed] bump allocator when self is **not** [`claimable`].
    ///
    /// [`reset`]: crate::Bump::reset
    /// [claimed]: crate::BumpScope::claim
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.allocated(), 5);
    ///     # _ = hello;
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.allocated(), 0);
    /// ```
    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe { self.imp().reset_to(checkpoint) }
    }

    /// Returns true if the bump allocator is currently [claimed].
    ///
    /// [claimed]: crate::BumpScope::claim
    #[must_use]
    #[inline(always)]
    fn is_claimed(&self) -> bool {
        self.imp().is_claimed()
    }

    /// Returns the amount of allocated bytes.
    /// This includes padding and wasted space due to reallocations.
    ///
    /// Equivalent to `.[stats]\().[allocated]\()`.
    ///
    /// [stats]: Bump::stats
    /// [allocated]: crate::stats::Stats::allocated
    #[inline(always)]
    fn allocated(&self) -> usize {
        self.imp().allocated()
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve`, <code>self.[stats][]().[remaining][]()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that these bytes are not necessarily in one contiguous region but
    /// might be spread across multiple chunks.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    ///
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    ///
    /// [stats]: crate::BumpScope::stats
    /// [remaining]: crate::stats::Stats::remaining
    /// [claimed]: crate::BumpScope::claim
    #[inline]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        panic_on_error(self.imp().reserve(additional));
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve`, <code>self.[stats][]().[remaining][]()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that these bytes are not necessarily in one contiguous region but
    /// might be spread across multiple chunks.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// Errors if the bump allocator is currently [claimed].
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.try_reserve(4096)?;
    /// assert!(bump.stats().capacity() >= 4096);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [stats]: crate::BumpScope::stats
    /// [remaining]: crate::stats::Stats::remaining
    /// [claimed]: crate::BumpScope::claim
    #[inline]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        self.imp().reserve(additional)
    }
}
unsafe impl<T: BumpAllocator> BumpAllocator for &T {}
unsafe impl<T: BumpAllocator> BumpAllocator for &mut T {}
unsafe impl<A: Allocator, const C: BumpConfig> BumpAllocator for Bump<C, A> {}
unsafe impl<A: Allocator, const C: BumpConfig> BumpAllocator for BumpScope<'_, C, A> {}
unsafe impl BumpAllocator for Unmanaged<'_> {}

/// A bump allocator scope.
///
/// This is a [`BumpAllocator`] which can make allocations that live for the lifetime
/// of `'a` instead of (just) the lifetime of itself.
///
/// # Examples
/// This function is sound:
/// ```
/// # #![expect(dead_code)]
/// use bump_scope::BumpAllocatorScope;
/// use core::alloc::Layout;
///
/// fn allocate_zeroed_bytes<'a>(allocator: impl BumpAllocatorScope<'a>, len: usize) -> &'a [u8] {
///     let layout = Layout::array::<u8>(len).unwrap();
///     let ptr = allocator.allocate_zeroed(layout).unwrap();
///     unsafe { ptr.as_ref() }
/// }
/// ```
#[expect(private_bounds, clippy::missing_safety_doc)]
pub unsafe trait BumpAllocatorScope<'a>: BumpAllocator + BumpAllocatorPrivate {}
unsafe impl<'a, T: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for &T {}
unsafe impl<'a, T: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for &mut T {}
unsafe impl<'a, A: Allocator, const C: BumpConfig> BumpAllocatorScope<'a> for &'a Bump<C, A> {}
unsafe impl<'a, A: Allocator, const C: BumpConfig> BumpAllocatorScope<'a> for &'a mut Bump<C, A> {}
unsafe impl<'a, A: Allocator, const C: BumpConfig> BumpAllocatorScope<'a> for BumpScope<'a, C, A> {}
unsafe impl<'a> BumpAllocatorScope<'a> for Unmanaged<'a> {}

/// Exclusive access to a bump allocator.
#[expect(private_bounds, clippy::missing_safety_doc)]
pub unsafe trait MutBumpAllocator: BumpAllocator + BumpAllocatorPrivate {
    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`alloc_iter`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let slice = bump.mut_alloc_iter([1, 2, 3]).into_ref();
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    ///
    /// [`alloc_iter`]: BumpAllocator::alloc_iter
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn mut_alloc_iter<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_mut_alloc_iter(iter));
        BumpAllocation(boxed)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_iter`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let slice = bump.try_mut_alloc_iter([1, 2, 3])?.into_ref();
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_iter`]: BumpAllocator::alloc_iter
    #[inline(always)]
    fn try_mut_alloc_iter<T>(
        &mut self,
        iter: impl IntoIterator<Item = T>,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_mut_alloc_iter(iter)?;
        Ok(BumpAllocation(boxed))
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
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.mut_alloc_fmt(format_args!("{one} + {two} = {}", one + two)).into_ref();
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    ///
    /// [`alloc_fmt`]: BumpAllocator::alloc_fmt
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn mut_alloc_fmt(&mut self, args: fmt::Arguments) -> BumpAllocation<str, &Self> {
        let boxed = panic_on_error(self.imp_mut_alloc_fmt(args));
        BumpAllocation(boxed)
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
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_mut_alloc_fmt(format_args!("{one} + {two} = {}", one + two))?.into_ref();
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_fmt`]: BumpAllocator::try_alloc_fmt
    #[inline(always)]
    fn try_mut_alloc_fmt(&mut self, args: fmt::Arguments) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxed = self.imp_mut_alloc_fmt(args)?;
        Ok(BumpAllocation(boxed))
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
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.mut_alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two)).into_ref();
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.mut_alloc_cstr_fmt(format_args!("{one}\0{two}")).into_ref();
    /// assert_eq!(one, c"1");
    /// ```
    ///
    /// [`alloc_cstr_fmt`]: BumpAllocator::alloc_cstr_fmt
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn mut_alloc_cstr_fmt(&mut self, args: fmt::Arguments) -> IntoRef<CStr, &Self> {
        panic_on_error(self.imp_mut_alloc_cstr_fmt(args))
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
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_mut_alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two))?.into_ref();
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_mut_alloc_cstr_fmt(format_args!("{one}\0{two}"))?.into_ref();
    /// assert_eq!(one, c"1");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_cstr_fmt`]: BumpAllocator::try_alloc_cstr_fmt
    #[inline(always)]
    fn try_mut_alloc_cstr_fmt(&mut self, args: fmt::Arguments) -> Result<IntoRef<CStr, &Self>, AllocError> {
        self.imp_mut_alloc_cstr_fmt(args)
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// This is just like [`alloc_try_with`], but optimized for a mutable reference.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(offset_of_enum)]
    /// use core::mem::offset_of;
    /// use bump_scope::{Bump, MutBumpAllocator, AsMutBumpScope};
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_bump_scope();
    /// let stats = bump.stats();
    ///
    /// let result = bump.mut_alloc_try_with(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(stats.allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// assert_eq!(*result.unwrap().into_box(), 123);
    /// ```
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator, AsMutBumpScope};
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_bump_scope();
    /// let stats = bump.stats();
    ///
    /// let result = bump.mut_alloc_try_with(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(stats.allocated(), 0);
    /// assert_eq!(result.unwrap_err(), 123);
    /// ```
    ///
    /// [`alloc_try_with`]: BumpAllocator::alloc_try_with
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    fn mut_alloc_try_with<T, E>(&mut self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpAllocation<T, &Self>, E> {
        let boxed = panic_on_error(self.imp_mut_alloc_try_with(f))?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// This is just like [`try_alloc_try_with`], but optimized for a mutable reference.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(offset_of_enum)]
    /// use core::mem::offset_of;
    /// use bump_scope::{Bump, MutBumpAllocator, AsMutBumpScope};
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_bump_scope();
    /// let stats = bump.stats();
    ///
    /// let result = bump.try_mut_alloc_try_with(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(stats.allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// assert_eq!(*result.unwrap().into_box(), 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator, AsMutBumpScope};
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_bump_scope();
    /// let stats = bump.stats();
    ///
    /// let result = bump.try_mut_alloc_try_with(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(stats.allocated(), 0);
    /// assert_eq!(result.unwrap_err(), 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_try_with`]: BumpAllocator::try_alloc_try_with
    #[inline(always)]
    fn try_mut_alloc_try_with<T, E>(
        &mut self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpAllocation<T, &Self>, E>, AllocError> {
        Ok(match self.imp_mut_alloc_try_with(f)? {
            Ok(boxed) => Ok(BumpAllocation(boxed)),
            Err(err) => Err(err),
        })
    }
}
unsafe impl<T: MutBumpAllocator> MutBumpAllocator for &mut T {}
unsafe impl<A: Allocator, const C: BumpConfig> MutBumpAllocator for Bump<C, A> {}
unsafe impl<A: Allocator, const C: BumpConfig> MutBumpAllocator for BumpScope<'_, C, A> {}

/// A shorthand for <code>[MutBumpAllocator] + [BumpAllocatorScope]<'a></code>.
#[expect(private_bounds, clippy::missing_safety_doc)]
pub unsafe trait MutBumpAllocatorScope<'a>: MutBumpAllocator + BumpAllocatorScope<'a> + BumpAllocatorPrivate {}
unsafe impl<'a, T: MutBumpAllocator + BumpAllocatorScope<'a>> MutBumpAllocatorScope<'a> for T {}

/// Provides `as_bump_scope` to be generic over `Bump` and `BumpScope`.
///
/// # Examples
/// ```
/// use bump_scope::AsBumpScope;
///
/// struct SomethingBuilder<B> {
///     bump: B,
/// }
///
/// impl<B> SomethingBuilder<B>
/// where
///     B: AsBumpScope
/// {
///     # #[expect(unused)]
///     fn something(&mut self) {
///         self.bump.as_bump_scope().claim().scoped(|bump| {
///             # _ = bump;
///             // ...
///         });
///     }
/// }
/// ```
pub trait AsBumpScope<const C: BumpConfig = DEFAULT_BUMP_CONFIG, A: Allocator = Global> {
    /// Returns the bump allocation scope.
    fn as_bump_scope(&self) -> &BumpScope<'_, C, A>;
}

impl<A: Allocator, const C: BumpConfig> AsBumpScope<C, A> for Bump<C, A> {
    fn as_bump_scope(&self) -> &BumpScope<'_, C, A> {
        self.allocator_ptr.set(&raw const self.allocator);
        unsafe { &*ptr::from_ref(self).cast() }
    }
}

impl<A: Allocator, const C: BumpConfig> AsBumpScope<C, A> for BumpScope<'_, C, A> {
    fn as_bump_scope(&self) -> &BumpScope<'_, C, A> {
        self
    }
}

/// Provides `as_mut_bump_scope` to be generic over `Bump` and `BumpScope`.
///
/// # Examples
/// ```
/// use bump_scope::AsMutBumpScope;
///
/// struct SomethingBuilder<B> {
///     bump: B,
/// }
///
/// impl<B> SomethingBuilder<B>
/// where
///     B: AsMutBumpScope
/// {
///     # #[expect(unused)]
///     fn something(&mut self) {
///         self.bump.as_mut_bump_scope().scoped(|bump| {
///             # _ = bump;
///             // ...
///         });
///     }
/// }
/// ```
pub trait AsMutBumpScope<const C: BumpConfig = DEFAULT_BUMP_CONFIG, A: Allocator = Global> {
    /// Returns the bump allocator scope.
    fn as_mut_bump_scope(&mut self) -> &mut BumpScope<'_, C, A>;
}

impl<A: Allocator, const C: BumpConfig> AsMutBumpScope<C, A> for Bump<C, A> {
    fn as_mut_bump_scope(&mut self) -> &mut BumpScope<'_, C, A> {
        self.allocator_ptr.set(&raw const self.allocator);
        unsafe { &mut *ptr::from_mut(self).cast() }
    }
}

impl<A: Allocator, const C: BumpConfig> AsMutBumpScope<C, A> for BumpScope<'_, C, A> {
    fn as_mut_bump_scope(&mut self) -> &mut BumpScope<'_, C, A> {
        unsafe { transmute_mut(self) }
    }
}

macro_rules! assert_implements {
    ([$($what:tt)*] $($ty:ty)*) => {
        #[cfg(test)]
        const _: () = {
            #[allow(dead_code)]
            type A = crate::util::NoopAllocator;
            #[allow(dead_code)]
            type Bump = crate::Bump<DEFAULT_BUMP_CONFIG, A>;
            #[allow(dead_code)]
            type BumpScope<'a> = crate::BumpScope<'a, DEFAULT_BUMP_CONFIG, A>;
            #[allow(clippy::extra_unused_lifetimes)]
            const fn implements<'a, What: $($what)*>() {}
            $(
                #[allow(clippy::mut_mut)]
                implements::<$ty>();
            )*
        };
    };
}

assert_implements! {
    [BumpAllocator]

    Bump
    &Bump
    &&Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &BumpScope
    &&BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

assert_implements! {
    [BumpAllocatorScope<'a>]

    &Bump
    &&Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &BumpScope
    &&BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

assert_implements! {
    [MutBumpAllocator]

    Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

assert_implements! {
    [MutBumpAllocatorScope<'a>]

    &mut Bump
    &mut &mut Bump

    BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}
